mod settings;

use super::{
    backend::{Backend, RendererKind},
    config::{Anchor, IslandConfig, PlacementConfig},
    config_store::ConfigStore,
    native::{self, NativeEvent},
    render::RenderStats,
    state::{Event, IslandState, Machine, TimerAction},
    surface::SurfaceEvent,
};
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender},
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};

type Job = Box<dyn FnOnce(&mut IslandManager) + Send>;
type Result<T> = std::result::Result<T, String>;

#[derive(Clone)]
pub(super) struct ManagerHandle(Sender<Job>);

impl ManagerHandle {
    pub fn start(app: AppHandle, directory: PathBuf) -> std::io::Result<Self> {
        let (send, receive) = mpsc::channel();
        let handle = Self(send);
        let worker = handle.clone();
        thread::Builder::new()
            .name("island-manager".into())
            .spawn(move || {
                let store = ConfigStore::new(directory);
                let (config, load_error) = match store.load() {
                    Ok(config) => (config, None),
                    Err(error) => (IslandConfig::default(), Some(error)),
                };
                let mut manager = IslandManager {
                    machine: Machine::new(&config),
                    config,
                    store,
                    worker,
                    app,
                    load_error,
                    last_error: None,
                    session_position: None,
                    window: None,
                    watching: false,
                    generation: 0,
                    sequence: 0,
                    timer: None,
                    ready: false,
                    drag_origin: None,
                    exiting: false,
                };
                if let Some(error) = manager.load_error.clone() {
                    manager.report(error);
                } else if let Err(error) = manager.restart() {
                    manager.report(error);
                }
                manager.run(receive);
            })?;
        Ok(handle)
    }

    pub fn post(
        &self,
        job: impl FnOnce(&mut IslandManager) -> Result<()> + Send + 'static,
    ) -> Result<()> {
        self.0
            .send(Box::new(move |manager| {
                if let Err(error) = job(manager) {
                    manager.report(error);
                }
            }))
            .map_err(|_| "灵动岛管理器已停止".to_owned())
    }

    pub async fn request<T: Send + 'static>(
        &self,
        job: impl FnOnce(&mut IslandManager) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let (reply, mut answer) = tauri::async_runtime::channel(1);
        self.post(move |manager| {
            let _ = reply.try_send(job(manager));
            Ok(())
        })?;
        answer.recv().await.ok_or("灵动岛请求已取消")?
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RuntimeStatus {
    state: IslandState,
    sequence: u64,
    last_error: Option<String>,
}

#[derive(Serialize)]
pub(super) struct PerformanceStats {
    renderer: RendererKind,
    native: Option<RenderStats>,
}

pub(super) struct IslandManager {
    pub app: AppHandle,
    config: IslandConfig,
    machine: Machine,
    store: ConfigStore,
    worker: ManagerHandle,
    window: Option<Backend>,
    watching: bool,
    session_position: Option<PlacementConfig>,
    load_error: Option<String>,
    last_error: Option<String>,
    generation: u64,
    sequence: u64,
    timer: Option<(Instant, Event)>,
    ready: bool,
    drag_origin: Option<(i32, i32)>,
    exiting: bool,
}

impl IslandManager {
    // One owner serializes input, system events, revisions and the current deadline.
    fn run(&mut self, receive: Receiver<Job>) {
        while !self.exiting {
            if let Some((deadline, event)) = self.timer {
                match receive.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                    Ok(job) => job(self),
                    Err(RecvTimeoutError::Timeout) => {
                        self.timer = None;
                        if let Err(error) = self.transition(event) {
                            self.report(error);
                        }
                    }
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            } else if let Ok(job) = receive.recv() {
                job(self);
            } else {
                break;
            }
        }
    }

    fn report(&mut self, error: String) {
        eprintln!("灵动岛：{error}");
        self.last_error = Some(error.clone());
        let _ = self.app.emit("island://runtime-error", error);
    }

    pub fn config(&self) -> Result<IslandConfig> {
        match &self.load_error {
            Some(error) => Err(error.clone()),
            None => Ok(self.config.clone()),
        }
    }

    pub fn status(&self) -> RuntimeStatus {
        RuntimeStatus {
            state: self.machine.state,
            sequence: self.sequence,
            last_error: self.last_error.clone(),
        }
    }

    pub fn performance(&self) -> Result<PerformanceStats> {
        let window = self.window.as_ref().ok_or("灵动岛窗口未启动")?;
        Ok(PerformanceStats {
            renderer: window.kind(),
            native: window.stats()?,
        })
    }

    fn effective_config(&self) -> IslandConfig {
        let mut config = self.config.clone();
        if let Some(position) = &self.session_position {
            config.placement.anchor = position.anchor;
            config.placement.display_id.clone_from(&position.display_id);
            config.placement.offset_x = position.offset_x;
            config.placement.offset_y = position.offset_y;
        }
        config
    }

    pub fn restart(&mut self) -> Result<()> {
        let config = self.store.load()?;
        let kind = RendererKind::from_env()?;
        self.ready = false;
        self.drag_origin = None;
        self.timer = None;
        self.generation += 1;
        self.stop_watching()?;
        let worker = self.worker.clone();
        let generation = self.generation;
        let notify = move |event| {
            let _ = worker.post(move |manager| {
                if manager.generation == generation {
                    manager.system_event(event)?;
                }
                Ok(())
            });
        };
        // Tauri releases a destroyed window label asynchronously. Reload the existing WebView.
        if let (RendererKind::Webview, Some(Backend::Webview(window))) = (kind, &self.window) {
            window.hide()?;
            window
                .webview()
                .reload()
                .map_err(|error| error.to_string())?;
        } else {
            if let Some(window) = self.window.take() {
                window.shutdown()?;
            }
            self.window = Some(Backend::new(kind, &self.app, notify.clone())?);
        }
        self.load_error = None;
        self.last_error = None;
        self.config = config;
        self.machine = Machine::new(&self.config);
        self.session_position = None;
        let window = self.window.as_ref().ok_or("灵动岛窗口不可用")?;
        window.install_watch(notify)?;
        self.watching = true;
        if let Err(error) = window.configure_hotkeys(self.config.active_hotkeys()) {
            self.report(error);
        }
        if let Err(error) = native::set_autostart(self.config.startup.launch_at_login) {
            self.report(error);
        }
        self.app
            .emit("island://config-changed", &self.config)
            .map_err(|e| e.to_string())?;
        if kind == RendererKind::Native {
            self.mark_ready()
        } else {
            self.refresh()
        }
    }

    pub fn mark_ready(&mut self) -> Result<()> {
        if self.ready {
            return Ok(());
        }
        if !self.watching {
            return Err("系统窗口初始化失败，请从设置中重启灵动岛".into());
        }
        self.ready = true;
        self.refresh()?;
        self.observe_initial_pointer()
    }

    fn observe_initial_pointer(&mut self) -> Result<()> {
        if self.ready
            && matches!(
                self.machine.state,
                IslandState::Compact | IslandState::Expanded
            )
        {
            if let Some(native) = &self.window {
                if !native.pointer_inside()? {
                    self.transition(Event::PointerLeave)?;
                }
            }
        }
        Ok(())
    }

    fn refresh(&mut self) -> Result<()> {
        self.sequence += 1;
        self.app
            .emit("island://state-changed", self.status())
            .map_err(|e| e.to_string())?;
        let state = self.machine.state;
        if let Some(window) = &self.window {
            if self.ready && self.drag_origin.is_none() {
                window.apply(&self.app, &self.effective_config(), state)?;
            }
            let fullscreen = self.ready
                && self.config.edge.hide_on_fullscreen
                && !matches!(state, IslandState::Disabled | IslandState::Hidden);
            if self.watching {
                window.track(
                    fullscreen,
                    self.ready
                        && state == IslandState::Expanded
                        && self.config.interaction.outside_collapse,
                )?;
            }
        }
        Ok(())
    }

    pub fn transition(&mut self, event: Event) -> Result<()> {
        if self.drag_origin.is_some()
            && matches!(
                event,
                Event::PointerEnter | Event::PointerLeave | Event::Click | Event::OutsideClick
            )
        {
            return Ok(());
        }
        let previous = self.machine.state;
        let effects = self.machine.apply(event, &self.config);
        if matches!(
            self.machine.state,
            IslandState::Hidden | IslandState::Disabled | IslandState::Suspended
        ) {
            self.drag_origin = None;
        }
        match effects.timer {
            TimerAction::Keep => {}
            TimerAction::Cancel => self.timer = None,
            TimerAction::After { ms, event } => {
                self.timer = Some((Instant::now() + Duration::from_millis(ms.into()), event));
            }
        }
        if self.machine.state != previous
            || matches!(event, Event::ConfigUpdated | Event::DisplayChanged)
        {
            self.refresh()?;
            if effects.observe_pointer {
                self.observe_initial_pointer()?;
            }
        }
        Ok(())
    }

    fn system_event(&mut self, event: NativeEvent) -> Result<()> {
        let event = match event {
            NativeEvent::Input(input) => match input {
                SurfaceEvent::PointerEnter => Event::PointerEnter,
                SurfaceEvent::PointerLeave => Event::PointerLeave,
                SurfaceEvent::Click => Event::Click,
                SurfaceEvent::Pin(true) => Event::Pin,
                SurfaceEvent::Pin(false) => Event::Unpin,
                SurfaceEvent::Hide => Event::Hide,
                SurfaceEvent::DragStart => return self.drag(),
                SurfaceEvent::Error(error) => return Err(error),
            },
            NativeEvent::Show => Event::Show,
            NativeEvent::Hide => Event::Hide,
            NativeEvent::Toggle => Event::Toggle,
            NativeEvent::Fullscreen(true) => Event::FullscreenEntered,
            NativeEvent::Fullscreen(false) => Event::FullscreenExited,
            NativeEvent::OutsideClick => Event::OutsideClick,
            NativeEvent::DisplayChanged => {
                self.app
                    .emit("island://display-changed", ())
                    .map_err(|e| e.to_string())?;
                Event::DisplayChanged
            }
            NativeEvent::DragEnded => return self.finish_drag(),
            NativeEvent::Error(error) => return Err(error),
        };
        self.transition(event)
    }

    pub fn close_main(&mut self) -> Result<()> {
        if self.config.startup.close_to_tray {
            if let Some(window) = self.app.get_webview_window("main") {
                window.hide().map_err(|e| e.to_string())?;
            }
            Ok(())
        } else {
            self.exit()
        }
    }

    pub fn open_main(&self, settings: bool) -> Result<()> {
        let window = self.app.get_webview_window("main").ok_or("主窗口不可用")?;
        let hash = if settings {
            "location.hash='/settings/island'"
        } else {
            "location.hash='/'"
        };
        window
            .eval(hash)
            .and_then(|_| window.unminimize())
            .and_then(|_| window.show())
            .and_then(|_| window.set_focus())
            .map_err(|e| e.to_string())
    }

    fn stop_watching(&mut self) -> Result<()> {
        if std::mem::take(&mut self.watching) {
            if let Some(window) = &self.window {
                window.uninstall_watch()?;
            }
        }
        Ok(())
    }

    fn close_window(&mut self) -> Result<()> {
        self.generation += 1;
        self.ready = false;
        self.drag_origin = None;
        let hooks = self.stop_watching();
        let window = self
            .window
            .take()
            .map(|window| window.shutdown())
            .transpose();
        hooks.and(window).map(|_| ())
    }

    pub fn exit(&mut self) -> Result<()> {
        self.exiting = true;
        self.timer = None;
        if let Err(error) = self.close_window() {
            self.report(error);
        }
        self.app.remove_tray_by_id("island-tray");
        self.app.exit(0);
        Ok(())
    }
}
