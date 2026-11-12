use crate::island::{
    config::IslandConfig,
    geometry::Rect,
    notifications::{AppIcon, NotificationSnapshot, Source, WindowEntry},
    state::IslandState,
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MessageAction {
    Window(u64),
    OpenInstances(Source),
    NextPage,
}

// Local DIP coordinates are shared by Direct2D, the pointer and MSAA.
#[derive(Clone, PartialEq)]
pub(crate) struct Item {
    pub bounds: Rect,
    pub action: Option<MessageAction>,
    pub name: String,
    pub text: String,
    pub source: Option<Source>,
    pub icon: Option<Arc<AppIcon>>,
    pub reminder: bool,
}

impl Item {
    pub fn screen_bounds(&self, visible: Rect, size: (i32, i32)) -> Rect {
        let x = |n| {
            visible.x + (f64::from(n) * f64::from(visible.width) / f64::from(size.0)).round() as i32
        };
        let y = |n| {
            visible.y
                + (f64::from(n) * f64::from(visible.height) / f64::from(size.1)).round() as i32
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

fn rect(x: i32, y: i32, width: i32, height: i32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct Panel {
    pub size: (i32, i32),
    pub items: Vec<Item>,
    pub ring: Option<Rect>,
    pub time: Option<(Rect, String)>,
    pub subtitle: Option<(Rect, String)>,
    pub status: String,
}

#[derive(Default)]
pub(crate) struct Messages {
    pub snapshot: NotificationSnapshot,
    pub page: usize,
    pub focus_source: Option<Source>,
}

impl Panel {
    fn instance_item(bounds: Rect, entry: &WindowEntry, reminder: bool, text: String) -> Item {
        let mut name = if entry.can_activate {
            format!("激活 {}", entry.label)
        } else {
            format!("{}，暂无可激活窗口", entry.label)
        };
        if reminder {
            name.push_str("，该应用有系统提醒");
        }
        Item {
            bounds,
            action: entry
                .can_activate
                .then_some(MessageAction::Window(entry.id)),
            name,
            text,
            source: Some(entry.source),
            icon: entry.icon.clone(),
            reminder,
        }
    }

    pub fn build(
        messages: &Messages,
        cfg: &IslandConfig,
        state: IslandState,
        target: Rect,
        scale: f64,
        time: &str,
        date: &str,
    ) -> Option<Self> {
        if !matches!(
            state,
            IslandState::Compact | IslandState::Expanded | IslandState::Pinned
        ) {
            return None;
        }
        let w = (f64::from(target.width) / scale).round().max(1.0) as i32;
        let h = (f64::from(target.height) / scale).round().max(1.0) as i32;
        let snapshot = &messages.snapshot;
        let windows = if cfg.notifications.enabled {
            snapshot.windows.as_slice()
        } else {
            &[]
        };
        let reminded = |s| snapshot.reminders.iter().any(|r| r.source == s);
        let sources: Vec<_> = [Source::Wechat, Source::Qq]
            .into_iter()
            .filter(|s| windows.iter().any(|w| w.source == *s))
            .collect();
        let groups: Vec<_> = sources
            .iter()
            .filter(|s| reminded(**s))
            .map(|s| s.label())
            .collect();
        let group_status = groups.join("、");
        let status = if group_status.is_empty() {
            time.into()
        } else {
            format!("{time}，{group_status}有系统提醒")
        };
        let mut panel = Self {
            size: (w, h),
            items: vec![],
            ring: None,
            time: None,
            subtitle: None,
            status,
        };
        if state == IslandState::Compact {
            let vertical = h > w;
            let long = w.max(h);
            let side = w.min(h).min(28);
            let along = |offset, length, cross| {
                if vertical {
                    rect((w - cross) / 2, offset, cross, length)
                } else {
                    rect(offset, (h - cross) / 2, length, cross)
                }
            };
            let clock = if vertical { 24 } else { 50 };
            let ring = 16.min(side);
            let icon_space = side * sources.len() as i32;
            let has_clock = long >= ring + clock + icon_space + 10;
            let has_ring = has_clock || long >= ring + icon_space + 4 || sources.is_empty();
            let reserved = if has_clock {
                ring + 6 + clock
            } else if has_ring {
                ring
            } else {
                0
            };
            let count = ((long - reserved).max(0) / side) as usize;
            let shown = count.min(sources.len());
            let start = (long - reserved - side * shown as i32) / 2;
            if has_ring {
                panel.ring = Some(along(start, ring, ring));
            }
            if has_clock {
                panel.time = Some((
                    along(start + ring + 6, clock, if vertical { w } else { h }),
                    time.into(),
                ));
            }
            for (index, source) in sources.into_iter().take(shown).enumerate() {
                let mut instances = windows.iter().filter(|w| w.source == source);
                let Some(entry) = instances.next() else {
                    continue;
                };
                let bounds = along(start + reserved + index as i32 * side, side, side);
                let mut item = Self::instance_item(bounds, entry, reminded(source), String::new());
                if instances.next().is_some() {
                    item.action = Some(MessageAction::OpenInstances(source));
                    item.name = format!(
                        "选择{}实例{}",
                        source.label(),
                        if item.reminder {
                            "，该应用有系统提醒"
                        } else {
                            ""
                        }
                    );
                }
                panel.items.push(item);
            }
            return Some(panel);
        }

        let pad = 10.min(w / 8).min(h / 8);
        let inner = (w - pad * 2).max(1);
        if windows.is_empty() {
            let center = h / 2;
            let width = 76.min(inner);
            let x = (w - width) / 2;
            if width >= 66 {
                panel.ring = Some(rect(x, center - 14, 16, 16));
                panel.time = Some((rect(x + 18, center - 20, width - 18, 28), time.into()));
            } else {
                panel.time = Some((rect(0, (center - 20).max(0), w, 28.min(h)), time.into()));
            }
            if h >= 48 {
                panel.subtitle = Some((
                    rect(pad, center + 7, inner, 22.min(h - center - 7)),
                    date.into(),
                ));
            }
            return Some(panel);
        }
        let header = if h >= 96 && w >= 100 { 34 } else { 0 };
        if header > 0 {
            panel.ring = Some(rect(pad + 4, pad + 6, 16, 16));
            panel.time = Some((rect(pad + 22, pad, 50, 28), time.into()));
            let detail = if groups.is_empty() {
                date.into()
            } else {
                format!("{group_status}有系统提醒")
            };
            panel.subtitle = Some((rect(pad + 76, pad, (inner - 76).max(1), 28), detail));
        }
        let top = pad + header;
        let available = (h - top - pad).max(1);
        let columns: usize = if inner >= 284 { 2 } else { 1 };
        let full_rows = (available / 34).max(1) as usize;
        let paged = windows.len() > full_rows * columns;
        let pager = if paged { 23.min(available / 2) } else { 0 };
        let body = available - pager;
        let rows = (body / 34).max(1) as usize;
        let capacity = rows * columns;
        let pages = windows.len().div_ceil(capacity);
        let page = messages.page % pages;
        let row_h = 34.min(body);
        let cell = inner / columns as i32;
        let selected = |w: &&WindowEntry| Some(w.source) == messages.focus_source;
        let ordered = windows
            .iter()
            .filter(selected)
            .chain(windows.iter().filter(|w| !selected(w)));
        for (index, entry) in ordered.skip(page * capacity).take(capacity).enumerate() {
            let x = pad + (index % columns) as i32 * cell;
            let y = top + (index / columns) as i32 * row_h;
            let label = if cell < 68 {
                entry.id.to_string()
            } else {
                entry.label.clone()
            };
            panel.items.push(Self::instance_item(
                rect(x, y, cell, row_h),
                entry,
                false,
                label,
            ));
        }
        if paged {
            let label = format!("{} / {}  ›", page + 1, pages);
            panel.items.push(Item {
                bounds: rect(pad, h - pad - pager, inner, pager),
                action: Some(MessageAction::NextPage),
                name: format!("下一页，当前第 {} 页，共 {pages} 页", page + 1),
                text: label,
                source: None,
                icon: None,
                reminder: false,
            });
        }
        Some(panel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::island::notifications::Reminder;
    fn entry(id: u64, source: Source) -> WindowEntry {
        WindowEntry {
            id,
            source,
            label: format!("{} {id}", source.label()),
            can_activate: true,
            icon: None,
        }
    }
    fn panel(messages: &Messages, state: IslandState, w: i32, h: i32) -> Panel {
        Panel::build(
            messages,
            &IslandConfig::default(),
            state,
            rect(-w, 0, w, h),
            1.0,
            "09:03",
            "11月12日",
        )
        .unwrap()
    }
    #[test]
    fn no_running_instance_keeps_only_status_and_clock() {
        let p = panel(&Messages::default(), IslandState::Compact, 156, 44);
        assert!(p.items.is_empty());
        assert!(p.ring.is_some() && p.time.is_some());
    }
    #[test]
    fn ambiguous_toast_only_marks_the_group_and_each_instance_has_its_own_action() {
        let mut messages = Messages::default();
        messages.snapshot.windows = (1..=9).map(|id| entry(id, Source::Wechat)).collect();
        messages.snapshot.reminders.push(Reminder {
            source: Source::Wechat,
            latest_id: 1,
            created_ticks: 1,
        });
        messages.snapshot.windows.push(entry(10, Source::Qq));
        let compact = panel(&messages, IslandState::Compact, 156, 44);
        assert_eq!(compact.items.len(), 2);
        assert_eq!(
            compact.items[0].action,
            Some(MessageAction::OpenInstances(Source::Wechat))
        );
        assert!(compact.items[0].reminder);
        assert_eq!(compact.items[1].action, Some(MessageAction::Window(10)));
        messages.snapshot.windows.pop();
        for (w, h) in [(328, 168), (48, 48), (168, 328)] {
            let mut seen = std::collections::HashSet::new();
            for index in 0..9 {
                messages.page = index;
                let p = panel(&messages, IslandState::Expanded, w, h);
                assert!(p.status.contains("微信有系统提醒"));
                for item in p.items {
                    assert!(
                        !item.reminder
                            && item.bounds.height > 0
                            && item.bounds.y + item.bounds.height <= h
                    );
                    if let Some(MessageAction::Window(id)) = item.action {
                        seen.insert(id);
                    }
                }
                if seen.len() == 9 {
                    break;
                }
            }
            assert_eq!(seen.len(), 9);
        }
        messages.snapshot.windows.truncate(1);
        let p = panel(&messages, IslandState::Compact, 156, 44);
        assert!(p.items[0].reminder);
        assert_eq!(p.items[0].action, Some(MessageAction::Window(1)));
    }
}
