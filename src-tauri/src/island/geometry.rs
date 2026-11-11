use super::config::{Anchor, ExpandDirection, IslandConfig, Orientation};
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

impl Display {
    fn px(&self, dips: f64) -> i32 {
        (dips * self.scale_factor).round() as i32
    }

    fn fit(&self, mut rect: Rect) -> Rect {
        let work = self.work;
        rect.width = rect.width.clamp(1, work.width);
        rect.height = rect.height.clamp(1, work.height);
        rect.x = rect.x.clamp(work.x, work.x + work.width - rect.width);
        rect.y = rect.y.clamp(work.y, work.y + work.height - rect.height);
        rect
    }

    fn shape(&self, cfg: &IslandConfig, expanded: bool) -> Rect {
        let a = &cfg.appearance;
        let (mut w, mut h) = if expanded {
            (a.expanded_width, a.expanded_height)
        } else {
            (a.compact_width, a.compact_height)
        };
        if matches!(cfg.placement.orientation, Orientation::Vertical)
            || (matches!(cfg.placement.orientation, Orientation::Auto)
                && self.work.width < self.work.height)
        {
            std::mem::swap(&mut w, &mut h);
        }
        self.fit(Rect {
            x: self.work.x,
            y: self.work.y,
            width: self.px(w),
            height: self.px(h),
        })
    }

    fn compact(&self, cfg: &IslandConfig) -> Rect {
        use Anchor::*;
        let mut rect = self.shape(cfg, false);
        let anchor = cfg.placement.anchor;
        let w = self.work.width - rect.width;
        let h = self.work.height - rect.height;
        rect.x += match anchor {
            TopRight | CenterRight | BottomRight => w,
            TopCenter | Center | BottomCenter => w / 2,
            _ => 0,
        } + self.px(cfg.placement.offset_x);
        rect.y += match anchor {
            BottomLeft | BottomCenter | BottomRight => h,
            CenterLeft | Center | CenterRight => h / 2,
            _ => 0,
        } + self.px(cfg.placement.offset_y);
        self.fit(rect)
    }

    fn gaps(&self, compact: Rect) -> [(ExpandDirection, i32); 4] {
        use ExpandDirection::*;
        let work = self.work;
        [
            (Down, work.y + work.height - compact.y - compact.height),
            (Up, compact.y - work.y),
            (Right, work.x + work.width - compact.x - compact.width),
            (Left, compact.x - work.x),
        ]
    }

    fn direction(&self, cfg: &IslandConfig, compact: Rect) -> ExpandDirection {
        if cfg.placement.expand_direction != ExpandDirection::Auto {
            return cfg.placement.expand_direction;
        }
        // Compare available fractions so a wide screen still expands down from the top.
        self.gaps(compact)
            .into_iter()
            .max_by(|(a, x), (b, y)| {
                let fraction = |d, v| {
                    let length = if matches!(d, ExpandDirection::Up | ExpandDirection::Down) {
                        self.work.height
                    } else {
                        self.work.width
                    };
                    f64::from(v) / f64::from(length)
                };
                fraction(*a, *x).total_cmp(&fraction(*b, *y))
            })
            .unwrap()
            .0
    }

    fn expansion_offset(compact: Rect, expanded: Rect, d: ExpandDirection) -> (i32, i32) {
        let dx = compact.width - expanded.width;
        let dy = compact.height - expanded.height;
        match d {
            ExpandDirection::Up => (dx / 2, dy),
            ExpandDirection::Left => (dx, dy / 2),
            ExpandDirection::Right => (0, dy / 2),
            _ => (dx / 2, 0),
        }
    }

    pub fn target(&self, cfg: &IslandConfig, state: IslandState) -> Rect {
        let mut compact = self.compact(cfg);
        if state == IslandState::EdgePeek {
            // Clip to a real strip inside the selected work area, including at shared seams.
            let strip = self
                .px(cfg.edge.trigger_width.max(cfg.edge.peek_size))
                .max(1);
            let (edge, _) = self
                .gaps(compact)
                .into_iter()
                .min_by_key(|(_, gap)| *gap)
                .unwrap();
            match edge {
                ExpandDirection::Up | ExpandDirection::Down => {
                    compact.height = strip.min(compact.height);
                    compact.y = self.work.y;
                    if edge == ExpandDirection::Down {
                        compact.y += self.work.height - compact.height;
                    }
                }
                _ => {
                    compact.width = strip.min(compact.width);
                    compact.x = self.work.x;
                    if edge == ExpandDirection::Right {
                        compact.x += self.work.width - compact.width;
                    }
                }
            }
        } else if matches!(state, IslandState::Expanded | IslandState::Pinned) {
            let mut expanded = self.shape(cfg, true);
            let (dx, dy) = Self::expansion_offset(compact, expanded, self.direction(cfg, compact));
            expanded.x = compact.x + dx;
            expanded.y = compact.y + dy;
            return self.fit(expanded);
        }
        compact
    }

    pub fn drag_offset(&self, cfg: &IslandConfig, state: IslandState, at: Rect) -> (f64, f64) {
        use ExpandDirection::*;
        let compact = self.shape(cfg, false);
        let mut custom = cfg.clone();
        custom.placement.anchor = Anchor::Custom;
        let mut best = (i64::MAX, 0.0, 0.0);
        for direction in [
            self.direction(cfg, self.compact(cfg)),
            Up,
            Down,
            Left,
            Right,
        ] {
            let (dx, dy) = if matches!(state, IslandState::Expanded | IslandState::Pinned) {
                Self::expansion_offset(compact, self.shape(cfg, true), direction)
            } else {
                (0, 0)
            };
            let x = f64::from(at.x - dx - self.work.x) / self.scale_factor;
            let y = f64::from(at.y - dy - self.work.y) / self.scale_factor;
            custom.placement.offset_x = x;
            custom.placement.offset_y = y;
            let actual = self.target(&custom, state);
            let error = i64::from((at.x - actual.x).abs()) + i64::from((at.y - actual.y).abs());
            if error < best.0 {
                best = (error, x, y);
            }
            if error == 0 {
                break;
            }
        }
        (best.1, best.2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_coordinates_use_local_scale_and_peek_stays_inside_work_area() {
        let display = Display {
            id: "left".into(),
            name: "left".into(),
            primary: false,
            scale_factor: 1.5,
            work: Rect {
                x: -1920,
                y: -400,
                width: 1880,
                height: 1080,
            },
        };
        let mut cfg = IslandConfig::default();
        cfg.placement.anchor = Anchor::Custom;
        cfg.placement.offset_x = 8.0;
        cfg.placement.offset_y = 120.0;
        let compact = display.target(&cfg, IslandState::Compact);
        assert_eq!((compact.x, compact.y), (-1908, -220));
        let peek = display.target(&cfg, IslandState::EdgePeek);
        assert_eq!(peek.x, -1920);
        assert_eq!(
            peek.width,
            display.px(cfg.edge.trigger_width.max(cfg.edge.peek_size))
        );
        let expanded = display.target(&cfg, IslandState::Expanded);
        let (x, y) = display.drag_offset(&cfg, IslandState::Expanded, expanded);
        cfg.placement.offset_x = x;
        cfg.placement.offset_y = y;
        assert_eq!(display.target(&cfg, IslandState::Expanded), expanded);
    }
}
