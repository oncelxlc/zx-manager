use super::config::{Anchor, IslandConfig, IslandShape, PlacementConfig};
use super::state::IslandState;
use serde::Serialize;

/// Window rectangles are physical pixels in virtual desktop coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn interpolate(self, end: Self, progress: f64) -> Self {
        let mix = |a, b| (f64::from(a) + (f64::from(b) - f64::from(a)) * progress).round() as i32;
        Self {
            x: mix(self.x, end.x),
            y: mix(self.y, end.y),
            width: mix(self.width, end.width).max(1),
            height: mix(self.height, end.height).max(1),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Display {
    pub id: String,
    pub name: String,
    pub primary: bool,
    pub scale_factor: f64,
    #[serde(skip)]
    pub work: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Edge {
    Top,
    Left,
    Right,
}

impl Edge {
    fn from_anchor(anchor: Anchor) -> Self {
        match anchor {
            Anchor::CenterLeft => Self::Left,
            Anchor::CenterRight => Self::Right,
            _ => Self::Top,
        }
    }

    fn anchor(self) -> Anchor {
        match self {
            Self::Left => Anchor::CenterLeft,
            Self::Right => Anchor::CenterRight,
            Self::Top => Anchor::TopCenter,
        }
    }

    fn gaps(x: i32, y: i32, work: Rect) -> [(Self, i32); 3] {
        [
            (Self::Top, (y - work.y).max(0)),
            (Self::Left, (x - work.x).max(0)),
            (Self::Right, (work.x + work.width - x).max(0)),
        ]
    }

    fn dragged(self, work: Rect, cursor: (i32, i32), scale: f64) -> Self {
        let gaps = Self::gaps(cursor.0, cursor.1, work);
        let gap = |edge| f64::from(gaps.iter().find(|pair| pair.0 == edge).unwrap().1);
        let (nearest, distance) = *gaps.iter().min_by_key(|pair| pair.1).unwrap();
        if f64::from(distance) <= 64.0 * scale && f64::from(distance) + 24.0 * scale < gap(self) {
            return nearest;
        }
        let opposite = match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Top => return self,
        };
        if gap(opposite) + f64::from(work.width) * 0.07 < gap(self) {
            opposite
        } else {
            self
        }
    }
}

impl Rect {
    pub(super) fn placement_bounds(self, work_top: i32) -> Self {
        let reserved = work_top - self.y;
        if reserved > 0 && reserved <= self.height / 3 {
            Self {
                y: work_top,
                height: self.height - reserved,
                ..self
            }
        } else {
            self
        }
    }

    fn fit(mut self, work: Rect) -> Self {
        self.width = self.width.clamp(1, work.width);
        self.height = self.height.clamp(1, work.height);
        self.x = self.x.clamp(work.x, work.x + work.width - self.width);
        self.y = self.y.clamp(work.y, work.y + work.height - self.height);
        self
    }

    pub(super) fn dock_anchor(self, work: Rect, preferred: Anchor) -> Anchor {
        let r = self.fit(work);
        [
            (Edge::Top, r.y - work.y),
            (Edge::Left, r.x - work.x),
            (Edge::Right, work.x + work.width - r.x - r.width),
        ]
        .into_iter()
        .min_by_key(|(edge, d)| (*d, *edge != Edge::from_anchor(preferred)))
        .unwrap()
        .0
        .anchor()
    }
}

impl PlacementConfig {
    pub(super) fn lock_to_anchor(&mut self) -> bool {
        use Anchor::*;
        match self.anchor {
            Custom => return false,
            CenterLeft | CenterRight => {
                self.offset_x = 0.0;
                self.offset_y = self.offset_y.clamp(-100.0, 100.0).round();
            }
            _ => {
                self.offset_x = match self.anchor {
                    TopLeft | BottomLeft => -100.0,
                    TopRight | BottomRight => 100.0,
                    _ => self.offset_x.clamp(-100.0, 100.0).round(),
                };
                self.anchor = TopCenter;
                self.offset_y = 0.0;
            }
        }
        true
    }
}

impl Display {
    pub fn scale(&self, cfg: &IslandConfig) -> f64 {
        self.scale_factor * f64::from(cfg.appearance.island_size) / 100.0
    }

    pub fn content_config(
        &self,
        cfg: &IslandConfig,
        media: &super::media::MediaSnapshot,
        reminders: usize,
    ) -> IslandConfig {
        let mut layout = cfg.clone();
        let a = &mut layout.appearance;
        let side = Edge::from_anchor(cfg.placement.anchor) != Edge::Top;
        let reminders = if cfg.notifications.enabled {
            reminders
        } else {
            0
        };
        let battery = a.show_battery && media.battery.is_some();
        let music = !media.title.is_empty();
        a.unread_extent = if reminders == 0 || a.minimal {
            0.0
        } else {
            32.0 + (reminders - 1) as f64 * 22.0
        };
        (a.compact_width, a.compact_height) = if a.minimal && (!side || reminders == 0) {
            if side {
                (4.5, 44.0)
            } else {
                (44.0, 4.5)
            }
        } else if side {
            let length = 46 + if battery { 20 } else { 0 } + if music { 18 } else { 0 };
            (36.0, f64::from(length) + a.unread_extent)
        } else {
            let body = 76
                + if a.pet { 44 } else { 0 }
                + if battery { 32 } else { 0 }
                + if music { 26 } else { 0 };
            (f64::from(body.max(96)) + a.unread_extent, 30.0)
        };
        a.radius = if a.shape == IslandShape::Pill {
            26.0
        } else {
            33.0
        };
        layout
    }

    fn travel(&self, edge: Edge, scale: f64) -> f64 {
        let (width, reserve) = if edge == Edge::Top {
            (self.work.width, 240.0)
        } else {
            (self.work.height, 150.0)
        };
        (f64::from(width) / 2.0 - reserve * scale).max(0.0)
    }

    pub fn target(&self, cfg: &IslandConfig, state: IslandState) -> Rect {
        let edge = Edge::from_anchor(cfg.placement.anchor);
        let a = &cfg.appearance;
        let expanded = matches!(state, IslandState::Expanded | IslandState::Pinned);
        let peek = state == IslandState::EdgePeek;
        let (mut w, mut h) = if expanded {
            (a.expanded_width, a.expanded_height)
        } else {
            (a.compact_width, a.compact_height)
        };
        let scale = self.scale(cfg);
        let notch = a.shape == IslandShape::Notch;
        let shoulder = if !notch || peek || (!expanded && w.min(h) <= 4.5) {
            0.0
        } else if expanded {
            17.6
        } else {
            5.0
        };
        if peek {
            (w, h) = if edge == Edge::Top {
                (44.0, 4.5)
            } else {
                (4.5, 44.0)
            };
        }
        if edge == Edge::Top {
            w += 2.0 * shoulder;
        } else {
            h += 2.0 * shoulder;
        }
        let gap = if notch { 0.0 } else { 6.0 * scale };
        let work = self.work;
        let (width, height) = ((w * scale).round() as i32, (h * scale).round() as i32);
        let ratio = if edge == Edge::Top {
            cfg.placement.offset_x
        } else {
            cfg.placement.offset_y
        };
        let unread = if expanded || peek {
            0.0
        } else {
            a.unread_extent * scale
        };
        let shift = self.travel(edge, scale) * ratio.clamp(-100.0, 100.0) / 100.0 + unread / 2.0;
        let (x, y) = if edge == Edge::Top {
            (
                work.x + (f64::from(work.width - width) / 2.0 + shift).round() as i32,
                work.y + gap.round() as i32,
            )
        } else {
            (
                if edge == Edge::Left {
                    work.x + gap.round() as i32
                } else {
                    work.x + work.width - width - gap.round() as i32
                },
                work.y + (f64::from(work.height - height) / 2.0 + shift).round() as i32,
            )
        };
        Rect {
            x,
            y,
            width,
            height,
        }
        .fit(work)
    }

    pub(super) fn drag_placement(
        &self,
        cfg: &IslandConfig,
        state: IslandState,
        at: Rect,
    ) -> PlacementConfig {
        let edge = Edge::from_anchor(cfg.placement.anchor);
        let mut p = cfg.placement.clone();
        p.display_id = Some(self.id.clone());
        p.anchor = edge.anchor();
        let (center, mid) = if edge == Edge::Top {
            (
                at.x as f64 + f64::from(at.width) / 2.0,
                self.work.x as f64 + f64::from(self.work.width) / 2.0,
            )
        } else {
            (
                at.y as f64 + f64::from(at.height) / 2.0,
                self.work.y as f64 + f64::from(self.work.height) / 2.0,
            )
        };
        let unread = if matches!(state, IslandState::Expanded | IslandState::Pinned) {
            0.0
        } else {
            cfg.appearance.unread_extent
        };
        let travel = self.travel(edge, self.scale(cfg));
        let offset = if travel <= 0.0 {
            0.0
        } else {
            ((center - mid - unread * self.scale(cfg) / 2.0) * 100.0 / travel)
                .round()
                .clamp(-100.0, 100.0)
        };
        (p.offset_x, p.offset_y) = if edge == Edge::Top {
            (offset, 0.0)
        } else {
            (0.0, offset)
        };
        p
    }

    pub(super) fn drag_layout(
        &self,
        cfg: &IslandConfig,
        state: IslandState,
        at: Rect,
        cursor: (i32, i32),
        media: &super::media::MediaSnapshot,
        reminders: usize,
    ) -> (IslandConfig, Rect) {
        let old = Edge::from_anchor(cfg.placement.anchor);
        let edge = old.dragged(self.work, cursor, self.scale(cfg));
        let mut placed = cfg.clone();
        placed.placement.anchor = edge.anchor();
        let mut sized = self.content_config(&placed, media, reminders);
        let mut center = at;
        if edge != old {
            center.x = cursor.0 - at.width / 2;
            center.y = cursor.1 - at.height / 2;
        }
        sized.placement = self.drag_placement(&sized, state, center);
        let (value, original) = if edge == Edge::Top {
            (&mut sized.placement.offset_x, cfg.placement.offset_x)
        } else {
            (&mut sized.placement.offset_y, cfg.placement.offset_y)
        };
        if value.abs() <= 5.0 {
            *value = 0.0;
        } else if old == edge && (*value - original).abs() <= 5.0 {
            *value = original;
        }
        let target = self.target(&sized, state);
        (sized, target)
    }

    pub(super) fn migrate_placement(
        &self,
        cfg: &mut IslandConfig,
        width: f64,
        height: f64,
    ) -> Result<(), String> {
        use Anchor::*;
        let p = &cfg.placement;
        let (dx, dy) = (p.offset_x, p.offset_y);
        if !(-65535.0..=65535.0).contains(&dx) || !(-65535.0..=65535.0).contains(&dy) {
            return Err("旧版窗口坐标超出有效范围".into());
        }
        let side = matches!(p.anchor, CenterLeft | CenterRight);
        let (width, height) = if side {
            (height, width)
        } else {
            (width, height)
        };
        let scale = self.scale_factor;
        let w = (width * scale)
            .round()
            .clamp(1.0, f64::from(self.work.width)) as i32;
        let h = (height * scale)
            .round()
            .clamp(1.0, f64::from(self.work.height)) as i32;
        let (free_x, free_y) = (self.work.width - w, self.work.height - h);
        let x = match p.anchor {
            TopRight | CenterRight | BottomRight => free_x,
            TopCenter | Center | BottomCenter => free_x / 2,
            _ => 0,
        };
        let y = match p.anchor {
            BottomLeft | BottomCenter | BottomRight => free_y,
            CenterLeft | Center | CenterRight => free_y / 2,
            _ => 0,
        };
        let at = Rect {
            x: self.work.x + x + (dx * scale).round() as i32,
            y: self.work.y + y + (dy * scale).round() as i32,
            width: w,
            height: h,
        }
        .fit(self.work);
        cfg.placement.anchor = match p.anchor {
            TopLeft | TopCenter | TopRight => TopCenter,
            CenterLeft | CenterRight => p.anchor,
            _ => at.dock_anchor(self.work, TopCenter),
        };
        let id = cfg.placement.display_id.clone();
        cfg.placement = self.drag_placement(cfg, IslandState::Compact, at);
        cfg.placement.display_id = id;
        Ok(())
    }

    pub(super) fn normalize_placement(&self, cfg: &mut IslandConfig) {
        if cfg.placement.lock_to_anchor() {
            return;
        }
        // set_position accepts the top-left corner in monitor-local desktop DIPs.
        let mut at = self.target(cfg, IslandState::Compact);
        at.x = self.work.x + (cfg.placement.offset_x * self.scale_factor).round() as i32;
        at.y = self.work.y + (cfg.placement.offset_y * self.scale_factor).round() as i32;
        let id = cfg.placement.display_id.clone();
        cfg.placement.anchor = at.dock_anchor(self.work, Anchor::TopCenter);
        cfg.placement = self.drag_placement(cfg, IslandState::Compact, at);
        cfg.placement.display_id = id;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::island::media::MediaSnapshot;

    fn screen() -> Display {
        Display {
            id: "left".into(),
            name: "left".into(),
            primary: true,
            scale_factor: 1.0,
            work: Rect {
                x: -1920,
                y: -400,
                width: 1920,
                height: 1080,
            },
        }
    }

    #[test]
    fn top_slots_shoulders_and_asymmetric_unread_match_the_reference() {
        let d = screen();
        let mut cfg = IslandConfig::default();
        let mut media = MediaSnapshot::default();
        let body = |cfg: &IslandConfig, media: &MediaSnapshot, n| {
            let layout = d.content_config(cfg, media, n);
            (
                layout.appearance.compact_width,
                d.target(&layout, IslandState::Compact),
            )
        };
        let (width, quiet) = body(&cfg, &media, 0);
        assert_eq!((width, quiet.width, quiet.height), (120.0, 130, 30));
        media.battery = Some((78, true));
        assert_eq!(body(&cfg, &media, 0).0, 152.0);
        media.title = "Song".into();
        assert_eq!(body(&cfg, &media, 0).0, 178.0);
        cfg.appearance.show_battery = false;
        assert_eq!(body(&cfg, &media, 0).0, 146.0);
        media.title.clear();
        cfg.appearance.pet = false;
        assert_eq!(body(&cfg, &media, 0).0, 96.0);
        for (n, extra) in [(1, 32), (2, 54)] {
            let (_, base) = body(&cfg, &media, 0);
            let (_, notice) = body(&cfg, &media, n);
            assert_eq!((notice.x, notice.width), (base.x, base.width + extra));
        }
        cfg.notifications.enabled = false;
        assert_eq!(body(&cfg, &media, 2), body(&cfg, &media, 0));
    }

    #[test]
    fn side_bar_has_two_line_clock_and_keeps_expanded_page_horizontal() {
        let mut d = screen();
        d.scale_factor = 1.25;
        let mut cfg = IslandConfig::default();
        cfg.appearance.island_size = 120;
        let media = MediaSnapshot {
            title: "Track".into(),
            battery: Some((78, false)),
            ..Default::default()
        };
        for dock in [Anchor::CenterLeft, Anchor::CenterRight] {
            cfg.placement.anchor = dock;
            let sized = d.content_config(&cfg, &media, 0);
            assert_eq!(
                (
                    sized.appearance.compact_width,
                    sized.appearance.compact_height
                ),
                (36.0, 84.0)
            );
            let c = d.target(&sized, IslandState::Compact);
            let e = d.target(&sized, IslandState::Expanded);
            assert_eq!((c.width, c.height), (54, 141));
            assert_eq!((e.width, e.height), (629, 263));
            assert_eq!((c.y * 2 + c.height - e.y * 2 - e.height).abs(), 0);
            let expected_x = if dock == Anchor::CenterLeft {
                d.work.x
            } else {
                d.work.x + d.work.width - c.width
            };
            assert_eq!(c.x, expected_x);
            let unread = d.content_config(&cfg, &media, 2);
            let extra = d.target(&unread, IslandState::Compact);
            assert_eq!((extra.y, extra.height), (c.y, c.height + 81));
        }
    }

    #[test]
    fn minimal_and_pill_follow_the_dock_and_omit_shoulders() {
        let d = screen();
        let mut cfg = IslandConfig::default();
        cfg.appearance.minimal = true;
        for shape in [IslandShape::Notch, IslandShape::Pill] {
            cfg.appearance.shape = shape;
            let gap = if shape == IslandShape::Pill { 6 } else { 0 };
            for anchor in [Anchor::TopCenter, Anchor::CenterLeft, Anchor::CenterRight] {
                cfg.placement.anchor = anchor;
                let layout = d.content_config(&cfg, &MediaSnapshot::default(), 0);
                for state in [IslandState::Compact, IslandState::EdgePeek] {
                    let r = d.target(&layout, state);
                    match anchor {
                        Anchor::TopCenter => {
                            assert_eq!((r.width, r.height, r.y), (44, 5, d.work.y + gap))
                        }
                        Anchor::CenterLeft => {
                            assert_eq!((r.width, r.height, r.x), (5, 44, d.work.x + gap))
                        }
                        _ => assert_eq!(
                            (r.width, r.height, r.x + r.width),
                            (5, 44, d.work.x + d.work.width - gap)
                        ),
                    }
                }
            }
        }
        cfg.placement.anchor = Anchor::TopCenter;
        assert_eq!(
            d.content_config(&cfg, &MediaSnapshot::default(), 1)
                .appearance
                .compact_width,
            44.0
        );
        cfg.placement.anchor = Anchor::CenterLeft;
        assert_eq!(
            d.content_config(&cfg, &MediaSnapshot::default(), 1)
                .appearance
                .compact_width,
            36.0
        );
    }

    #[test]
    fn physical_coordinates_round_trip_as_percentages_on_three_edges() {
        let d = screen();
        let mut cfg = IslandConfig::default();
        cfg.appearance.island_size = 125;
        for anchor in [Anchor::TopCenter, Anchor::CenterLeft, Anchor::CenterRight] {
            cfg.placement.anchor = anchor;
            for offset in [-100.0, -56.0, 0.0, 32.0, 100.0] {
                (cfg.placement.offset_x, cfg.placement.offset_y) = if anchor == Anchor::TopCenter {
                    (offset, 0.0)
                } else {
                    (0.0, offset)
                };
                let sized = d.content_config(&cfg, &MediaSnapshot::default(), 2);
                for state in [
                    IslandState::Compact,
                    IslandState::Expanded,
                    IslandState::Pinned,
                ] {
                    let r = d.target(&sized, state);
                    let p = d.drag_placement(&sized, state, r);
                    assert_eq!(
                        (p.anchor, p.offset_x, p.offset_y),
                        (anchor, cfg.placement.offset_x, cfg.placement.offset_y)
                    );
                    assert_eq!(p.display_id.as_deref(), Some("left"));
                }
            }
        }
    }

    #[test]
    fn drag_switches_with_hysteresis_and_snap_uses_new_dock_slots() {
        let d = screen();
        let cfg = IslandConfig::default();
        let media = MediaSnapshot::default();
        let top = d.target(&cfg, IslandState::Compact);
        let away = (d.work.x + 130, d.work.y + d.work.height / 2);
        let (same, _) = d.drag_layout(&cfg, IslandState::Compact, top, away, &media, 0);
        assert_eq!(same.placement.anchor, Anchor::TopCenter);
        let center = d.work.y + d.work.height / 2;
        let (left, r) = d.drag_layout(
            &cfg,
            IslandState::Compact,
            top,
            (d.work.x + 10, center),
            &media,
            0,
        );
        assert_eq!(
            (left.placement.anchor, r.width, r.height),
            (Anchor::CenterLeft, 36, 56)
        );
        assert_eq!(r.y + r.height / 2, center);
        let (right, r) = d.drag_layout(
            &left,
            IslandState::Compact,
            r,
            (-400, center + 200),
            &media,
            0,
        );
        assert_eq!(right.placement.anchor, Anchor::CenterRight);
        assert!((r.y + r.height / 2 - center - 200).abs() <= 2);
    }

    #[test]
    fn explicit_position_dips_are_converted_using_monitor_dpi() {
        let mut d = screen();
        d.scale_factor = 1.5;
        let mut cfg = IslandConfig::default();
        cfg.placement.anchor = Anchor::Custom;
        cfg.placement.offset_x = 0.0;
        cfg.placement.offset_y = 360.0;
        d.normalize_placement(&mut cfg);
        assert_eq!(cfg.placement.anchor, Anchor::CenterLeft);
        assert_eq!(cfg.placement.display_id, None);
        let bounds = Rect {
            x: 0,
            y: 0,
            width: 1280,
            height: 720,
        };
        assert_eq!(bounds.placement_bounds(36).height, 684);
        assert_eq!(bounds.placement_bounds(400), bounds);
    }
}
