use crate::island::{
    config::IslandConfig,
    geometry::Rect,
    notifications::{NotificationSnapshot, Permission, Source},
    state::IslandState,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MessageAction {
    Source(Source),
    Window(u64),
    Settings,
    NextPage,
    Pin(bool),
    Hide,
}

// Local DIP coordinates are shared by Direct2D, the pointer and MSAA.
#[derive(Clone, PartialEq)]
pub(crate) struct Item {
    pub bounds: Rect,
    pub action: MessageAction,
    pub name: String,
    pub text: String,
    pub source: Option<Source>,
    pub reminder: bool,
    pub selected: bool,
}

impl Item {
    fn new(bounds: Rect, action: MessageAction, text: &str, name: &str) -> Self {
        Self {
            bounds,
            action,
            name: name.into(),
            text: text.into(),
            source: None,
            reminder: false,
            selected: false,
        }
    }

    pub fn screen_bounds(&self, visible: Rect, target_dips: (i32, i32)) -> Rect {
        let x = |n: i32| {
            visible.x
                + (f64::from(n) * f64::from(visible.width) / f64::from(target_dips.0)).round()
                    as i32
        };
        let y = |n: i32| {
            visible.y
                + (f64::from(n) * f64::from(visible.height) / f64::from(target_dips.1)).round()
                    as i32
        };
        let r = self.bounds;
        rect(
            x(r.x),
            y(r.y),
            x(r.x + r.width) - x(r.x),
            y(r.y + r.height) - y(r.y),
        )
    }
}

pub(super) fn rect(x: i32, y: i32, width: i32, height: i32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct Caption {
    pub bounds: Rect,
    pub text: String,
    pub primary: bool,
}

#[derive(Clone, PartialEq)]
pub(crate) struct Panel {
    pub size: (i32, i32),
    pub items: Vec<Item>,
    pub captions: Vec<Caption>,
    pub contains_preview: bool,
    pub pulse: Option<(Source, u8)>,
}

#[derive(Default)]
pub(crate) struct Messages {
    pub snapshot: Option<NotificationSnapshot>,
    pub selected_source: Option<Source>,
    pub selected_window: Option<u64>,
    pub page: usize,
}

impl Panel {
    fn caption(&mut self, bounds: Rect, text: &str, primary: bool) {
        self.captions.push(Caption {
            bounds,
            text: text.into(),
            primary,
        });
    }

    fn source(
        &mut self,
        bounds: Rect,
        source: Source,
        snapshot: &NotificationSnapshot,
        selected_source: Option<Source>,
    ) {
        let mut windows = snapshot
            .windows
            .iter()
            .filter(|window| window.source == source);
        let first = windows.next();
        let unread = snapshot.reminders.iter().any(|r| r.source == source);
        let status = if unread {
            "有系统提醒"
        } else if first.is_some() {
            "有可用窗口"
        } else {
            "未发现可用窗口"
        };
        // Compact controls carry their window ID even if hover has already expanded the island.
        let (action, name) = match (selected_source, first, windows.next()) {
            (None, Some(window), None) => (
                MessageAction::Window(window.id),
                format!("激活 {}，{status}", window.label),
            ),
            _ => (
                MessageAction::Source(source),
                format!("{}，{status}", source.label()),
            ),
        };
        let mut item = Item::new(bounds, action, "", &name);
        item.source = Some(source);
        item.reminder = unread;
        item.selected = selected_source == Some(source);
        self.items.push(item);
    }

    fn settings(&mut self, bounds: Rect, short: bool) {
        self.items.push(Item::new(
            bounds,
            MessageAction::Settings,
            if short { "···" } else { "通知设置" },
            "打开通知设置和窗口列表",
        ));
    }

    pub fn build(
        messages: &Messages,
        cfg: &IslandConfig,
        state: IslandState,
        target: Rect,
        scale: f64,
    ) -> Option<Self> {
        let snapshot = messages.snapshot.as_ref()?;
        let config = &cfg.notifications;
        if !config.enabled
            || !matches!(
                state,
                IslandState::Compact | IslandState::Expanded | IslandState::Pinned
            )
        {
            return None;
        }
        let sources: Vec<_> = [
            (Source::Wechat, config.wechat_enabled),
            (Source::Qq, config.qq_enabled),
        ]
        .into_iter()
        .filter_map(|(s, enabled)| enabled.then_some(s))
        .collect();
        let (w, h) = (
            ((f64::from(target.width) / scale).round() as i32).max(1),
            ((f64::from(target.height) / scale).round() as i32).max(1),
        );
        let mut panel = Self {
            size: (w, h),
            items: Vec::new(),
            captions: Vec::new(),
            contains_preview: false,
            pulse: None,
        };
        if sources.is_empty() {
            return None;
        }
        if state == IslandState::Compact {
            let vertical = h > w;
            let (long, short) = if vertical { (h, w) } else { (w, h) };
            let side = short.clamp(16, 30);
            let icons = side * sources.len() as i32;
            if long < icons + 4 {
                // A single target stays usable even at the legacy minimum of 24 DIP.
                let source = snapshot
                    .reminders
                    .iter()
                    .find(|r| sources.contains(&r.source))
                    .map(|r| r.source)
                    .unwrap_or(sources[0]);
                if sources.len() == 1 {
                    panel.source(
                        rect((w - side) / 2, (h - side) / 2, side, side),
                        source,
                        snapshot,
                        None,
                    );
                } else {
                    panel.settings(rect(0, 0, w, h), true);
                    panel.items[0].reminder = snapshot
                        .reminders
                        .iter()
                        .any(|r| sources.contains(&r.source));
                }
                return Some(panel);
            }
            let brand = !vertical && long >= icons + 68;
            if brand {
                panel.caption(rect(12, 0, w - icons - 20, h), "zx-once", true);
            }
            let start = if brand {
                w - icons - 8
            } else {
                (long - icons) / 2
            };
            for (index, source) in sources.into_iter().enumerate() {
                let pos = start + side * index as i32;
                let bounds = if vertical {
                    rect((w - side) / 2, pos, side, side)
                } else {
                    rect(pos, (h - side) / 2, side, side)
                };
                panel.source(bounds, source, snapshot, None);
            }
            return Some(panel);
        }
        if w < 120 || h < 102 {
            panel.settings(rect(4, (h - 32) / 2, w - 8, 32), w < 86);
            return Some(panel);
        }
        let selected = messages
            .selected_source
            .filter(|s| sources.contains(s))
            .or_else(|| {
                snapshot
                    .reminders
                    .iter()
                    .find(|r| sources.contains(&r.source))
                    .map(|r| r.source)
            })
            .unwrap_or(sources[0]);
        let wide = w >= 250;
        let has_list = config.account_list_enabled && if wide { h >= 112 } else { h >= 238 };
        let split = if wide && has_list {
            (w * 55 / 100).clamp(132, 230)
        } else {
            w
        };
        panel.caption(rect(12, 7, split - 22, 22), "通知与消息", true);
        for (index, s) in sources.iter().enumerate() {
            panel.source(
                rect(10 + index as i32 * 34, 32, 30, 28),
                *s,
                snapshot,
                Some(selected),
            );
        }
        if !has_list {
            let width = (w - 84).clamp(30, 68);
            let text = if width >= 54 {
                "窗口列表"
            } else {
                "窗口"
            };
            panel.items.push(Item::new(
                rect(w - width - 6, 32, width, 28),
                MessageAction::Settings,
                text,
                "打开窗口列表",
            ));
        }
        let win_count = snapshot
            .windows
            .iter()
            .filter(|entry| entry.source == selected)
            .count();
        let reminder = snapshot.reminders.iter().find(|r| r.source == selected);
        let status = if reminder.is_some() {
            "有系统提醒"
        } else if win_count == 0 {
            "未发现窗口"
        } else if snapshot.permission == Permission::Allowed {
            "暂无新提醒"
        } else {
            "系统提醒不可用"
        };
        let summary = format!("{} · {status}", selected.label());
        let bottom = h - 36;
        if bottom >= 81 {
            panel.caption(rect(12, 62, split - 20, 20), &summary, false);
        }
        let preview = reminder
            .and_then(|r| r.preview.as_deref())
            .filter(|s| cfg.notifications.show_preview && !s.is_empty());
        if bottom >= 108 {
            let selected_label = snapshot
                .windows
                .iter()
                .find(|entry| {
                    Some(entry.id) == messages.selected_window && entry.source == selected
                })
                .map(|entry| entry.label.as_str());
            if let Some(text) = preview.or(selected_label) {
                panel.caption(rect(12, 84, split - 23, 22), text, false);
                panel.contains_preview = preview.is_some();
            }
        }
        let mut x = 10;
        let pinned = state == IslandState::Pinned;
        if cfg.interaction.pin_enabled {
            let mut item = Item::new(
                rect(x, bottom, 44, 28),
                MessageAction::Pin(!pinned),
                if pinned { "已固定" } else { "固定" },
                if pinned {
                    "取消固定"
                } else {
                    "固定岛屿"
                },
            );
            item.selected = pinned;
            panel.items.push(item);
            x += 48;
        }
        panel.items.push(Item::new(
            rect(x, bottom, 44, 28),
            MessageAction::Hide,
            "隐藏",
            "隐藏岛屿",
        ));
        if !has_list {
            return Some(panel);
        }
        let (list_x, list_y, list_w, end) = if wide {
            (split, 8, w - split - 10, h - 8)
        } else {
            (10, 112, w - 20, bottom - 4)
        };
        panel.caption(rect(list_x + 3, list_y, list_w - 4, 20), "窗口", true);
        let start = list_y + 25;
        if win_count == 0 {
            panel.caption(
                rect(list_x + 3, start, list_w - 4, 24),
                "暂无可用窗口",
                false,
            );
            return Some(panel);
        }
        let max_rows = ((end - start) / 30).max(1) as usize;
        let paged = win_count > max_rows;
        let rows = if paged {
            max_rows.saturating_sub(1).max(1)
        } else {
            max_rows
        };
        let pages = win_count.div_ceil(rows);
        let page = messages.page % pages;
        for (index, entry) in snapshot
            .windows
            .iter()
            .filter(|w| w.source == selected)
            .skip(page * rows)
            .take(rows)
            .enumerate()
        {
            let mut item = Item::new(
                rect(list_x, start + index as i32 * 30, list_w, 28),
                MessageAction::Window(entry.id),
                &entry.label,
                &format!("激活 {}", entry.label),
            );
            item.source = Some(entry.source);
            item.selected = Some(entry.id) == messages.selected_window;
            panel.items.push(item);
        }
        if paged {
            panel.items.push(Item::new(
                rect(list_x, end - 26, list_w, 26),
                MessageAction::NextPage,
                &format!("{} / {}  ›", page + 1, pages),
                "下一页窗口",
            ));
        }
        Some(panel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::island::notifications::{Reminder, WindowEntry};

    #[test]
    fn compact_single_window_target_survives_hover_expansion() {
        use crate::island::state::{Event, Machine};

        let cfg = IslandConfig::default();
        let messages = Messages {
            snapshot: Some(NotificationSnapshot {
                windows: [(11, Source::Wechat), (12, Source::Wechat), (20, Source::Qq)]
                    .into_iter()
                    .map(|(id, source)| WindowEntry {
                        id,
                        source,
                        label: format!("{} {id}", source.label()),
                    })
                    .collect(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut machine = Machine::new(&cfg);
        let compact =
            Panel::build(&messages, &cfg, machine.state, rect(0, 0, 156, 44), 1.0).unwrap();
        machine.apply(Event::PointerEnter, &cfg);
        assert_eq!(machine.state, IslandState::Expanded);
        let expanded =
            Panel::build(&messages, &cfg, machine.state, rect(0, 0, 328, 168), 1.0).unwrap();
        let icon = |panel: &Panel, source| {
            panel
                .items
                .iter()
                .find(|i| i.source == Some(source) && i.text.is_empty())
                .unwrap()
                .action
        };
        assert_eq!(icon(&compact, Source::Qq), MessageAction::Window(20));
        assert_eq!(
            icon(&compact, Source::Wechat),
            MessageAction::Source(Source::Wechat)
        );
        assert_eq!(
            icon(&expanded, Source::Qq),
            MessageAction::Source(Source::Qq)
        );
    }

    #[test]
    fn source_reminder_never_marks_a_window_and_every_page_fits() {
        let cfg = IslandConfig::default();
        let mut messages = Messages {
            snapshot: Some(NotificationSnapshot {
                windows: (1..=9)
                    .map(|id| WindowEntry {
                        id,
                        source: Source::Wechat,
                        label: format!("微信 {id}"),
                    })
                    .collect(),
                reminders: vec![Reminder {
                    source: Source::Wechat,
                    preview: None,
                    latest_id: 5,
                    created_ticks: 50,
                }],
                ..Default::default()
            }),
            ..Default::default()
        };
        let target = rect(-328, 0, 328, 168);
        let mut exposed = std::collections::HashSet::new();
        for page in 0..3 {
            messages.page = page;
            let panel = Panel::build(&messages, &cfg, IslandState::Expanded, target, 1.0).unwrap();
            let pager = panel
                .items
                .iter()
                .find(|r| r.action == MessageAction::NextPage)
                .unwrap();
            assert!(panel
                .items
                .iter()
                .any(|r| r.action == MessageAction::Source(Source::Wechat) && r.reminder));
            for item in &panel.items {
                if let MessageAction::Window(id) = item.action {
                    assert!(!item.reminder);
                    assert!(item.bounds.y + item.bounds.height <= pager.bounds.y);
                    assert!(exposed.insert(id));
                }
            }
        }
        assert_eq!(exposed.len(), 9);
    }
}
