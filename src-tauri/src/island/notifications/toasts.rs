use super::{identities::Sources, windows::Registry, Permission, Reminder, Source};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::SyncSender,
    },
    time::{Duration, Instant},
};
use windows::{
    Foundation::TypedEventHandler,
    UI::Notifications::{
        Management::{
            UserNotificationListener as Listener, UserNotificationListenerAccessStatus as Access,
        },
        NotificationKinds,
    },
};

impl From<Access> for Permission {
    fn from(status: Access) -> Self {
        match status {
            Access::Allowed => Self::Allowed,
            Access::Denied => Self::Denied,
            Access::Unspecified => Self::NotRequested,
            _ => Self::Error,
        }
    }
}

fn listener() -> Result<Listener, Permission> {
    match crate::island::native::packaged() {
        Ok(true) => Listener::Current().map_err(|_| Permission::Unavailable),
        Ok(false) => Err(Permission::Unavailable),
        Err(_) => Err(Permission::Error),
    }
}

pub fn request_access(
    done: impl FnOnce(Result<Permission, String>) + Send + 'static,
) -> Result<(), String> {
    let listener = match listener() {
        Ok(listener) => listener,
        Err(status) => {
            done(Ok(status));
            return Ok(());
        }
    };
    // Starting the request here keeps the consent dialog on the settings UI thread.
    let request = listener
        .RequestAccessAsync()
        .map_err(|_| "无法请求 Windows 通知权限。")?;
    tauri::async_runtime::spawn(async move {
        done(
            request
                .await
                .map(Permission::from)
                .map_err(|_| "Windows 通知授权未完成。".into()),
        );
    });
    Ok(())
}

pub(super) struct Toasts {
    listener: Listener,
    token: Option<i64>,
    sources: Sources,
}

impl Toasts {
    pub(super) fn open() -> Result<Self, Permission> {
        Ok(Self {
            listener: listener()?,
            token: None,
            sources: Sources::default(),
        })
    }

    pub(super) fn read(
        &mut self,
        registry: &Registry,
        cached: Option<&[Reminder]>,
        stop: &AtomicBool,
        wake: &SyncSender<()>,
    ) -> (Permission, Vec<Reminder>) {
        let permission = self.access();
        if permission != Permission::Allowed || registry.empty() {
            self.unsubscribe();
            self.sources = Sources::default();
            return (permission, vec![]);
        }
        let sources_changed = self.sources.refresh(&registry.identities);
        if !sources_changed && self.token.is_some() {
            if let Some(reminders) = cached {
                return (permission, reminders.to_vec());
            }
        }
        if self.token.is_none() {
            let wake = wake.clone();
            self.token = self
                .listener
                .NotificationChanged(&TypedEventHandler::new(move |_, _| {
                    let _ = wake.try_send(());
                    Ok(())
                }))
                .ok();
        }
        let result = self.collect(registry, stop);
        let current = self.access();
        match (current, result) {
            (Permission::Allowed, Ok(reminders)) => (current, reminders),
            (Permission::Allowed, Err(_)) => (Permission::Error, vec![]),
            _ => (current, vec![]),
        }
    }

    fn access(&self) -> Permission {
        self.listener
            .GetAccessStatus()
            .map(Permission::from)
            .unwrap_or(Permission::Error)
    }

    fn collect(
        &self,
        registry: &Registry,
        stop: &AtomicBool,
    ) -> windows_core::Result<Vec<Reminder>> {
        let operation = self
            .listener
            .GetNotificationsAsync(NotificationKinds::Toast)?;
        let started = Instant::now();
        while operation.Status()? == windows_future::AsyncStatus::Started {
            if stop.load(Ordering::Acquire) || started.elapsed() > Duration::from_secs(2) {
                let _ = operation.Cancel();
                return Err(windows_core::Error::from_hresult(windows_core::HRESULT(
                    0x80004004u32 as i32,
                )));
            }
            std::thread::sleep(Duration::from_millis(15));
        }
        let items = operation.GetResults()?;
        let mut latest: [Option<Reminder>; 2] = [None, None];
        for index in 0..items.Size()? {
            let Ok(item) = items.GetAt(index) else {
                continue;
            };
            let Ok(app_id) = item.AppInfo().and_then(|app| app.AppUserModelId()) else {
                continue;
            };
            let Some(source) = self.sources.lookup(&app_id.to_string_lossy()) else {
                continue;
            };
            let (Ok(id), Ok(created)) = (item.Id(), item.CreationTime()) else {
                continue;
            };
            let created = created.UniversalTime;
            if !registry.accepts_reminder(source, created) {
                continue;
            }
            let slot = &mut latest[usize::from(source == Source::Qq)];
            if slot
                .as_ref()
                .is_none_or(|r| (created, id) > (r.created_ticks, r.latest_id))
            {
                *slot = Some(Reminder {
                    source,
                    created_ticks: created,
                    latest_id: id,
                });
            }
        }
        Ok(latest.into_iter().flatten().collect())
    }

    fn unsubscribe(&mut self) {
        if let Some(token) = self.token.take() {
            let _ = self.listener.RemoveNotificationChanged(token);
        }
    }
}

impl Drop for Toasts {
    fn drop(&mut self) {
        self.unsubscribe();
    }
}
