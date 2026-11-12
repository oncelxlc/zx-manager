mod accessibility;
mod hit_clock;
mod input;
mod messages;
mod placement;
mod poc;
pub use poc::run_poc;

use super::{
    config::IslandConfig,
    geometry::{Display, Rect},
    notifications::{NotificationSnapshot, Source},
    render::{RenderStats, Renderer},
    state::IslandState,
};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    ptr::{null, null_mut},
    sync::{mpsc, Arc, Mutex},
    thread::{self, JoinHandle},
};
use windows_sys::{
    core::w,
    Win32::{
        Foundation::{GetLastError, ERROR_CLASS_ALREADY_EXISTS, HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Gdi::ValidateRect,
        System::{
            Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED},
            LibraryLoader::GetModuleHandleW,
        },
        UI::{
            HiDpi::{SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2},
            WindowsAndMessaging::*,
        },
    },
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum SurfaceEvent {
    PointerEnter,
    PointerLeave,
    Click,
    OpenNotificationSource(Source),
    ActivateWindow(u64),
    NotificationSettings,
    Pin(bool),
    Hide,
    DragStart,
    Error(String),
}

impl super::render::MessageAction {
    fn surface_event(self) -> Option<SurfaceEvent> {
        Some(match self {
            Self::Source(s) => SurfaceEvent::OpenNotificationSource(s),
            Self::Window(id) => SurfaceEvent::ActivateWindow(id),
            Self::Settings => SurfaceEvent::NotificationSettings,
            Self::Pin(value) => SurfaceEvent::Pin(value),
            Self::Hide => SurfaceEvent::Hide,
            Self::NextPage => return None,
        })
    }
}

type Result<T> = std::result::Result<T, String>;
type Job = Box<dyn FnOnce(HWND, &Context) + Send>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum DuringMove {
    Dispatch,
    Defer,
}
const WORK: u32 = WM_APP + 0x521;
const RECOVER: u32 = WM_APP + 0x522;
pub(super) const SESSION_GUARD: u32 = WM_APP + 0x524;
const NEXT_PAGE: u32 = WM_APP + 0x525;

struct Context {
    messages: RefCell<super::render::Messages>,
    preview_allowed: Cell<bool>,
    pulse: Cell<Option<(Source, std::time::Instant)>>,
    queue: Arc<Mutex<Queue>>,
    a11y: accessibility::Accessibility,
    renderer: RefCell<Option<Renderer>>,
    frame: RefCell<Option<placement::Frame>>,
    clock: hit_clock::HitClock,
    mask: Cell<Option<(Rect, i32)>>,
    recovery_pending: Cell<bool>,
    redraw_pending: Cell<bool>,
    dragging: Cell<bool>,
    in_move_loop: Cell<bool>,
    acrylic: Cell<bool>,
    input: input::Pointer,
    emit: Arc<dyn Fn(SurfaceEvent) + Send + Sync>,
}

#[derive(Default)]
struct Queue {
    jobs: VecDeque<(DuringMove, Job)>,
    closing: bool,
}

impl Queue {
    fn close(&mut self) {
        self.closing = true;
        self.jobs.clear();
    }
}

struct Thread {
    hwnd: usize,
    queue: Arc<Mutex<Queue>>,
    worker: Mutex<Option<JoinHandle<Result<()>>>>,
}

#[derive(Clone)]
pub(super) struct Surface(Arc<Thread>);

fn error(operation: &str) -> String {
    format!("{operation}: {}", std::io::Error::last_os_error())
}

impl Surface {
    pub fn start(emit: impl Fn(SurfaceEvent) + Send + Sync + 'static) -> Result<Self> {
        let queue = Arc::new(Mutex::new(Queue::default()));
        let work = queue.clone();
        let (started, ready) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("island-composition".into())
            .spawn(move || {
                let result = run(work.clone(), started, Arc::new(emit));
                work.lock().unwrap().close();
                result
            })
            .map_err(|e| e.to_string())?;
        let hwnd = match ready.recv() {
            Ok(hwnd) => hwnd,
            Err(_) => {
                return Err(worker
                    .join()
                    .ok()
                    .and_then(|r| r.err())
                    .unwrap_or_else(|| "原生窗口初始化中断".into()));
            }
        };
        Ok(Self(Arc::new(Thread {
            hwnd,
            queue,
            worker: Mutex::new(Some(worker)),
        })))
    }

    fn request<T: Send + 'static>(
        &self,
        during_move: DuringMove,
        f: impl FnOnce(HWND, &Context) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let (reply, result) = mpsc::sync_channel(1);
        {
            let mut queue = self.0.queue.lock().unwrap();
            if queue.closing {
                return Err("原生窗口线程已关闭".into());
            }
            // Post under the queue lock so a failed wake cannot leave an orphaned operation.
            if unsafe { PostMessageW(self.0.hwnd as HWND, WORK, 0, 0) } == 0 {
                return Err(error("投递原生窗口请求失败"));
            }
            queue.jobs.push_back((
                during_move,
                Box::new(move |hwnd, ui| {
                    let _ = reply.send(f(hwnd, ui));
                }),
            ));
        }
        result
            .recv()
            .map_err(|_| "原生窗口线程已关闭".to_string())?
    }

    pub fn call<T: Send + 'static>(
        &self,
        f: impl FnOnce(HWND) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        self.request(DuringMove::Dispatch, move |hwnd, _| f(hwnd))
    }

    pub fn apply(&self, cfg: &IslandConfig, state: IslandState, screen: &Display) -> Result<()> {
        if !state.is_visible() {
            return self.hide();
        }
        let target = screen.target(cfg, state);
        let scale = screen.scale_factor;
        let cfg = cfg.clone();
        self.request(DuringMove::Defer, move |hwnd, ui| {
            placement::apply(ui, hwnd, cfg, state, target, scale)
        })
    }

    pub fn set_notifications(
        &self,
        snapshot: NotificationSnapshot,
        selected_source: Option<Source>,
        selected_window: Option<u64>,
    ) -> Result<()> {
        self.request(DuringMove::Dispatch, move |hwnd, ui| {
            messages::update(ui, hwnd, snapshot, selected_source, selected_window)
        })
    }

    pub fn hide(&self) -> Result<()> {
        self.cancel_drag()?;
        self.request(DuringMove::Defer, |hwnd, ui| placement::hide(ui, hwnd))
    }

    pub fn outer_rect(&self) -> Result<Rect> {
        self.request(DuringMove::Dispatch, |hwnd, ui| {
            placement::outer_rect(ui, hwnd)
        })
    }

    pub fn prepare_drag(&self) -> Result<Rect> {
        self.request(DuringMove::Defer, |hwnd, ui| {
            placement::prepare_drag(ui, hwnd)
        })
    }

    pub fn cancel_drag(&self) -> Result<()> {
        let queue = self.0.queue.lock().unwrap();
        if !queue.closing && unsafe { PostMessageW(self.0.hwnd as HWND, WM_CANCELMODE, 0, 0) } == 0
        {
            return Err(error("取消原生窗口拖动失败"));
        }
        Ok(())
    }

    pub fn stats(&self) -> Result<RenderStats> {
        self.request(DuringMove::Dispatch, |_, ui| {
            Ok(ui
                .renderer
                .borrow()
                .as_ref()
                .map(Renderer::stats)
                .unwrap_or_default())
        })
    }

    pub fn shutdown(&self) -> Result<()> {
        self.0.stop()
    }
}

impl Thread {
    fn stop(&self) -> Result<()> {
        let mut worker = self.worker.lock().unwrap();
        if worker.is_none() {
            return Ok(());
        }
        {
            let mut queue = self.queue.lock().unwrap();
            if !queue.closing {
                // The system move loop can own the pump until the button is released.
                unsafe {
                    PostMessageW(self.hwnd as HWND, WM_CANCELMODE, 0, 0);
                }
                if unsafe { PostMessageW(self.hwnd as HWND, WORK, 0, 0) } == 0 {
                    return Err(error("请求退出原生窗口失败"));
                }
                queue.closing = true;
            }
        }
        worker
            .take()
            .unwrap()
            .join()
            .map_err(|_| "原生窗口线程异常退出".to_string())?
    }
}

impl Drop for Thread {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn run(
    work: Arc<Mutex<Queue>>,
    started: mpsc::SyncSender<usize>,
    emit: Arc<dyn Fn(SurfaceEvent) + Send + Sync>,
) -> Result<()> {
    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe { CoUninitialize() }
        }
    }
    let hr = unsafe { CoInitializeEx(null(), COINIT_APARTMENTTHREADED as u32) };
    if hr < 0 {
        return Err(format!("初始化原生窗口 STA 失败: 0x{hr:08x}"));
    }
    let _apartment = Apartment;
    unsafe {
        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    let instance = unsafe { GetModuleHandleW(null()) };
    let class = WNDCLASSW {
        lpfnWndProc: Some(wndproc),
        hInstance: instance,
        hCursor: unsafe { LoadCursorW(null_mut(), IDC_ARROW) },
        lpszClassName: w!("ZXOnce.NativeIsland.v1"),
        ..Default::default()
    };
    if unsafe { RegisterClassW(&class) } == 0
        && unsafe { GetLastError() } != ERROR_CLASS_ALREADY_EXISTS
    {
        return Err(error("注册灵动岛窗口类失败"));
    }
    let ui = Context {
        queue: work.clone(),
        a11y: accessibility::Accessibility::new(emit.clone()),
        messages: Default::default(),
        preview_allowed: Cell::new(false),
        pulse: Cell::new(None),
        renderer: RefCell::new(None),
        frame: RefCell::new(None),
        clock: hit_clock::HitClock::new()?,
        mask: Cell::new(None),
        recovery_pending: Cell::new(false),
        redraw_pending: Cell::new(false),
        dragging: Cell::new(false),
        in_move_loop: Cell::new(false),
        acrylic: Cell::new(false),
        input: Default::default(),
        emit,
    };
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_NOREDIRECTIONBITMAP,
            class.lpszClassName,
            w!("zx-once 灵动岛（原生）"),
            WS_POPUP,
            0,
            0,
            1,
            1,
            null_mut(),
            null_mut(),
            instance,
            (&raw const ui).cast(),
        )
    };
    if hwnd.is_null() {
        return Err(error("创建原生灵动岛失败"));
    }
    let result = (|| {
        started
            .send(hwnd as usize)
            .map_err(|_| "原生窗口启动已取消".to_string())?;
        let mut msg = MSG::default();
        loop {
            let job = {
                let mut queue = work.lock().unwrap();
                if queue.closing {
                    break;
                }
                queue.jobs.pop_front()
            };
            if let Some((_, job)) = job {
                job(hwnd, &ui);
                continue;
            }
            if ui.redraw_pending.replace(false) && !ui.dragging.get() {
                if let (Some(f), Some(r)) = (
                    ui.frame.borrow().as_ref(),
                    ui.renderer.borrow_mut().as_mut(),
                ) {
                    if let Err(e) = r.apply(&f.cfg, f.state, f.target, f.host, f.scale, false) {
                        (ui.emit)(SurfaceEvent::Error(e));
                    }
                }
            }
            if ui.clock.wait()? {
                if let Err(e) = placement::update_mask(&ui, hwnd) {
                    ui.clock.stop();
                    (ui.emit)(SurfaceEvent::Error(e));
                }
                continue;
            }
            if unsafe { PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) } == 0 {
                continue;
            }
            match msg.message {
                WM_QUIT => break,
                WORK if msg.hwnd == hwnd => (),
                RECOVER if msg.hwnd == hwnd => {
                    let frame = ui
                        .frame
                        .borrow()
                        .as_ref()
                        .map(|f| (f.cfg.clone(), f.state, f.target, f.scale));
                    if let Some((cfg, state, target, scale)) = frame {
                        if let Err(e) = placement::apply(&ui, hwnd, cfg, state, target, scale) {
                            (ui.emit)(SurfaceEvent::Error(e));
                        }
                    }
                }
                _ => unsafe {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                },
            }
        }
        Ok(())
    })();
    work.lock().unwrap().close();
    ui.renderer.take();
    ui.a11y.clear();
    if unsafe { IsWindow(hwnd) } != 0 {
        unsafe {
            DestroyWindow(hwnd);
        }
    }
    result
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let create = &*(lp as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
    }
    // Context lives on this thread until after DestroyWindow returns.
    let ui = (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Context).as_ref();
    if let Some(ui) = ui {
        if let Some(result) = input::message(ui, hwnd, msg, lp) {
            return result;
        }
        match msg {
            SESSION_GUARD => {
                ui.preview_allowed.set(wp != 0);
                if wp == 0 {
                    if let Err(e) =
                        messages::clear_previews(ui).and_then(|()| messages::redraw(ui, hwnd))
                    {
                        (ui.emit)(SurfaceEvent::Error(e));
                    }
                }
                return 0;
            }
            NEXT_PAGE => {
                let page = ui.messages.borrow().page;
                ui.messages.borrow_mut().page = page.wrapping_add(1);
                if let Err(e) = messages::redraw(ui, hwnd) {
                    (ui.emit)(SurfaceEvent::Error(e));
                }
                return 0;
            }
            WM_TIMER if wp == messages::PULSE_TIMER => {
                if let Err(e) = messages::redraw(ui, hwnd) {
                    (ui.emit)(SurfaceEvent::Error(e));
                }
                return 0;
            }
            WM_ENTERSIZEMOVE => ui.in_move_loop.set(true),
            WM_EXITSIZEMOVE => ui.in_move_loop.set(false),
            WORK if ui.in_move_loop.get() => {
                // The OS dispatches window messages inside SC_MOVE while our outer pump is suspended.
                let job = {
                    let mut queue = ui.queue.lock().unwrap();
                    match queue.jobs.front() {
                        Some((DuringMove::Dispatch, _)) if !queue.closing => queue.jobs.pop_front(),
                        Some(_) => {
                            PostMessageW(hwnd, WM_CANCELMODE, 0, 0);
                            None
                        }
                        None => None,
                    }
                };
                if let Some((_, job)) = job {
                    job(hwnd, ui);
                }
                return 0;
            }
            WM_GETOBJECT => {
                if let Some(result) = ui.a11y.object(hwnd, wp, lp) {
                    return result;
                }
            }
            WM_CLOSE => {
                (ui.emit)(SurfaceEvent::Hide);
                return 0;
            }
            WM_MOUSEACTIVATE => return MA_NOACTIVATE as LRESULT,
            WM_ERASEBKGND => return 1,
            WM_PAINT => {
                ValidateRect(hwnd, null());
                if ui
                    .renderer
                    .try_borrow()
                    .ok()
                    .is_some_and(|r| r.as_ref().is_some_and(|r| !r.valid()))
                    && !ui.recovery_pending.replace(true)
                    && PostMessageW(hwnd, RECOVER, 0, 0) == 0
                {
                    ui.recovery_pending.set(false);
                    (ui.emit)(SurfaceEvent::Error(error("请求图形设备恢复失败")));
                }
                return 0;
            }
            WM_DESTROY => {
                ui.queue.lock().unwrap().close();
                PostQuitMessage(0);
                return 0;
            }
            WM_NCDESTROY => {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            _ => (),
        }
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hidden_window_destroys_before_join_and_restarts() {
        for _ in 0..2 {
            let surface = Surface::start(|_| ()).unwrap();
            let hwnd = surface
                .call(|hwnd| {
                    assert_eq!(unsafe { IsWindowVisible(hwnd) }, 0);
                    Ok(hwnd as usize)
                })
                .unwrap();
            assert_eq!(surface.stats().unwrap().presents, 0);
            surface.shutdown().unwrap();
            assert_eq!(unsafe { IsWindow(hwnd as HWND) }, 0);
            assert!(surface.call(|_| Ok(())).is_err());
            surface.shutdown().unwrap();
        }
    }

    #[test]
    fn destroyed_handle_rejects_work_before_it_can_be_reused() {
        let surface = Surface::start(|_| ()).unwrap();
        surface
            .call(|hwnd| {
                if unsafe { DestroyWindow(hwnd) } == 0 {
                    return Err(error("测试窗口销毁失败"));
                }
                Ok(())
            })
            .unwrap();
        assert!(surface.call(|_| Ok(())).is_err());
        surface.shutdown().unwrap();
    }

    #[test]
    fn move_loop_serves_reads_and_defers_renderer_work_until_cancelled() {
        const FINISH_TEST: u32 = WM_APP + 0x523;
        let surface = Surface::start(|_| ()).unwrap();
        let (started, entered) = mpsc::sync_channel(1);
        let owner = surface.clone();
        let modal = thread::spawn(move || {
            owner.call(move |hwnd| unsafe {
                SendMessageW(hwnd, WM_ENTERSIZEMOVE, 0, 0);
                started.send(hwnd as usize).unwrap();
                let mut msg = MSG::default();
                let cancelled = loop {
                    if GetMessageW(&mut msg, null_mut(), 0, 0) <= 0 || msg.message == FINISH_TEST {
                        break false;
                    }
                    if msg.message == WM_CANCELMODE {
                        break true;
                    }
                    DispatchMessageW(&msg);
                };
                SendMessageW(hwnd, WM_EXITSIZEMOVE, 0, 0);
                Ok(cancelled)
            })
        });
        let hwnd = entered.recv().unwrap() as HWND;
        let (done, result) = mpsc::sync_channel(1);
        let reader = thread::spawn(move || {
            let observed = surface
                .request(DuringMove::Dispatch, |_, ui| Ok(ui.in_move_loop.get()))
                .unwrap();
            let drawn = surface.stats().unwrap().presents;
            let deferred = surface
                .request(DuringMove::Defer, |_, ui| Ok(ui.in_move_loop.get()))
                .unwrap();
            done.send((observed, drawn, deferred)).unwrap();
        });
        let outcome = result.recv_timeout(std::time::Duration::from_secs(5));
        if outcome.is_err() {
            unsafe {
                PostMessageW(hwnd, FINISH_TEST, 0, 0);
            }
        }
        let cancelled = modal.join().unwrap().unwrap();
        reader.join().unwrap();
        assert_eq!(outcome.unwrap(), (true, 0, false));
        assert!(cancelled);
    }
}
