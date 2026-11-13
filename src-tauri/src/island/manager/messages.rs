use super::*;
use crate::island::notifications::{NotificationSnapshot, NotificationWorker, Permission};
use std::sync::{Arc, Mutex};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NotificationStatus {
    sequence: u64,
    permission: Permission,
}

#[derive(Default)]
pub(super) struct Messages {
    worker: Option<NotificationWorker>,
    generation: u64,
    sequence: u64,
    snapshot: NotificationSnapshot,
}

impl IslandManager {
    pub(super) fn reminder_count(&self) -> usize {
        self.messages.snapshot.visible_reminders().count()
    }

    pub fn notification_status(&self) -> NotificationStatus {
        NotificationStatus {
            sequence: self.messages.sequence,
            permission: self.messages.snapshot.permission,
        }
    }

    fn receive_messages(&mut self, snapshot: NotificationSnapshot) -> Result<()> {
        let resized = self.reminder_count() != snapshot.visible_reminders().count();
        let previous = self.messages.snapshot.permission;
        self.messages.snapshot = snapshot;
        if let Some(window) = &self.window {
            window.set_notifications(self.messages.snapshot.clone())?;
        }
        if resized
            && self.messages.worker.is_some()
            && self.window.is_some()
            && self.config.enabled
            && self.config.notifications.enabled
        {
            self.refresh()?;
        }
        if previous != self.messages.snapshot.permission {
            self.messages.sequence += 1;
            self.app
                .emit_to(
                    "main",
                    "island://notifications-changed",
                    self.notification_status(),
                )
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub(super) fn stop_messages(&mut self) -> Result<()> {
        self.messages.generation += 1;
        drop(self.messages.worker.take());
        self.receive_messages(NotificationSnapshot::default())
    }

    pub(super) fn configure_messages(&mut self) -> Result<()> {
        if !self.config.enabled || !self.config.notifications.enabled {
            if self.messages.worker.is_some()
                || self.messages.snapshot.permission != Permission::Disabled
            {
                self.stop_messages()?;
            }
            return Ok(());
        }
        if self.messages.worker.is_some() {
            return Ok(());
        }
        let manager_handle = self.worker.clone();
        let generation = self.messages.generation;
        let latest = Arc::new(Mutex::new(None));
        self.messages.worker = Some(NotificationWorker::start(move |snapshot| {
            if latest.lock().unwrap().replace(snapshot).is_some() {
                return;
            }
            let latest = latest.clone();
            let _ = manager_handle.post(move |manager| {
                let snapshot = latest.lock().unwrap().take();
                if manager.messages.generation == generation {
                    if let Some(snapshot) = snapshot {
                        manager.receive_messages(snapshot)?;
                    }
                }
                Ok(())
            });
        })?);
        Ok(())
    }

    pub fn refresh_message_source(&self) {
        if let Some(worker) = &self.messages.worker {
            worker.refresh();
        }
    }

    pub fn can_request_notification_access(&self) -> Result<()> {
        if self.messages.worker.is_some() {
            Ok(())
        } else {
            Err("请先启用灵动岛并保存通知设置。".into())
        }
    }

    pub fn activate_notification_window(&mut self, id: u64) -> Result<()> {
        let worker = self.messages.worker.as_ref().ok_or("通知尚未启用。")?;
        worker.activate(id)?;
        self.transition(Event::WindowActivated)
    }
}
