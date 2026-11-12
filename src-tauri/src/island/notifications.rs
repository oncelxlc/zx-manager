mod toasts;
mod windows;

use super::config::NotificationConfig;
use ::windows::Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED};
use serde::Serialize;
use std::{
    collections::HashMap,
    sync::{
        mpsc::{sync_channel, Receiver, SyncSender},
        Arc, Mutex,
    },
    thread::{Builder, JoinHandle},
    time::{Duration, Instant},
};
pub use toasts::request_access;
use windows::{Registry, Target};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    Wechat,
    Qq,
}

impl Source {
    pub fn label(self) -> &'static str {
        match self {
            Self::Wechat => "微信",
            Self::Qq => "QQ",
        }
    }

    pub(super) fn enabled(self, config: &NotificationConfig) -> bool {
        config.enabled
            && match self {
                Self::Wechat => config.wechat_enabled,
                Self::Qq => config.qq_enabled,
            }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Permission {
    #[default]
    Disabled,
    Unavailable,
    NotRequested,
    Denied,
    Allowed,
    Error,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowEntry {
    pub id: u64,
    pub source: Source,
    pub label: String,
}

#[derive(Clone, Eq, PartialEq)]
pub struct Reminder {
    pub source: Source,
    pub preview: Option<String>,
    pub latest_id: u32,
    pub created_ticks: i64,
}

impl std::fmt::Debug for Reminder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Reminder")
            .field("source", &self.source)
            .field("latest_id", &self.latest_id)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NotificationSnapshot {
    pub epoch: u64,
    pub permission: Permission,
    pub windows: Vec<WindowEntry>,
    pub reminders: Vec<Reminder>,
}

struct Settings {
    config: NotificationConfig,
    allow_preview: bool,
    preview_since: i64,
    epoch: u64,
    scan: bool,
    stop: bool,
    targets: HashMap<u64, Target>,
}

pub struct NotificationWorker {
    settings: Arc<Mutex<Settings>>,
    wake: SyncSender<()>,
    thread: Option<JoinHandle<()>>,
}

impl NotificationWorker {
    pub fn start(
        config: NotificationConfig,
        allow_preview: bool,
        emit: impl Fn(NotificationSnapshot) + Send + 'static,
    ) -> Result<Self, String> {
        let (wake, queue) = sync_channel(1);
        let settings = Arc::new(Mutex::new(Settings {
            config,
            allow_preview,
            preview_since: toasts::now_ticks(),
            epoch: 0,
            scan: true,
            stop: false,
            targets: HashMap::new(),
        }));
        let shared = Arc::clone(&settings);
        let events = wake.clone();
        let thread = Builder::new()
            .name("island-notifications".into())
            .spawn(move || {
                let initialized = unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.is_ok();
                run(shared, events, queue, emit, initialized);
                if initialized {
                    unsafe { RoUninitialize() };
                }
            })
            .map_err(|_| "无法启动通知监听线程。")?;
        Ok(Self {
            settings,
            wake,
            thread: Some(thread),
        })
    }

    pub fn configure(&self, config: NotificationConfig, allow_preview: bool) -> u64 {
        let mut state = self.settings.lock().unwrap();
        if state.config != config || state.allow_preview != allow_preview {
            state.epoch += 1;
            state.preview_since = toasts::now_ticks();
            state
                .targets
                .retain(|_, target| target.source_enabled(&config));
            state.config = config;
            state.allow_preview = allow_preview;
            state.scan = true;
            let _ = self.wake.try_send(());
        }
        state.epoch
    }

    pub fn refresh(&self) {
        self.settings.lock().unwrap().scan = true;
        let _ = self.wake.try_send(());
    }

    pub fn activate(&self, id: u64) -> Result<(), String> {
        let target = self
            .settings
            .lock()
            .unwrap()
            .targets
            .get(&id)
            .copied()
            .ok_or("窗口已关闭或来源已停用。")?;
        let result = target.activate();
        self.refresh();
        result
    }
}

impl Drop for NotificationWorker {
    fn drop(&mut self) {
        self.settings.lock().unwrap().stop = true;
        let _ = self.wake.try_send(());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run(
    settings: Arc<Mutex<Settings>>,
    wake: SyncSender<()>,
    queue: Receiver<()>,
    emit: impl Fn(NotificationSnapshot),
    initialized: bool,
) {
    // Some desktop clients recreate their windows silently. Reconcile at low frequency even
    // when no toast arrives; the notification callback otherwise wakes this thread immediately.
    const RECONCILE: Duration = Duration::from_secs(5);
    let mut registry = Registry::default();
    let mut listener = None;
    let mut previous: Option<NotificationSnapshot> = None;
    let mut next_scan = Instant::now();
    let mut preview_expiry = None;
    loop {
        let (config, epoch, cutoff, force_scan) = {
            let mut s = settings.lock().unwrap();
            if s.stop {
                break;
            }
            let cutoff = if s.allow_preview {
                s.preview_since
            } else {
                i64::MAX
            };
            (
                s.config.clone(),
                s.epoch,
                cutoff,
                std::mem::take(&mut s.scan),
            )
        };
        let mut snapshot = NotificationSnapshot {
            epoch,
            ..Default::default()
        };
        if preview_expiry.is_some_and(|at| at <= Instant::now()) {
            if let Some(old) = previous.as_mut().filter(|old| old.epoch == epoch) {
                for item in &mut old.reminders {
                    item.preview = None;
                }
                emit(old.clone());
            }
        }
        preview_expiry = None;
        if config.enabled && (config.wechat_enabled || config.qq_enabled) {
            if force_scan || Instant::now() >= next_scan {
                registry.scan(&config);
                next_scan = Instant::now() + RECONCILE;
            }
            snapshot.windows = registry.windows();
            if config.windows_notifications_enabled {
                let source = if let Some(source) = &mut listener {
                    Ok(source)
                } else if !initialized {
                    Err(Permission::Error)
                } else {
                    toasts::Toasts::open().map(|source| listener.insert(source))
                };
                match source {
                    Err(permission) => snapshot.permission = permission,
                    Ok(source) => {
                        (snapshot.permission, snapshot.reminders, preview_expiry) = source.read(
                            &registry,
                            &config,
                            toasts::ReadPolicy {
                                settings: &settings,
                                epoch,
                                cutoff,
                            },
                            &wake,
                        );
                    }
                }
            } else {
                listener = None;
            }
        } else {
            listener = None;
            registry = Registry::default();
        }
        {
            let mut s = settings.lock().unwrap();
            if s.stop {
                break;
            }
            if s.epoch != epoch {
                previous = None;
                continue;
            }
            s.targets = registry.targets();
        }
        if previous.as_ref() != Some(&snapshot) {
            previous = Some(snapshot.clone());
            emit(snapshot);
        }
        if config.enabled && (config.wechat_enabled || config.qq_enabled) {
            let deadline = preview_expiry.map_or(next_scan, |at: Instant| at.min(next_scan));
            let _ = queue.recv_timeout(deadline.saturating_duration_since(Instant::now()));
        } else if queue.recv().is_err() {
            break;
        }
    }
}
