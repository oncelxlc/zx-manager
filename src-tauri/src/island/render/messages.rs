use super::notch::{side, Notch};
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
    ToggleLyrics,
    Next,
    Seek,
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
    body: Rect,
    pub side: bool,
    pub items: Vec<Item>,
    pub pet: Option<Rect>,
    pub pet_pose: PetPose,
    pub battery: Option<(Rect, u8, bool)>,
    pub waveform: Option<Rect>,
    pub wave_phase: u8,
    pub music: Option<MediaSnapshot>,
    pub lyrics: bool,
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
    pub fn music_track(&self) -> Rect {
        if self.lyrics {
            rect(44, 80, 176, 4)
        } else {
            rect(96, 91, 221, 4)
        }
    }

    pub fn seek_fraction(&self, visible: Rect, screen_x: i32) -> u16 {
        let (x, _, scale) = self.music_frame();
        let local = (screen_x - visible.x) as f32 * self.size.0 as f32 / visible.width as f32;
        let track = self.music_track();
        let fraction = ((local - x) / scale - track.x as f32) / track.width as f32;
        (fraction.clamp(0.0, 1.0) * 1000.0).round() as u16
    }

    // The page, its controls and its accessibility bounds use the same 419 × 140 body.
    pub fn music_frame(&self) -> (f32, f32, f32) {
        let b = self.body;
        let s = (b.width as f32 / 419.0).min(b.height as f32 / 140.0);
        (
            b.x as f32 + (b.width as f32 - 419.0 * s) / 2.0,
            b.y as f32 + (b.height as f32 - 140.0 * s) / 2.0,
            s,
        )
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
        let mut sources: Vec<_> = snapshot
            .visible_reminders()
            .filter(|_| cfg.notifications.enabled)
            .collect();
        let groups: Vec<_> = sources.iter().map(|s| s.label()).collect();
        let group_status = groups.join("、");
        let status = if group_status.is_empty() {
            time.into()
        } else {
            format!("{time}，{group_status}有系统提醒")
        };
        let body = Notch::for_config(cfg, state, f64::from(w), f64::from(h))
            .map_or(rect(0, 0, w, h), |n| n.body(w, h));
        let mut panel = Self {
            size: (w, h),
            body,
            side: side(cfg.placement.anchor),
            items: vec![],
            pet: None,
            pet_pose: PetPose::Awake,
            battery: None,
            waveform: None,
            wave_phase: 0,
            music: None,
            lyrics: false,
            time: None,
            subtitle: None,
            status,
        };
        if state == IslandState::Compact {
            let media = &messages.media;
            let vertical = side(cfg.placement.anchor);
            let (long, cross) = if vertical {
                (body.height, body.width)
            } else {
                (body.width, body.height)
            };
            if cfg.appearance.minimal && cross < 6 {
                return Some(panel);
            }
            let along = |offset, length, breadth| {
                if vertical {
                    rect(
                        body.x + (cross - breadth) / 2,
                        body.y + offset,
                        breadth,
                        length,
                    )
                } else {
                    rect(
                        body.x + offset,
                        body.y + (cross - breadth) / 2,
                        length,
                        breadth,
                    )
                }
            };
            if cfg.appearance.minimal {
                sources = Vec::new();
            }
            let extra = (cfg.appearance.unread_extent.round() as i32).clamp(0, long);
            let shown = sources.len().min(((extra - 10).max(0) / 22) as usize);
            let usable = long - extra;
            let gap = if vertical { 0 } else { 12 };
            let clock = (if vertical { 32 } else { 44 }).min(usable);
            let mut used = clock;
            let mut reserve = |length: i32, enabled| {
                if enabled && used + gap + length <= usable {
                    used += gap + length;
                    length
                } else {
                    0
                }
            };
            let pet = reserve(32, !vertical && cfg.appearance.pet);
            let battery = reserve(20, cfg.appearance.show_battery && media.battery.is_some());
            let wave = reserve(if vertical { 18 } else { 14 }, !media.title.is_empty());
            let mut cursor = (usable - used) / 2;
            if vertical {
                panel.time = Some((along(cursor, clock, cross), time.into()));
                cursor += clock;
            }
            if battery > 0 {
                let r = along(cursor, 20, if vertical { 14 } else { 20 });
                panel.battery = media.battery.map(|(pct, charging)| (r, pct, charging));
                cursor += battery + gap;
            }
            if pet > 0 {
                panel.pet = Some(along(cursor, pet, 18));
                cursor += pet + gap;
            }
            if !vertical {
                panel.time = Some((along(cursor, clock, cross), time.into()));
                cursor += clock + gap;
            }
            if wave > 0 {
                panel.waveform = Some(along(cursor, wave, 16));
            }
            for (index, source) in sources.into_iter().take(shown).enumerate() {
                let mut instances = windows.iter().filter(|w| w.source == source);
                let Some(entry) = instances.next() else {
                    continue;
                };
                let bounds = along(usable + 5 + index as i32 * 22, 22, 24.min(cross));
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
            panel.battery = media
                .battery
                .filter(|_| cfg.appearance.show_battery)
                .map(|(pct, charging)| (rect(0, 11, 16, 16), pct, charging));
            panel.time = Some((rect(210, 7, 44, 23), time.into()));
            panel.subtitle = Some((rect(128, 7, 74, 23), date.into()));
            if !media.title.is_empty() {
                panel.waveform = Some(rect(360, 10, 14, 16));
            }
            panel.music = Some(media.clone());
            let has = !media.title.is_empty();
            panel.lyrics = has && cfg.appearance.lyrics_enabled;
            panel.status = format!(
                "{date} {time}，{}",
                if has { &media.title } else { "暂无音乐" }
            );
            use MessageAction::*;
            let track = panel.music_track();
            panel.music_control(
                rect(track.x - 4, track.y - 5, track.width + 8, 14),
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
                (74, 26, has.then_some(ToggleLyrics), "歌词", "词"),
                (
                    162,
                    30,
                    (has && media.can_previous).then_some(Previous),
                    "上一首",
                    "⏪",
                ),
                (
                    211,
                    30,
                    (has && media.can_toggle).then_some(TogglePlayback),
                    label,
                    glyph,
                ),
                (
                    258,
                    30,
                    (has && media.can_next).then_some(Next),
                    "下一首",
                    "⏩",
                ),
                (367, 26, None, "音乐应用音量", "\u{e767}"),
            ];
            for (x, width, action, label, glyph) in controls {
                let bounds = rect(x - width / 2, 106, width, 24);
                panel.music_control(bounds, action, label, glyph);
            }
            if has && !panel.side {
                panel.music_control(rect(279, 108, 68, 20), None, "岛上歌词", "岛上歌词");
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
        let mut cfg = IslandConfig::default();
        let count = messages.snapshot.visible_reminders().count();
        cfg.appearance.unread_extent = if count == 0 {
            0.0
        } else {
            10.0 + 22.0 * count as f64
        };
        Panel::build(
            messages,
            &cfg,
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
    fn classic_slots_use_body_coordinates_inside_the_shoulders() {
        let mut cfg = IslandConfig::default();
        let mut model = Messages::default();
        model.media.battery = Some((75, true));
        model.media.title = "音乐".into();
        let make = |cfg: &IslandConfig, w, h| {
            Panel::build(
                &model,
                cfg,
                IslandState::Compact,
                rect(0, 0, w, h),
                1.0,
                "09:41",
                "",
            )
            .unwrap()
        };
        let bar = make(&cfg, 188, 30);
        let cx = |r: Rect| r.x + r.width / 2;
        assert_eq!(bar.body, rect(5, 0, 178, 30));
        assert_eq!(
            [
                cx(bar.battery.unwrap().0),
                cx(bar.pet.unwrap()),
                cx(bar.time.unwrap().0),
                cx(bar.waveform.unwrap())
            ],
            [31, 69, 119, 160]
        );
        cfg.placement.anchor = crate::island::config::Anchor::CenterRight;
        let bar = make(&cfg, 36, 94);
        assert_eq!(bar.body, rect(0, 5, 36, 84));
        assert!(bar.pet.is_none());
        let cy = |r: Rect| r.y + r.height / 2;
        assert_eq!(
            [
                cy(bar.time.unwrap().0),
                cy(bar.battery.unwrap().0),
                cy(bar.waveform.unwrap())
            ],
            [28, 54, 73]
        );
        cfg.appearance.minimal = true;
        let minimal = make(&cfg, 5, 44);
        assert!(minimal.time.is_none() && minimal.battery.is_none() && minimal.waveform.is_none());
    }

    #[test]
    fn seek_uses_the_painted_track_inside_its_larger_hit_area() {
        let p = panel(&Messages::default(), IslandState::Expanded, 454, 140);
        let visible = rect(-100, 0, 454, 140);
        let (x, _, s) = p.music_frame();
        assert_eq!(s, 1.0);
        assert_eq!(p.seek_fraction(visible, visible.x + x as i32 + 92), 0);
        assert_eq!(p.seek_fraction(visible, visible.x + x as i32 + 317), 1000);
    }
    #[test]
    fn lyrics_switch_changes_the_seek_track_and_is_available_only_with_music() {
        let mut cfg = IslandConfig::default();
        let make = |cfg: &IslandConfig, model: &Messages| {
            Panel::build(
                model,
                cfg,
                IslandState::Expanded,
                rect(0, 0, 454, 140),
                1.0,
                "09:41",
                "",
            )
            .unwrap()
        };
        let mut model = Messages::default();
        assert!(!make(&cfg, &model)
            .items
            .iter()
            .any(|i| i.action == Some(MessageAction::ToggleLyrics)));
        model.media.title = "Test".into();
        let page = make(&cfg, &model);
        assert!(
            page.lyrics
                && page
                    .items
                    .iter()
                    .any(|i| i.action == Some(MessageAction::ToggleLyrics))
        );
        assert_eq!(page.music_track().width, 176);
        assert!(page
            .items
            .iter()
            .any(|i| i.name == "岛上歌词" && i.action.is_none()));
        cfg.appearance.lyrics_enabled = false;
        cfg.placement.anchor = crate::island::config::Anchor::CenterLeft;
        let page = make(&cfg, &model);
        assert!(!page.lyrics && page.items.iter().all(|i| i.name != "岛上歌词"));
        assert_eq!(page.music_track().width, 221);
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
        assert!(empty.items.iter().all(|item| item.action.is_none()));
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
