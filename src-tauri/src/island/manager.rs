mod main_window;
pub(super) use main_window::MainSnapshot;
mod messages;
mod settings;
pub(super) use messages::NotificationStatus;

use super::{
    config::{Anchor, IslandConfig, PlacementConfig},
    config_store::ConfigStore,
    displays,
    media::{MediaSnapshot, Worker as MediaWorker},
    native::{self, NativeEvent},
    render::RenderStats,
    state::{Event, IslandState, Machine, TimerAction},
    surface::{Surface, SurfaceEvent},
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
                let host = app.clone();
                let store = ConfigStore::new(directory, move || displays::displays(&host));
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
                    generation: 0,
                    sequence: 0,
                    messages: messages::Messages::default(),
                    media: MediaSnapshot::default(),
                    media_worker: None,
                    timer: None,
                    drag_origin: None,
                    exiting: false,
                    main_window: Default::default(),
                };
                if let Some(error) = manager.load_error.clone() {
                    manager.report(error);
                } else {
                    let _ = manager.restart();
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
    renderer: &'static str,
    native: RenderStats,
}

pub(super) struct IslandManager {
    pub app: AppHandle,
    config: IslandConfig,
    machine: Machine,
    store: ConfigStore,
    worker: ManagerHandle,
    window: Option<Surface>,
    session_position: Option<PlacementConfig>,
    load_error: Option<String>,
    last_error: Option<String>,
    generation: u64,
    sequence: u64,
    timer: Option<(Instant, Event)>,
    messages: messages::Messages,
    media: MediaSnapshot,
    media_worker: Option<MediaWorker>,
    drag_origin: Option<(i32, i32)>,
    exiting: bool,
    main_window: main_window::MainWindow,
}

impl IslandManager {
    // One owner serializes input, system events, revisions and the current deadline.
    fn run(&mut self, receive: Receiver<Job>) {
        while !self.exiting {
            if let Some((deadline, event)) = self.timer {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    self.timer = None;
                    if let Err(error) = self.transition(event) {
                        self.report(error);
                    }
                    continue;
                }
                match receive.recv_timeout(remaining) {
                    Ok(job) => job(self),
                    Err(RecvTimeoutError::Timeout) => continue,
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
            renderer: "native",
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
        self.rebuild_window()
            .inspect_err(|error| self.report(error.clone()))
    }

    fn rebuild_window(&mut self) -> Result<()> {
        let config = self.edge_config(self.store.load()?)?;
        self.close_window()?;
        self.load_error = None;
        self.last_error = None;
        self.config = config;
        self.machine = Machine::new(&self.config);
        self.session_position = None;
        let startup =
            native::sync_autostart(self.config.startup.launch_at_login).and_then(|enabled| {
                if enabled != self.config.startup.launch_at_login {
                    let mut draft = self.config.clone();
                    draft.startup.launch_at_login = enabled;
                    self.config = self.store.save(&draft, self.config.revision)?;
                }
                Ok(())
            });
        if let Err(error) = startup {
            self.report(error);
        }
        let initialized = self
            .app
            .emit("island://config-changed", &self.config)
            .map_err(|e| e.to_string())
            .and_then(|()| self.refresh())
            .and_then(|()| self.observe_initial_pointer());
        if let Err(error) = initialized {
            self.close_window()
                .map_err(|cleanup| format!("{error}；关闭原生岛屿失败：{cleanup}"))?;
            return Err(error);
        }
        Ok(())
    }

    fn start_surface(&mut self) -> Result<()> {
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
        let input = notify.clone();
        let window = Surface::start(move |event| input(NativeEvent::Input(event)))?;
        window.install_watch(notify)?;
        let handle = self.worker.clone();
        self.media_worker = Some(
            MediaWorker::start(move |snapshot| {
                let _ = handle.post(move |manager| {
                    if manager.generation == generation {
                        manager.receive_media(snapshot)?;
                    }
                    Ok(())
                });
            })
            .map_err(|error| error.to_string())?,
        );
        self.window = Some(window);
        Ok(())
    }

    pub fn media(&self) -> &MediaSnapshot {
        &self.media
    }

    fn receive_media(&mut self, next: MediaSnapshot) -> Result<()> {
        let resized = self.media.battery.is_some() != next.battery.is_some()
            || self.media.title.is_empty() != next.title.is_empty();
        let previous_position = self.media.position_ms;
        self.media.position_ms = next.position_ms;
        let notify_preview = self.media != next
            || !next.playing
            || !(0..=1800).contains(&next.position_ms.saturating_sub(previous_position));
        self.media = next;
        if let Some(window) = &self.window {
            window.set_media(self.media.clone())?;
        }
        if resized {
            self.refresh()?;
        }
        if notify_preview {
            self.app
                .emit_to("main", "island://media-changed", ())
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn observe_initial_pointer(&mut self) -> Result<()> {
        if matches!(
            self.machine.state,
            IslandState::Compact | IslandState::Expanded
        ) {
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
        if state == IslandState::Disabled {
            if self.window.is_some() {
                self.close_window()?;
            }
            return Ok(());
        }
        if self.window.is_none() {
            self.start_surface()?;
        }
        self.configure_messages()?;
        if let Some(window) = &self.window {
            if self.drag_origin.is_none() {
                if state.is_visible() {
                    let cfg = self.effective_config();
                    let screens = displays::displays(&self.app)?;
                    let display =
                        displays::target_display(&screens, cfg.placement.display_id.as_deref())?;
                    let layout = display.content_config(&cfg, &self.media, self.reminder_count());
                    window.apply(&layout, state, display)?;
                } else {
                    window.hide()?;
                }
            }
            window.track(
                self.config.edge.hide_on_fullscreen
                    && !matches!(state, IslandState::Disabled | IslandState::Hidden),
            )?;
        }
        Ok(())
    }

    pub fn transition(&mut self, event: Event) -> Result<()> {
        if self.drag_origin.is_some()
            && matches!(
                event,
                Event::PointerEnter | Event::PointerLeave | Event::Click
            )
        {
            return Ok(());
        }
        let previous = self.machine.state;
        let effects = self.machine.apply(event, &self.config);
        if !self.machine.state.is_visible() {
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
                SurfaceEvent::ToggleLyrics => {
                    let mut config = self.read_config()?;
                    config.appearance.lyrics_enabled = !config.appearance.lyrics_enabled;
                    self.save(config)?;
                    return Ok(());
                }
                SurfaceEvent::Hide => Event::Hide,
                SurfaceEvent::OpenInstances(source) => {
                    if let Some(window) = &self.window {
                        window.focus_source(source)?;
                    }
                    Event::OpenNotifications
                }
                SurfaceEvent::ActivateWindow(id) => return self.activate_notification_window(id),
                SurfaceEvent::Transport(action) => {
                    return self
                        .media_worker
                        .as_ref()
                        .ok_or("媒体监听已停止")?
                        .control(&self.media, action);
                }
                SurfaceEvent::DragStart => return self.drag(),
                SurfaceEvent::Error(error) => return Err(error),
            },
            NativeEvent::Fullscreen(true) => Event::FullscreenEntered,
            NativeEvent::Fullscreen(false) => Event::FullscreenExited,
            NativeEvent::DisplayChanged => {
                self.refresh_message_source();
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

    fn close_window(&mut self) -> Result<()> {
        self.generation += 1;
        self.media_worker.take();
        self.media = MediaSnapshot::default();
        let _ = self.app.emit_to("main", "island://media-changed", ());
        let messages = self.stop_messages();
        self.drag_origin = None;
        self.timer = None;
        if let Some(window) = self.window.take() {
            let hooks = window.uninstall_watch();
            hooks.and(window.shutdown())?;
        }
        messages
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
