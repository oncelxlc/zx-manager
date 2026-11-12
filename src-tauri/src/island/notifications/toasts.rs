use super::{identities::Sources, windows::Registry, Permission, Reminder, Settings, Source};
use crate::island::config::NotificationConfig;
use std::{
    mem::size_of,
    ptr::null_mut,
    sync::{mpsc::SyncSender, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use windows::{
    Foundation::TypedEventHandler,
    UI::Notifications::{
        KnownNotificationBindings,
        Management::{
            UserNotificationListener as Listener, UserNotificationListenerAccessStatus as Access,
        },
        NotificationKinds, UserNotification,
    },
};
use windows_sys::Win32::System::RemoteDesktop::*;

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

pub(super) fn now_ticks() -> i64 {
    const EPOCH_TICKS: i64 = 116444736000000000;
    EPOCH_TICKS
        + (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            / 100) as i64
}

fn session_unlocked() -> bool {
    unsafe {
        let mut data = null_mut();
        let mut len = 0;
        if WTSQuerySessionInformationW(
            WTS_CURRENT_SERVER_HANDLE,
            WTS_CURRENT_SESSION,
            WTSSessionInfoEx,
            &mut data,
            &mut len,
        ) == 0
        {
            return false;
        }
        let unlocked = !data.is_null() && len as usize >= size_of::<WTSINFOEXW>() && {
            let info = &*data.cast::<WTSINFOEXW>();
            info.Level == 1
                && info.Data.WTSInfoExLevel1.SessionState == WTSActive
                && info.Data.WTSInfoExLevel1.SessionFlags == WTS_SESSIONSTATE_UNLOCK as i32
        };
        WTSFreeMemory(data.cast());
        unlocked
    }
}

fn sanitize(parts: &[String]) -> Option<String> {
    let message = parts.join(" ");
    let lower = message.to_lowercase();
    // Hide the whole excerpt when digits span fields or separators, even when this over-masks a date.
    if message.chars().filter(|ch| ch.is_numeric()).take(4).count() == 4
        || [
            "验证码",
            "校验码",
            "驗證碼",
            "校驗碼",
            "动态码",
            "verification",
            "passcode",
            "security code",
            "one-time",
            "otp",
        ]
        .iter()
        .any(|term| lower.contains(term))
    {
        return Some("敏感内容已隐藏".into());
    }
    let line: String = message
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .filter(|ch| {
            !ch.is_control() && !matches!(*ch, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
        .take(80)
        .collect();
    (!line.is_empty()).then_some(line)
}

fn preview(item: &UserNotification) -> Option<String> {
    let binding = item
        .Notification()
        .ok()?
        .Visual()
        .ok()?
        .GetBinding(&KnownNotificationBindings::ToastGeneric().ok()?)
        .ok()?;
    let texts = binding.GetTextElements().ok()?;
    let size = texts.Size().ok()?;
    if size > 8 {
        return Some("通知内容已省略".into());
    }
    let mut parts = Vec::new();
    for i in 0..size {
        let text = texts.GetAt(i).ok()?.Text().ok()?;
        if text.len() > 2048 {
            return Some("通知内容已省略".into());
        }
        parts.push(text.to_string_lossy());
    }
    sanitize(&parts)
}

pub(super) struct Toasts {
    listener: Listener,
    token: Option<i64>,
    sources: Sources,
}

pub(super) struct ReadPolicy<'a> {
    pub settings: &'a Mutex<Settings>,
    pub epoch: u64,
    pub cutoff: i64,
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
        config: &NotificationConfig,
        policy: ReadPolicy<'_>,
        wake: &SyncSender<()>,
    ) -> (Permission, Vec<Reminder>, Option<Instant>) {
        let permission = self
            .listener
            .GetAccessStatus()
            .map(Permission::from)
            .unwrap_or(Permission::Error);
        if permission != Permission::Allowed {
            self.unsubscribe();
            self.sources = Sources::default();
            return (permission, vec![], None);
        }
        self.sources.refresh(&registry.identities);
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
        let result = self.collect(config, &policy);
        let current = self
            .listener
            .GetAccessStatus()
            .map(Permission::from)
            .unwrap_or(Permission::Error);
        match (current, result) {
            (Permission::Allowed, Ok((reminders, expiry))) => (current, reminders, expiry),
            (Permission::Allowed, Err(_)) => (Permission::Error, vec![], None),
            _ => (current, vec![], None),
        }
    }

    fn collect(
        &self,
        config: &NotificationConfig,
        policy: &ReadPolicy<'_>,
    ) -> windows_core::Result<(Vec<Reminder>, Option<Instant>)> {
        let operation = self
            .listener
            .GetNotificationsAsync(NotificationKinds::Toast)?;
        let started = Instant::now();
        // An unresponsive broker must not hold worker shutdown indefinitely.
        while operation.Status()? == windows_future::AsyncStatus::Started {
            let cancelled = {
                let state = policy.settings.lock().unwrap();
                state.stop || state.epoch != policy.epoch
            };
            if cancelled || started.elapsed() > Duration::from_secs(2) {
                let _ = operation.Cancel();
                return Err(windows_core::Error::from_hresult(windows_core::HRESULT(
                    0x80004004u32 as i32,
                )));
            }
            std::thread::sleep(Duration::from_millis(15));
        }
        let items = operation.GetResults()?;
        let mut latest = [None, None];
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
            if !source.enabled(config) {
                continue;
            }
            let (Ok(id), Ok(created)) = (item.Id(), item.CreationTime()) else {
                continue;
            };
            let slot = &mut latest[usize::from(source == Source::Qq)];
            let key = (created.UniversalTime, id);
            if slot.as_ref().is_none_or(|(old, _, _)| key > *old) {
                *slot = Some((key, source, item));
            }
        }
        let now = now_ticks();
        let mut expiry: Option<Instant> = None;
        let mut result = vec![];
        for ((created, latest_id), source, item) in latest.into_iter().flatten() {
            let remaining = created
                .saturating_add(i64::from(config.preview_timeout_seconds) * 10_000_000)
                .saturating_sub(now);
            let mut excerpt = None;
            if config.show_preview
                && created >= policy.cutoff
                && created <= now
                && remaining > 0
                && session_unlocked()
                && self.listener.GetAccessStatus()? == Access::Allowed
            {
                // Serialize the content read with a lock/hide/disable transition.
                let state = policy.settings.lock().unwrap();
                if !state.stop && state.epoch == policy.epoch && state.allow_preview {
                    excerpt = preview(&item);
                }
                drop(state);
                if excerpt.is_some() {
                    let deadline = Instant::now() + Duration::from_nanos(remaining as u64 * 100);
                    expiry = Some(expiry.map_or(deadline, |old| old.min(deadline)));
                }
            }
            result.push(Reminder {
                source,
                preview: excerpt,
                latest_id,
                created_ticks: created,
            });
        }
        Ok((result, expiry))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protects_codes_across_text_fields_and_truncation() {
        for parts in [
            vec!["验证码".into(), "ABCD".into()],
            vec!["12".into(), "３ - ４".into()],
            vec![format!("{} 123456", "x".repeat(90))],
        ] {
            assert_eq!(sanitize(&parts).as_deref(), Some("敏感内容已隐藏"));
        }
        assert_eq!(
            sanitize(&["会议  开始".into()]).as_deref(),
            Some("会议 开始")
        );
    }
}
