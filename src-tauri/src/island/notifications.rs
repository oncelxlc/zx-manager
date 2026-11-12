mod icon;
mod identities;
mod toasts;
mod windows;

use ::windows::Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED};
use serde::Serialize;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
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
    fn from_filename(path: &std::path::Path) -> Option<Self> {
        match path.file_name()?.to_str()?.to_ascii_lowercase().as_str() {
            "wechat.exe" | "weixin.exe" => Some(Self::Wechat),
            "qq.exe" => Some(Self::Qq),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Wechat => "微信",
            Self::Qq => "QQ",
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

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct AppIcon {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowEntry {
    pub id: u64,
    pub source: Source,
    pub label: String,
    pub can_activate: bool,
    pub icon: Option<Arc<AppIcon>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reminder {
    pub source: Source,
    pub latest_id: u32,
    pub created_ticks: i64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NotificationSnapshot {
    pub permission: Permission,
    pub windows: Vec<WindowEntry>,
    pub reminders: Vec<Reminder>,
}

#[derive(Default)]
struct WorkerState {
    stop: AtomicBool,
    scan: AtomicBool,
    targets: Mutex<HashMap<u64, Target>>,
}

pub struct NotificationWorker {
    shared: Arc<WorkerState>,
    wake: SyncSender<()>,
    thread: Option<JoinHandle<()>>,
}

impl NotificationWorker {
    pub fn start(emit: impl Fn(NotificationSnapshot) + Send + 'static) -> Result<Self, String> {
        let (wake, queue) = sync_channel(1);
        let shared = Arc::new(WorkerState::default());
        let state = Arc::clone(&shared);
        let events = wake.clone();
        let thread = Builder::new()
            .name("island-notifications".into())
            .spawn(move || {
                let initialized = unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.is_ok();
                run(state, events, queue, emit, initialized);
                if initialized {
                    unsafe { RoUninitialize() };
                }
            })
            .map_err(|_| "无法启动通知监听线程。")?;
        Ok(Self {
            shared,
            wake,
            thread: Some(thread),
        })
    }

    pub fn refresh(&self) {
        self.shared.scan.store(true, Ordering::Release);
        let _ = self.wake.try_send(());
    }

    pub fn activate(&self, id: u64) -> Result<(), String> {
        let target = self.shared.targets.lock().unwrap().get(&id).copied();
        let result = target
            .ok_or("窗口已关闭，请重新选择。".to_string())
            .and_then(Target::activate);
        self.refresh();
        result
    }
}

impl Drop for NotificationWorker {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Release);
        let _ = self.wake.try_send(());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run(
    state: Arc<WorkerState>,
    wake: SyncSender<()>,
    queue: Receiver<()>,
    emit: impl Fn(NotificationSnapshot),
    initialized: bool,
) {
    const RECONCILE: Duration = Duration::from_secs(2);
    let mut registry = Registry::default();
    let mut listener = None;
    let mut previous: Option<NotificationSnapshot> = None;
    let mut next_scan = Instant::now();
    let mut next_read = next_scan;
    let mut notified = true;
    while !state.stop.load(Ordering::Acquire) {
        if state.scan.swap(false, Ordering::AcqRel) || Instant::now() >= next_scan {
            registry.scan();
            *state.targets.lock().unwrap() = registry.targets();
            next_scan = Instant::now() + RECONCILE;
        }
        let source = if let Some(source) = &mut listener {
            Ok(source)
        } else if initialized {
            toasts::Toasts::open().map(|source| listener.insert(source))
        } else {
            Err(Permission::Error)
        };
        let windows = registry.windows();
        let cached = previous.as_ref().filter(|old| {
            !notified
                && Instant::now() < next_read
                && old.windows == windows
                && old.permission == Permission::Allowed
        });
        if cached.is_none() {
            next_read = Instant::now() + Duration::from_secs(30);
        }
        let (permission, reminders) = match source {
            Ok(source) => source.read(
                &registry,
                cached.map(|s| s.reminders.as_slice()),
                &state.stop,
                &wake,
            ),
            Err(permission) => (permission, vec![]),
        };
        let snapshot = NotificationSnapshot {
            windows,
            reminders,
            permission,
        };
        if previous.as_ref() != Some(&snapshot) {
            previous = Some(snapshot.clone());
            emit(snapshot);
        }
        notified = queue
            .recv_timeout(next_scan.saturating_duration_since(Instant::now()))
            .is_ok();
    }
}
