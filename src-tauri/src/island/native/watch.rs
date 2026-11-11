use super::{os_error, NativeEvent};
use std::{
    cell::RefCell,
    mem::size_of,
    ptr::{null, null_mut},
    rc::Rc,
};
use windows_sys::Win32::{
    Foundation::{
        GetLastError, SetLastError, ERROR_HOOK_NOT_INSTALLED, ERROR_INVALID_HOOK_HANDLE, HWND,
        LPARAM, LRESULT, POINT, RECT, WPARAM,
    },
    Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST},
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK},
        Input::KeyboardAndMouse::{
            GetAsyncKeyState, GetCapture, RegisterHotKey, ReleaseCapture, UnregisterHotKey,
            MOD_NOREPEAT, VK_LBUTTON,
        },
        Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::*,
    },
};

type Binding = (u32, u32);
const SUBCLASS: usize = 0x4953;
const RECHECK: u32 = WM_APP + 0x491;
const START_DRAG: u32 = WM_APP + 0x492;
const BUTTON_DOWN: u32 = WM_APP + 0x493;

struct Watch {
    hwnd: HWND,
    emit: Rc<dyn Fn(NativeEvent)>,
    hotkeys: Vec<(i32, Binding)>,
    actions: [Option<Binding>; 3],
    mouse: HHOOK,
    foreground: HWINEVENTHOOK,
    location: HWINEVENTHOOK,
    foreground_thread: u32,
    pending: bool,
    fullscreen: bool,
}

thread_local! { static WATCH: RefCell<Option<Watch>> = const { RefCell::new(None) }; }

enum OldHook {
    Mouse(HHOOK),
    Event(HWINEVENTHOOK),
}

impl OldHook {
    fn remove(&self) -> bool {
        unsafe {
            let result = match self {
                OldHook::Mouse(h) => UnhookWindowsHookEx(*h),
                OldHook::Event(h) => UnhookWinEvent(*h),
            };
            result != 0
                || matches!(
                    GetLastError(),
                    ERROR_INVALID_HOOK_HANDLE | ERROR_HOOK_NOT_INSTALLED
                )
        }
    }
}

thread_local! { static RETIRED: RefCell<Vec<OldHook>> = const { RefCell::new(Vec::new()) }; }

pub(super) fn notify(event: NativeEvent) {
    let emit = WATCH.with_borrow(|w| w.as_ref().map(|w| w.emit.clone()));
    if let Some(emit) = emit {
        emit(event);
    }
}

pub(super) fn install(hwnd: HWND, callback: impl Fn(NativeEvent) + 'static) -> Result<(), String> {
    if WATCH.with_borrow(|w| w.is_some()) {
        return Err("窗口监听已注册".into());
    }
    RETIRED.with_borrow_mut(|hooks| hooks.retain(|h| !h.remove()));
    // Tao maps focusable(false) to NOACTIVATE. TOOLWINDOW also removes it from Alt+Tab.
    unsafe {
        SetLastError(0);
        let style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        if style == 0 && GetLastError() != 0 {
            return Err(os_error("读取工具窗口样式失败"));
        }
        SetLastError(0);
        if SetWindowLongW(hwnd, GWL_EXSTYLE, (style | WS_EX_TOOLWINDOW) as i32) == 0
            && GetLastError() != 0
        {
            return Err(os_error("设置工具窗口样式失败"));
        }
        if SetWindowPos(
            hwnd,
            null_mut(),
            0,
            0,
            0,
            0,
            SWP_NOSIZE | SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        ) == 0
        {
            return Err(os_error("更新工具窗口样式失败"));
        }
    }
    if unsafe { SetWindowSubclass(hwnd, Some(window_message), SUBCLASS, 0) } == 0 {
        return Err(os_error("监听窗口事件失败"));
    }
    WATCH.replace(Some(Watch {
        hwnd,
        emit: Rc::new(callback),
        hotkeys: Vec::new(),
        actions: [None; 3],
        mouse: null_mut(),
        foreground: null_mut(),
        location: null_mut(),
        foreground_thread: 0,
        pending: false,
        fullscreen: false,
    }));
    Ok(())
}

pub(in crate::island) fn begin_drag(hwnd: HWND) -> Result<(), String> {
    super::motion::cancel(hwnd);
    if unsafe { PostMessageW(hwnd, START_DRAG, 0, 0) } == 0 {
        return Err(os_error("请求拖动窗口失败"));
    }
    Ok(())
}

pub(super) fn hotkeys(hwnd: HWND, actions: [Option<Binding>; 3]) -> Result<(), String> {
    let previous = WATCH
        .with_borrow(|slot| slot.as_ref().map(|w| w.actions))
        .ok_or("窗口监听未安装")?;
    match update_hotkeys(hwnd, actions) {
        Ok(()) => Ok(()),
        Err(error) => match update_hotkeys(hwnd, previous) {
            Ok(()) => Err(error),
            Err(rollback) => Err(format!("{error}；恢复旧快捷键失败：{rollback}")),
        },
    }
}

fn update_hotkeys(hwnd: HWND, actions: [Option<Binding>; 3]) -> Result<(), String> {
    WATCH.with_borrow_mut(|slot| {
        let w = slot.as_mut().ok_or("窗口监听未安装")?;
        // Keep existing registrations (even when swapping actions). Acquire new bindings
        // before releasing old ones, so a conflict leaves the previous shortcuts usable.
        let mut added: Vec<(i32, Binding)> = Vec::new();
        for binding in actions.iter().flatten() {
            if w.hotkeys.iter().chain(&added).any(|(_, b)| b == binding) {
                continue;
            }
            let id = (0x4900..0x4910)
                .find(|id| !w.hotkeys.iter().chain(&added).any(|(k, _)| k == id))
                .ok_or("快捷键 ID 已耗尽")?;
            if unsafe { RegisterHotKey(hwnd, id, binding.0 | MOD_NOREPEAT, binding.1) } == 0 {
                let mut error = os_error("注册快捷键失败（可能已被其他应用占用）");
                for (id, b) in added {
                    if unsafe { UnregisterHotKey(hwnd, id) } == 0 {
                        error.push_str(&os_error("；回滚快捷键失败"));
                        w.hotkeys.push((id, b));
                    }
                }
                return Err(error);
            }
            added.push((id, *binding));
        }
        w.hotkeys.extend(added);
        let mut failure = None;
        w.hotkeys.retain(|(id, binding)| {
            if actions.contains(&Some(*binding)) {
                return true;
            }
            if unsafe { UnregisterHotKey(hwnd, *id) } == 0 {
                failure = Some(os_error("注销快捷键失败"));
                return true;
            }
            false
        });
        w.actions = actions;
        failure.map_or(Ok(()), Err)
    })
}

fn remove_event(hook: &mut HWINEVENTHOOK) -> Result<(), String> {
    if !hook.is_null() {
        if unsafe { UnhookWinEvent(*hook) } == 0 {
            return Err(os_error("注销系统事件失败"));
        }
        *hook = null_mut();
    }
    Ok(())
}

pub(super) fn track(hwnd: HWND, fullscreen: bool, outside: bool) -> Result<(), String> {
    let mut exited = false;
    WATCH.with_borrow_mut(|slot| -> Result<(), String> {
        let w = slot.as_mut().ok_or("窗口监听未安装")?;
        if outside && w.mouse.is_null() {
            let module = unsafe { GetModuleHandleW(null()) };
            w.mouse = unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_button), module, 0) };
            if w.mouse.is_null() {
                return Err(os_error("启用外部点击收起失败"));
            }
        } else if !outside && !w.mouse.is_null() {
            if unsafe { UnhookWindowsHookEx(w.mouse) } == 0 {
                return Err(os_error("注销外部点击监听失败"));
            }
            w.mouse = null_mut();
        }
        if fullscreen && w.foreground.is_null() {
            w.foreground = unsafe {
                SetWinEventHook(
                    EVENT_SYSTEM_FOREGROUND,
                    EVENT_SYSTEM_FOREGROUND,
                    null_mut(),
                    Some(win_event),
                    0,
                    0,
                    WINEVENT_OUTOFCONTEXT,
                )
            };
            if w.foreground.is_null() {
                return Err(os_error("监听前台窗口失败"));
            }
        } else if !fullscreen {
            remove_event(&mut w.foreground)?;
            remove_event(&mut w.location)?;
            w.foreground_thread = 0;
            exited = std::mem::take(&mut w.fullscreen);
        }
        Ok(())
    })?;
    if exited {
        notify(NativeEvent::Fullscreen(false));
    }
    if fullscreen {
        refresh_fullscreen(hwnd)?;
    }
    Ok(())
}

unsafe extern "system" fn mouse_button(code: i32, action: WPARAM, data: LPARAM) -> LRESULT {
    if code >= 0
        && matches!(
            action as u32,
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN
        )
    {
        let hwnd = WATCH.with(|w| {
            w.try_borrow()
                .ok()
                .and_then(|w| w.as_ref().filter(|w| !w.mouse.is_null()).map(|w| w.hwnd))
        });
        if let Some(hwnd) = hwnd {
            let pt = (*(data as *const MSLLHOOKSTRUCT)).pt;
            PostMessageW(hwnd, BUTTON_DOWN, pt.x as WPARAM, pt.y as LPARAM);
        }
    }
    CallNextHookEx(null_mut(), code, action, data)
}

unsafe extern "system" fn win_event(
    hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    object: i32,
    child: i32,
    _: u32,
    _: u32,
) {
    if event == EVENT_OBJECT_LOCATIONCHANGE
        && (hwnd != GetForegroundWindow() || object != OBJID_WINDOW || child != 0)
    {
        return;
    }
    WATCH.with(|slot| {
        let Ok(mut guard) = slot.try_borrow_mut() else {
            return;
        };
        if let Some(w) = guard.as_mut() {
            if !w.pending && (hook == w.foreground || hook == w.location) {
                w.pending = PostMessageW(w.hwnd, RECHECK, 0, 0) != 0;
            }
        }
    });
}

fn hits(hwnd: HWND, point: POINT) -> bool {
    let target = unsafe { WindowFromPoint(point) };
    target == hwnd || unsafe { IsChild(hwnd, target) != 0 }
}

pub(super) fn pointer_inside(hwnd: HWND) -> Result<bool, String> {
    let mut cursor = POINT::default();
    if unsafe { GetCursorPos(&mut cursor) } == 0 {
        return Err(os_error("读取指针位置失败"));
    }
    Ok(hits(hwnd, cursor))
}

fn is_fullscreen(foreground: HWND, island: HWND) -> bool {
    unsafe {
        if foreground.is_null()
            || foreground == island
            || IsIconic(foreground) != 0
            || IsWindowVisible(foreground) == 0
        {
            return false;
        }
        let mut pid = 0;
        GetWindowThreadProcessId(foreground, &mut pid);
        if pid == std::process::id() {
            return false;
        }
        let mut class = [0u16; 128];
        let len = GetClassNameW(foreground, class.as_mut_ptr(), class.len() as i32);
        if matches!(
            String::from_utf16_lossy(&class[..len.max(0) as usize]).as_str(),
            "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd"
        ) {
            return false;
        }
        if IsZoomed(foreground) != 0
            && GetWindowLongW(foreground, GWL_STYLE) as u32 & WS_CAPTION != 0
        {
            return false;
        }
        let monitor = MonitorFromWindow(foreground, MONITOR_DEFAULTTONEAREST);
        if monitor != MonitorFromWindow(island, MONITOR_DEFAULTTONEAREST) {
            return false;
        }
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let mut rect = RECT::default();
        if GetMonitorInfoW(monitor, &mut info) == 0 || GetWindowRect(foreground, &mut rect) == 0 {
            return false;
        }
        let m = info.rcMonitor;
        rect.left <= m.left && rect.top <= m.top && rect.right >= m.right && rect.bottom >= m.bottom
    }
}

fn refresh_fullscreen(hwnd: HWND) -> Result<(), String> {
    let foreground = unsafe { GetForegroundWindow() };
    let thread = if foreground.is_null() {
        0
    } else {
        unsafe { GetWindowThreadProcessId(foreground, null_mut()) }
    };
    let full = is_fullscreen(foreground, hwnd);
    let changed = WATCH.with_borrow_mut(|slot| -> Result<bool, String> {
        let w = slot.as_mut().ok_or("窗口监听未安装")?;
        w.pending = false;
        if w.foreground.is_null() {
            return Ok(false);
        }
        if thread != w.foreground_thread {
            remove_event(&mut w.location)?;
            w.foreground_thread = 0;
            if thread != 0 {
                w.location = unsafe {
                    SetWinEventHook(
                        EVENT_OBJECT_LOCATIONCHANGE,
                        EVENT_OBJECT_LOCATIONCHANGE,
                        null_mut(),
                        Some(win_event),
                        0,
                        thread,
                        WINEVENT_OUTOFCONTEXT,
                    )
                };
                if w.location.is_null() {
                    return Err(os_error("监听前台窗口尺寸失败"));
                }
                w.foreground_thread = thread;
            }
        }
        Ok(std::mem::replace(&mut w.fullscreen, full) != full)
    })?;
    if changed {
        notify(NativeEvent::Fullscreen(full));
    }
    Ok(())
}

unsafe extern "system" fn window_message(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    _: usize,
) -> LRESULT {
    if !WATCH.with(|slot| {
        slot.try_borrow()
            .ok()
            .is_some_and(|w| w.as_ref().is_some_and(|w| w.hwnd == hwnd))
    }) {
        return DefSubclassProc(hwnd, msg, wp, lp);
    }
    match msg {
        WM_MOUSEACTIVATE => return MA_NOACTIVATE as LRESULT,
        BUTTON_DOWN => {
            if WATCH.with_borrow(|w| w.as_ref().is_some_and(|w| !w.mouse.is_null()))
                && !hits(
                    hwnd,
                    POINT {
                        x: wp as i32,
                        y: lp as i32,
                    },
                )
            {
                notify(NativeEvent::OutsideClick);
            }
            return 0;
        }
        WM_HOTKEY => {
            let action = WATCH.with_borrow(|slot| {
                let w = slot.as_ref()?;
                let (_, binding) = w.hotkeys.iter().find(|(id, _)| *id == wp as i32)?;
                w.actions.iter().position(|a| *a == Some(*binding))
            });
            if let Some(i) = action {
                notify([NativeEvent::Toggle, NativeEvent::Show, NativeEvent::Hide][i].clone());
                return 0;
            }
        }
        START_DRAG => {
            let mut cursor = POINT::default();
            // The pointer may have been released while its IPC command was in flight.
            if GetAsyncKeyState(i32::from(VK_LBUTTON)) < 0 && GetCursorPos(&mut cursor) != 0 {
                let capture = GetCapture();
                if capture == hwnd || IsChild(hwnd, capture) != 0 {
                    ReleaseCapture();
                }
                let point = (cursor.x as u16 as u32) | ((cursor.y as u16 as u32) << 16);
                SendMessageW(
                    hwnd,
                    WM_SYSCOMMAND,
                    (SC_MOVE | HTCAPTION) as usize,
                    point as LPARAM,
                );
            }
            // The OS move loop has returned, including cancellation by Escape.
            notify(NativeEvent::DragEnded);
            return 0;
        }
        WM_DISPLAYCHANGE | WM_DPICHANGED | WM_SETTINGCHANGE | WM_POWERBROADCAST => {
            notify(NativeEvent::DisplayChanged)
        }
        RECHECK => {
            if let Err(e) = refresh_fullscreen(hwnd) {
                notify(NativeEvent::Error(e));
            }
            return 0;
        }
        _ => (),
    }
    DefSubclassProc(hwnd, msg, wp, lp)
}

pub(super) fn uninstall(hwnd: HWND) -> Result<(), String> {
    let Some(w) = WATCH.take() else { return Ok(()) };
    let mut errors = Vec::new();
    for hook in [
        (!w.mouse.is_null()).then_some(OldHook::Mouse(w.mouse)),
        (!w.location.is_null()).then_some(OldHook::Event(w.location)),
        (!w.foreground.is_null()).then_some(OldHook::Event(w.foreground)),
    ]
    .into_iter()
    .flatten()
    {
        if !hook.remove() {
            errors.push(os_error("注销系统监听失败"));
            RETIRED.with_borrow_mut(|hooks| hooks.push(hook));
        }
    }
    for (id, _) in w.hotkeys {
        if unsafe { UnregisterHotKey(hwnd, id) } == 0 {
            errors.push(os_error("注销快捷键失败"));
        }
    }
    if unsafe { RemoveWindowSubclass(hwnd, Some(window_message), SUBCLASS) } == 0 {
        errors.push(os_error("移除窗口监听失败"));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("；"))
    }
}
