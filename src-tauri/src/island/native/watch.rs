use super::{os_error, NativeEvent};
use crate::island::geometry::{Display, Rect};
use std::{cell::RefCell, mem::size_of, ptr::null_mut, rc::Rc};
use windows_sys::Win32::{
    Foundation::{
        GetLastError, ERROR_HOOK_NOT_INSTALLED, ERROR_INVALID_HOOK_HANDLE, HWND, LPARAM, LRESULT,
        POINT, RECT, WPARAM,
    },
    Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    },
    UI::{
        Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK},
        HiDpi::{GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI},
        Input::KeyboardAndMouse::{GetAsyncKeyState, GetCapture, ReleaseCapture, VK_LBUTTON},
        Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::*,
    },
};

const SUBCLASS: usize = 0x4953;
const RECHECK: u32 = WM_APP + 0x491;
const START_DRAG: u32 = WM_APP + 0x492;

struct Watch {
    hwnd: HWND,
    emit: Rc<dyn Fn(NativeEvent)>,
    foreground: HWINEVENTHOOK,
    location: HWINEVENTHOOK,
    foreground_thread: u32,
    pending: bool,
    fullscreen: bool,
    drag_grip: Option<POINT>,
}

thread_local! { static WATCH: RefCell<Option<Watch>> = const { RefCell::new(None) }; }

fn unhook(hook: HWINEVENTHOOK) -> bool {
    unsafe {
        UnhookWinEvent(hook) != 0
            || matches!(
                GetLastError(),
                ERROR_INVALID_HOOK_HANDLE | ERROR_HOOK_NOT_INSTALLED
            )
    }
}

thread_local! { static RETIRED: RefCell<Vec<HWINEVENTHOOK>> = const { RefCell::new(Vec::new()) }; }

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
    RETIRED.with_borrow_mut(|hooks| hooks.retain(|h| !unhook(*h)));
    if unsafe { SetWindowSubclass(hwnd, Some(window_message), SUBCLASS, 0) } == 0 {
        return Err(os_error("监听窗口事件失败"));
    }
    WATCH.replace(Some(Watch {
        hwnd,
        emit: Rc::new(callback),
        foreground: null_mut(),
        location: null_mut(),
        foreground_thread: 0,
        pending: false,
        fullscreen: false,
        drag_grip: None,
    }));
    Ok(())
}

pub(in crate::island) fn begin_drag(hwnd: HWND) -> Result<(), String> {
    if unsafe { PostMessageW(hwnd, START_DRAG, 0, 0) } == 0 {
        return Err(os_error("请求拖动窗口失败"));
    }
    Ok(())
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

pub(super) fn track(hwnd: HWND, fullscreen: bool) -> Result<(), String> {
    let mut exited = false;
    WATCH.with_borrow_mut(|slot| -> Result<(), String> {
        let w = slot.as_mut().ok_or("窗口监听未安装")?;
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
        WM_ENTERSIZEMOVE => {
            let (mut cursor, mut rect) = (POINT::default(), RECT::default());
            if GetCursorPos(&mut cursor) != 0 && GetWindowRect(hwnd, &mut rect) != 0 {
                WATCH.with_borrow_mut(|s| {
                    if let Some(w) = s {
                        w.drag_grip = Some(POINT {
                            x: cursor.x - rect.left,
                            y: cursor.y - rect.top,
                        });
                    }
                });
            }
        }
        WM_MOVING if lp != 0 => {
            let proposed = &mut *(lp as *mut RECT);
            let mut at = Rect {
                x: proposed.left,
                y: proposed.top,
                width: proposed.right - proposed.left,
                height: proposed.bottom - proposed.top,
            };
            let mut cursor = POINT {
                x: at.x + at.width / 2,
                y: at.y + at.height / 2,
            };
            let grip = WATCH.with_borrow(|slot| slot.as_ref().and_then(|w| w.drag_grip));
            if GetCursorPos(&mut cursor) == 0 {
                GetWindowRect(hwnd, proposed);
                return 1;
            }
            if let Some(grip) = grip {
                // USER32 proposes positions relative to its original move rectangle, even after a change of dock.
                let mut current = RECT::default();
                if GetWindowRect(hwnd, &mut current) != 0 {
                    at.width = current.right - current.left;
                    at.height = current.bottom - current.top;
                }
                at.x = cursor.x - grip.x;
                at.y = cursor.y - grip.y;
            }
            let monitor = MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST);
            let mut info = MONITORINFO {
                cbSize: size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(monitor, &mut info) == 0 {
                GetWindowRect(hwnd, proposed);
                return 1;
            }
            let mut dpi_x = 0;
            let mut dpi_y = 0;
            if GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) < 0 {
                dpi_x = GetDpiForWindow(hwnd).max(96);
            }
            let r = info.rcMonitor;
            let display = Display {
                id: String::new(),
                name: String::new(),
                primary: false,
                scale_factor: f64::from(dpi_x) / 96.0,
                work: Rect {
                    x: r.left,
                    y: r.top,
                    width: r.right - r.left,
                    height: r.bottom - r.top,
                }
                .placement_bounds(info.rcWork.top),
            };
            match crate::island::surface::move_drag_frame(hwnd, &display, at, (cursor.x, cursor.y))
            {
                Ok((r, switched)) => {
                    *proposed = RECT {
                        left: r.x,
                        top: r.y,
                        right: r.x + r.width,
                        bottom: r.y + r.height,
                    };
                    if switched {
                        WATCH.with_borrow_mut(|slot| {
                            if let Some(w) = slot {
                                w.drag_grip = Some(POINT {
                                    x: cursor.x - r.x,
                                    y: cursor.y - r.y,
                                });
                            }
                        });
                    }
                }
                Err(error) => {
                    GetWindowRect(hwnd, proposed);
                    notify(NativeEvent::Error(error));
                }
            }
            return 1;
        }
        WM_EXITSIZEMOVE => WATCH.with_borrow_mut(|slot| {
            if let Some(w) = slot {
                w.drag_grip = None;
            }
        }),
        START_DRAG => {
            let mut cursor = POINT::default();
            // The pointer may have been released before the queued move starts.
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
        WM_POWERBROADCAST | WM_DISPLAYCHANGE | WM_DPICHANGED | WM_SETTINGCHANGE => {
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
    for hook in [w.location, w.foreground] {
        if !hook.is_null() && !unhook(hook) {
            errors.push(os_error("注销系统监听失败"));
            RETIRED.with_borrow_mut(|hooks| hooks.push(hook));
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

#[cfg(test)]
#[path = "edge_drag_test.rs"]
mod edge_drag_test;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::island::surface::Surface;

    #[test]
    #[ignore = "uses an interactive Windows desktop"]
    fn fullscreen_hooks_release_before_the_next_registration() {
        let surface = Surface::start(|_| {}).unwrap();
        for _ in 0..2 {
            surface
                .call(|hwnd| {
                    install(hwnd, |_| {})?;
                    let result = track(hwnd, true).and_then(|_| track(hwnd, false));
                    assert!(WATCH.with_borrow(|w| w
                        .as_ref()
                        .is_some_and(|w| w.foreground.is_null() && w.location.is_null())));
                    result.and(uninstall(hwnd))
                })
                .unwrap();
        }
        surface.shutdown().unwrap();
    }
}
