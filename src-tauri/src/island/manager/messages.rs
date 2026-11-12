use super::*;
use crate::island::notifications::{
    NotificationSnapshot, NotificationWorker, Permission, Source, WindowEntry,
};
use std::sync::{Arc, Mutex};

#[derive(Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NotificationStatus {
    sequence: u64,
    permission: Permission,
    windows: Vec<WindowEntry>,
    reminders: Vec<Source>,
}

#[derive(Default)]
pub(super) struct Messages {
    worker: Option<NotificationWorker>,
    generation: u64,
    epoch: u64,
    sequence: u64,
    pub(super) locked: bool,
    snapshot: NotificationSnapshot,
    source: Option<Source>,
    selected: Option<u64>,
}

impl IslandManager {
    pub fn notification_status(&self) -> NotificationStatus {
        let messages = &self.messages;
        NotificationStatus {
            sequence: messages.sequence,
            permission: messages.snapshot.permission,
            windows: messages.snapshot.windows.clone(),
            reminders: messages
                .snapshot
                .reminders
                .iter()
                .map(|r| r.source)
                .collect(),
        }
    }

    fn publish_messages(&mut self, previous: NotificationStatus) -> Result<()> {
        let drawn = self.draw_messages();
        let mut status = self.notification_status();
        if previous != status {
            self.messages.sequence += 1;
            status.sequence = self.messages.sequence;
            self.app
                .emit_to("main", "island://notifications-changed", status)
                .map_err(|err| err.to_string())?;
        }
        drawn
    }

    fn draw_messages(&self) -> Result<()> {
        if let Some(window) = &self.window {
            window.set_notifications(
                self.messages.snapshot.clone(),
                self.messages.source,
                self.messages.selected,
            )?;
        }
        Ok(())
    }

    pub(super) fn stop_messages(&mut self) {
        let previous = self.notification_status();
        self.messages.generation += 1;
        let worker = self.messages.worker.take();
        self.messages.epoch = 0;
        self.messages.snapshot = NotificationSnapshot::default();
        self.messages.selected = None;
        self.messages.source = None;
        if let Err(error) = self.publish_messages(previous) {
            self.report(error);
        }
        drop(worker);
    }

    pub(super) fn configure_messages(&mut self) -> Result<()> {
        let cfg = self.config.notifications.clone();
        if !self.config.enabled || !cfg.enabled {
            if self.messages.worker.is_some()
                || self.messages.snapshot.permission != Permission::Disabled
            {
                self.stop_messages();
            }
            return Ok(());
        }
        let preview = cfg.show_preview
            && !self.messages.locked
            && matches!(
                self.machine.state,
                IslandState::Compact | IslandState::Expanded | IslandState::Pinned
            );
        if let Some(worker) = &self.messages.worker {
            let epoch = worker.configure(cfg.clone(), preview);
            if self.messages.epoch != epoch {
                let previous = self.notification_status();
                self.messages.epoch = epoch;
                let snapshot = &mut self.messages.snapshot;
                snapshot.windows.retain(|w| w.source.enabled(&cfg));
                snapshot
                    .reminders
                    .retain(|r| cfg.windows_notifications_enabled && r.source.enabled(&cfg));
                for reminder in &mut snapshot.reminders {
                    reminder.preview = None;
                }
                if !cfg.windows_notifications_enabled {
                    snapshot.permission = Permission::Disabled;
                }
                self.reconcile_selection();
                return self.publish_messages(previous);
            }
            return Ok(());
        }
        let manager_handle = self.worker.clone();
        let generation = self.messages.generation;
        // A notification burst replaces the pending snapshot instead of queuing message bodies.
        let latest = Arc::new(Mutex::new(None));
        match NotificationWorker::start(cfg, preview, move |snapshot| {
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
        }) {
            Ok(worker) => self.messages.worker = Some(worker),
            Err(_) => {
                let previous = self.notification_status();
                self.messages.snapshot.permission = Permission::Error;
                self.publish_messages(previous)?;
            }
        }
        Ok(())
    }

    fn receive_messages(&mut self, snapshot: NotificationSnapshot) -> Result<()> {
        if snapshot.epoch != self.messages.epoch {
            return Ok(());
        }
        let previous = self.notification_status();
        self.messages.snapshot = snapshot;
        self.reconcile_selection();
        self.publish_messages(previous)
    }

    fn reconcile_selection(&mut self) {
        let messages = &mut self.messages;
        if !messages
            .snapshot
            .windows
            .iter()
            .any(|w| Some(w.id) == messages.selected)
        {
            messages.selected = None;
        }
        messages.source = messages
            .source
            .filter(|s| s.enabled(&self.config.notifications));
    }

    pub fn refresh_message_source(&self) {
        if let Some(worker) = &self.messages.worker {
            worker.refresh();
        }
    }

    pub fn can_request_notification_access(&self) -> Result<()> {
        let config = &self.config.notifications;
        if self.messages.worker.is_some()
            && config.windows_notifications_enabled
            && (config.wechat_enabled || config.qq_enabled)
        {
            Ok(())
        } else {
            Err("请先启用并保存 Windows 通知监听".into())
        }
    }

    pub fn activate_notification_window(&mut self, id: u64) -> Result<()> {
        let entry = self
            .messages
            .snapshot
            .windows
            .iter()
            .find(|w| w.id == id)
            .ok_or("窗口已关闭，请重新选择")?;
        self.messages.selected = Some(id);
        self.messages.source = Some(entry.source);
        self.draw_messages()?;
        let worker = self.messages.worker.as_ref().ok_or("消息来源尚未启动")?;
        worker.activate(id)
    }

    pub(super) fn open_source(&mut self, source: Source) -> Result<()> {
        self.messages.source = Some(source);
        self.messages.selected = None;
        let mut matches = self
            .messages
            .snapshot
            .windows
            .iter()
            .filter(|w| w.source == source);
        match (matches.next(), matches.next()) {
            (Some(window), None) if !self.config.notifications.account_list_enabled => {
                return self.activate_notification_window(window.id);
            }
            (_, Some(_)) if !self.config.notifications.account_list_enabled => {
                return self.open_message_settings();
            }
            _ => {}
        }
        self.draw_messages()?;
        self.transition(Event::OpenNotifications)
    }
}
