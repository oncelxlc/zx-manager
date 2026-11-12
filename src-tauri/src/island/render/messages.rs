use crate::island::{
    config::IslandConfig,
    geometry::Rect,
    media::MediaSnapshot,
    notifications::{AppIcon, NotificationSnapshot, Source, WindowEntry},
    state::IslandState,
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MessageAction {
    Window(u64),
    OpenInstances(Source),
    NextPage,
    Previous,
    TogglePlayback,
    Next,
    Seek,
    OpenSettings,
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
    pub pet: Option<Rect>,
    pub pet_pose: PetPose,
    pub battery: Option<(Rect, u8, bool)>,
    pub waveform: Option<Rect>,
    pub wave_phase: u8,
    pub music: Option<MediaSnapshot>,
    pub time: Option<(Rect, String)>,
    pub subtitle: Option<(Rect, String)>,
    pub status: String,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum PetPose {
    Awake,
    Blink,
    Dozing,
    Sleeping,
}

#[derive(Default)]
pub(crate) struct Messages {
    pub snapshot: NotificationSnapshot,
    pub media: MediaSnapshot,
    pub page: usize,
    pub focus_source: Option<Source>,
}

impl Panel {
    pub fn narrow_music(&self) -> bool {
        self.size.0 < self.size.1 * 3 / 2
    }

    pub fn music_track(&self) -> Rect {
        if self.narrow_music() {
            rect(12, 162, 116, 4)
        } else {
            rect(96, 91, 221, 4)
        }
    }

    // The reference music page and its hit regions use a 419 × 140 DIP canvas.
    pub fn music_frame(&self) -> (f32, f32, f32) {
        let (w, h) = (self.size.0 as f32, self.size.1 as f32);
        let (cw, ch) = if self.narrow_music() {
            (140.0, 226.0)
        } else {
            (419.0, 140.0)
        };
        let scale = (w / cw).min(h / ch);
        ((w - cw * scale) / 2.0, (h - ch * scale) / 2.0, scale)
    }

    fn music_control(
        &mut self,
        bounds: Rect,
        action: Option<MessageAction>,
        name: &str,
        glyph: &str,
    ) {
        let (x, y, scale) = self.music_frame();
        let bounds = rect(
            (x + bounds.x as f32 * scale).round() as i32,
            (y + bounds.y as f32 * scale).round() as i32,
            (bounds.width as f32 * scale).round() as i32,
            (bounds.height as f32 * scale).round() as i32,
        );
        self.items.push(Item {
            bounds,
            action,
            name: name.into(),
            text: glyph.into(),
            source: None,
            icon: None,
            reminder: false,
        });
    }

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
            .filter(|s| reminded(*s) && windows.iter().any(|w| w.source == *s))
            .collect();
        let groups: Vec<_> = sources.iter().map(|s| s.label()).collect();
        let group_status = groups.join("、");
        let status = if group_status.is_empty() {
            time.into()
        } else {
            format!("{time}，{group_status}有系统提醒")
        };
        let mut panel = Self {
            size: (w, h),
            items: vec![],
            pet: None,
            pet_pose: PetPose::Awake,
            battery: None,
            waveform: None,
            wave_phase: 0,
            music: None,
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
            let media = &messages.media;
            let clock = if vertical { 20 } else { 44.min(w) };
            let shown = sources.len().min((long / side) as usize);
            let icons = side * shown as i32;
            let has_clock = long >= clock + icons;
            let mut reserved = icons + if has_clock { clock } else { 0 };
            let mut reserve = |length: i32, present: bool| {
                if present && long - reserved >= length + 12 {
                    reserved += length + 12;
                    length + 12
                } else {
                    0
                }
            };
            let pet = reserve(32, !vertical);
            let battery = reserve(20, media.battery.is_some());
            let wave = reserve(14, !media.title.is_empty());
            let mut cursor = (long - reserved) / 2;
            if battery > 0 {
                panel.battery = media
                    .battery
                    .map(|(pct, charging)| (along(cursor, 20, side.min(20)), pct, charging));
                cursor += battery;
            }
            if pet > 0 {
                panel.pet = Some(along(cursor, 32, side.min(18)));
                cursor += pet;
            }
            if has_clock {
                panel.time = Some((
                    along(cursor, clock, if vertical { w } else { h }),
                    time.into(),
                ));
                cursor += clock;
            }
            if wave > 0 {
                panel.waveform = Some(along(cursor + 12, 14, side.min(16)));
                cursor += wave;
            }
            for (index, source) in sources.into_iter().take(shown).enumerate() {
                let mut instances = windows.iter().filter(|w| w.source == source);
                let Some(entry) = instances.next() else {
                    continue;
                };
                let bounds = along(cursor + index as i32 * side, side, side);
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

        let media = &messages.media;
        let instances: Vec<_> = windows
            .iter()
            .filter(|entry| Some(entry.source) == messages.focus_source)
            .collect();
        if instances.is_empty() {
            if w < 96 || h < 56 {
                panel.time = Some((rect(0, 0, w, h), time.into()));
                return Some(panel);
            }
            let narrow = panel.narrow_music();
            let header_x = if narrow { 58 } else { 210 };
            let header_width = if narrow { 108 } else { 140 };
            let start = header_x - header_width / 2;
            let battery = media.battery.filter(|_| !narrow);
            let date_width = header_width - 39 - if battery.is_some() { 20 } else { 0 };
            if let Some((pct, charging)) = battery {
                panel.battery = Some((rect(start, 11, 15, 15), pct, charging));
            }
            let date_x = start + if battery.is_some() { 20 } else { 0 };
            panel.time = Some((rect(date_x + date_width, 7, 39, 23), time.into()));
            panel.subtitle = Some((rect(date_x, 7, date_width, 23), date.into()));
            if !media.title.is_empty() && !narrow {
                panel.waveform = Some(rect(359, 10, 16, 16));
            }
            panel.music = Some(media.clone());
            let has = !media.title.is_empty();
            panel.status = format!(
                "{date} {time}，{}",
                if has { &media.title } else { "暂无音乐" }
            );
            use MessageAction::*;
            panel.music_control(
                rect(if narrow { 112 } else { 380 }, 4, 28, 28),
                Some(OpenSettings),
                "打开灵动岛设置",
                "\u{e713}",
            );
            let track = panel.music_track();
            panel.music_control(
                rect(track.x, track.y - 6, track.width, 15),
                (has && media.duration_ms > 0 && media.can_seek).then_some(Seek),
                "播放进度",
                "",
            );
            let (label, glyph) = if media.playing {
                ("暂停", "⏸")
            } else {
                ("播放", "▶")
            };
            let controls = [
                (74, 15, None, "歌词", "词"),
                (
                    162,
                    42,
                    (has && media.can_previous).then_some(Previous),
                    "上一首",
                    "⏪",
                ),
                (
                    211,
                    70,
                    (has && media.can_toggle).then_some(TogglePlayback),
                    label,
                    glyph,
                ),
                (
                    258,
                    98,
                    (has && media.can_next).then_some(Next),
                    "下一首",
                    "⏩",
                ),
                (367, 125, None, "音乐应用音量", "\u{e767}"),
            ];
            for (wide_x, narrow_x, action, label, glyph) in controls {
                let bounds = if narrow {
                    rect(narrow_x - 12, 192, 24, 24)
                } else {
                    rect(wide_x - 13, 106, 26, 24)
                };
                panel.music_control(bounds, action, label, glyph);
            }
            return Some(panel);
        }
        let pad = 12.min(w / 8).min(h / 8);
        let inner = (w - pad * 2).max(1);
        let header = if h >= 96 { 28 } else { 0 };
        if header > 0 {
            panel.time = Some((rect(pad, pad, 44.min(inner), 24), time.into()));
            if inner > 54 {
                panel.subtitle = Some((rect(pad + 50, pad, inner - 50, 24), group_status));
            }
        }
        let top = pad + header;
        let available = (h - top - pad).max(1);
        let columns: usize = if inner >= 284 { 2 } else { 1 };
        let gap = 4;
        let card_h = 30;
        let fit_rows = |height: i32| ((height + gap) / (card_h + gap)).max(1) as usize;
        let full_rows = fit_rows(available);
        let paged = instances.len() > full_rows * columns;
        let pager = if paged { 23.min(available / 2) } else { 0 };
        let body = available - pager;
        let rows = fit_rows(body);
        let capacity = rows * columns;
        let pages = instances.len().div_ceil(capacity);
        let page = messages.page % pages;
        let row_h = card_h.min(body);
        let cell = (inner - (columns as i32 - 1) * gap) / columns as i32;
        for (index, entry) in instances
            .into_iter()
            .skip(page * capacity)
            .take(capacity)
            .enumerate()
        {
            let x = pad + (index % columns) as i32 * (cell + gap);
            let y = top + (index / columns) as i32 * (row_h + gap);
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
    fn idle_compact_shows_pet_and_clock_without_inventing_battery_or_playback() {
        let p = panel(&Messages::default(), IslandState::Compact, 156, 44);
        assert!(p.items.is_empty());
        assert!(p.pet.is_some() && p.time.is_some());
        assert!(p.battery.is_none() && p.waveform.is_none());
    }
    #[test]
    fn idle_clock_fits_compact_and_expanded_sizes() {
        for (state, w, h, pet, date) in [
            (IslandState::Compact, 163, 30, true, false),
            (IslandState::Compact, 30, 163, false, false),
            (IslandState::Compact, 24, 24, false, false),
            (IslandState::Expanded, 419, 140, false, true),
            (IslandState::Pinned, 48, 120, false, false),
            (IslandState::Expanded, 48, 48, false, false),
        ] {
            let p = panel(&Messages::default(), state, w, h);
            let (clock, _) = p.time.unwrap();
            assert!(clock.x >= 0 && clock.y >= 0);
            assert!(clock.x + clock.width <= w && clock.y + clock.height <= h);
            assert_eq!(p.pet.is_some(), pet);
            assert_eq!(p.subtitle.is_some(), date);
            if let Some((bounds, _)) = p.subtitle {
                assert!(bounds.x + bounds.width <= clock.x);
            }
        }
    }
    #[test]
    fn music_controls_use_real_session_capabilities_and_fit_both_orientations() {
        let mut messages = Messages::default();
        let empty = panel(&messages, IslandState::Expanded, 419, 140);
        assert!(empty.status.contains("暂无音乐"));
        assert_eq!(
            empty
                .items
                .iter()
                .filter_map(|item| item.action)
                .collect::<Vec<_>>(),
            [MessageAction::OpenSettings]
        );
        messages.media = MediaSnapshot {
            title: "Audio".into(),
            playing: true,
            can_toggle: true,
            can_seek: true,
            duration_ms: 120_000,
            battery: Some((56, true)),
            ..Default::default()
        };
        let bar = panel(&messages, IslandState::Compact, 178, 30);
        assert!(bar.battery.is_some() && bar.waveform.is_some() && bar.pet.is_some());
        for (w, h) in [(419, 140), (328, 168), (140, 419), (260, 90)] {
            let p = panel(&messages, IslandState::Pinned, w, h);
            assert!(p.music.is_some() && p.status.contains("Audio"));
            assert!(p
                .items
                .iter()
                .any(|i| i.action == Some(MessageAction::Seek)));
            assert!(p
                .items
                .iter()
                .any(|i| i.action == Some(MessageAction::TogglePlayback) && i.name == "暂停"));
            for item in &p.items {
                let r = item.bounds;
                assert!(r.x >= 0 && r.y >= 0 && r.x + r.width <= w && r.y + r.height <= h);
            }
        }
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
        assert_eq!(compact.items.len(), 1);
        assert_eq!(
            compact.items[0].action,
            Some(MessageAction::OpenInstances(Source::Wechat))
        );
        assert!(compact.items[0].reminder);
        assert!(panel(&messages, IslandState::Expanded, 419, 140)
            .music
            .is_some());
        messages.snapshot.windows.pop();
        messages.focus_source = Some(Source::Wechat);
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
