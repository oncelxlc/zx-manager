use crate::island::{
    config::{Anchor, IslandConfig, IslandShape},
    state::IslandState,
};

pub(super) fn side(anchor: Anchor) -> bool {
    matches!(anchor, Anchor::CenterLeft | Anchor::CenterRight)
}

/// Fractions of the target texture. The compositor scales this same outline
/// to the current animated window size; input uses those coordinates too.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Notch {
    anchor: Anchor,
    shoulder: f64,
    shoulder_height: f64,
    radius_x: f64,
    radius_y: f64,
}

impl Notch {
    pub fn for_config(
        cfg: &IslandConfig,
        state: IslandState,
        width: f64,
        height: f64,
    ) -> Option<Self> {
        let anchor = cfg.placement.anchor;
        let (width, height) = if side(anchor) {
            (height, width)
        } else {
            (width, height)
        };
        if cfg.appearance.shape == IslandShape::Pill {
            return None;
        }
        let radius = cfg.appearance.radius;
        let (wing, depth, radius) = if state == IslandState::EdgePeek
            || state == IslandState::Compact && cfg.appearance.minimal && height < 6.0
        {
            (0.0, 2.25, 2.25)
        } else if state == IslandState::Compact {
            (5.0_f64, 8.0_f64, radius * 12.5 / 33.0)
        } else {
            (17.6, 16.0, radius)
        };
        let wing = wing.min(width / 4.0);
        let radius = radius.min(height / 2.0).min(width / 2.0 - wing);
        Some(Self {
            anchor,
            shoulder: wing / width,
            shoulder_height: (depth / height).min(0.5),
            radius_x: radius / width,
            radius_y: radius / height,
        })
    }

    fn inset(self, y: f64) -> f64 {
        if y < self.shoulder_height {
            self.shoulder * (1.0 - (1.0 - y / self.shoulder_height).powf(2.4))
        } else if y > 1.0 - self.radius_y {
            let t = (y - (1.0 - self.radius_y)) / self.radius_y;
            self.shoulder + self.radius_x * (1.0 - (1.0 - t * t).max(0.0).sqrt())
        } else {
            self.shoulder
        }
    }

    pub(super) fn body(self, width: i32, height: i32) -> crate::island::geometry::Rect {
        let side = side(self.anchor);
        let along = if side { height } else { width };
        let wing = self.shoulder * f64::from(along);
        let start = wing.round() as i32;
        let len = (f64::from(along) - 2.0 * wing).round().max(1.0) as i32;
        crate::island::geometry::Rect {
            x: if side { 0 } else { start },
            y: if side { start } else { 0 },
            width: if side { width } else { len },
            height: if side { len } else { height },
        }
    }

    // A physical pixel belongs to the mask if any part of its unit square touches
    // the filled contour. The nearest edge of a scanline has the smallest inset.
    fn pixel_inset(self, along: i32, depth: i32, row: i32) -> i32 {
        (self.inset(f64::from(row) / f64::from(depth)) * f64::from(along)).floor() as i32
    }

    pub fn contains_pixel(self, width: i32, height: i32, x: i32, y: i32) -> bool {
        if !(0..width).contains(&x) || !(0..height).contains(&y) {
            return false;
        }
        let (along, depth, col, row) = match self.anchor {
            Anchor::CenterLeft => (height, width, y, x),
            Anchor::CenterRight => (height, width, y, width - 1 - x),
            _ => (width, height, x, y),
        };
        let inset = self.pixel_inset(along, depth, row);
        (inset..along - inset).contains(&col)
    }

    pub fn pixel_outline<T>(
        self,
        width: i32,
        height: i32,
        point: impl Fn(i32, i32) -> T,
    ) -> Vec<T> {
        let (along, depth) = if side(self.anchor) {
            (height, width)
        } else {
            (width, height)
        };
        let mut edge = vec![(0, 0)];
        let mut prev = 0;
        for row in 1..depth {
            let inset = self.pixel_inset(along, depth, row);
            if inset != prev {
                edge.push((prev, row));
                edge.push((inset, row));
                prev = inset;
            }
        }
        edge.push((prev, depth));
        let map = |x, y| match self.anchor {
            Anchor::CenterLeft => point(y, x),
            Anchor::CenterRight => point(width - y, x),
            _ => point(x, y),
        };
        edge.iter()
            .map(|&(x, y)| map(along - x, y))
            .chain(edge.iter().rev().map(|&(x, y)| map(x, y)))
            .collect()
    }

    pub fn outline<T>(self, width: f64, height: f64, point: impl Fn(f64, f64) -> T) -> Vec<T> {
        let map = |x, y| match self.anchor {
            Anchor::CenterLeft => point(y * width, x * height),
            Anchor::CenterRight => point((1.0 - y) * width, x * height),
            _ => point(x * width, y * height),
        };
        let rows = (0..=14)
            .map(|i| self.shoulder_height * f64::from(i) / 14.0)
            .chain((0..=20).map(|i| {
                let angle = f64::from(i) / 20.0 * std::f64::consts::FRAC_PI_2;
                1.0 - self.radius_y * (1.0 - angle.sin())
            }))
            .map(|y| (self.inset(y), y))
            .collect::<Vec<_>>();
        rows.iter()
            .map(|&(x, y)| map(1.0 - x, y))
            .chain(rows.iter().rev().map(|&(x, y)| map(x, y)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::{
        Foundation::{POINT, RECT},
        Graphics::Gdi::{CreatePolygonRgn, DeleteObject, GetRgnBox, PtInRegion, WINDING},
    };

    #[test]
    fn gdi_mask_keeps_both_shoulder_tips_and_matches_each_hit_pixel() {
        for anchor in [Anchor::TopCenter, Anchor::CenterLeft, Anchor::CenterRight] {
            let mut cfg = IslandConfig::default();
            cfg.placement.anchor = anchor;
            let (w, h) = if side(anchor) {
                (419.0, 175.2)
            } else {
                (454.2, 140.0)
            };
            let notch = Notch::for_config(&cfg, IslandState::Expanded, w, h).unwrap();
            for scale in [1.0, 1.25] {
                let (w, h) = ((w * scale).round() as i32, (h * scale).round() as i32);
                let p = notch.pixel_outline(w, h, |x, y| POINT { x, y });
                unsafe {
                    let mask = CreatePolygonRgn(p.as_ptr(), p.len() as i32, WINDING);
                    assert!(!mask.is_null());
                    let mut r = RECT::default();
                    let kind = GetRgnBox(mask, &mut r);
                    let mismatch = (-1..=h).find_map(|y| {
                        (-1..=w)
                            .find(|&x| {
                                (PtInRegion(mask, x, y) != 0) != notch.contains_pixel(w, h, x, y)
                            })
                            .map(|x| (x, y))
                    });
                    DeleteObject(mask);
                    assert!(kind > 0);
                    assert_eq!(
                        (r.left, r.top, r.right, r.bottom),
                        (0, 0, w, h),
                        "{anchor:?} {scale}"
                    );
                    assert_eq!(mismatch, None, "{anchor:?} {scale}");
                }
            }
        }
    }

    #[test]
    fn classic_shoulders_and_bottom_corners_scale_with_the_compositor() {
        for (state, w, h, wing, depth, radius) in [
            (IslandState::Compact, 163.0, 30.0, 5.0, 8.0, 12.5),
            (IslandState::Expanded, 419.0, 140.0, 17.6, 16.0, 33.0),
        ] {
            let notch = Notch::for_config(&IslandConfig::default(), state, w, h).unwrap();
            assert_eq!(notch.inset(0.0), 0.0);
            assert!((notch.inset(depth / h) * w - wing).abs() < 0.001);
            assert!((notch.inset(1.0) * w - wing - radius).abs() < 0.001);
            assert!(!notch.contains_pixel(
                w as i32,
                h as i32,
                wing.floor() as i32 - 1,
                depth as i32
            ));
            assert!(notch.contains_pixel(w as i32, h as i32, wing.ceil() as i32, depth as i32));
            let points = notch.outline(w * 2.0, h * 1.5, |x, y| (x, y));
            assert_eq!(points[0], (w * 2.0, 0.0));
            assert_eq!(points.last(), Some(&(0.0, 0.0)));
        }
    }

    #[test]
    fn side_shoulders_follow_the_screen_edge_and_minimal_keeps_one_flat_side() {
        let mut cfg = IslandConfig::default();
        for anchor in [Anchor::CenterLeft, Anchor::CenterRight] {
            cfg.placement.anchor = anchor;
            let notch = Notch::for_config(&cfg, IslandState::Expanded, 419.0, 175.2).unwrap();
            let edge = if anchor == Anchor::CenterLeft {
                0.001
            } else {
                0.999
            };
            let inner = 1.0 - edge;
            assert!(notch.contains_pixel(419, 175, (edge * 419.0) as i32, 3));
            assert!(!notch.contains_pixel(419, 175, (inner * 419.0) as i32, 3));
            cfg.appearance.minimal = true;
            let line = Notch::for_config(&cfg, IslandState::Compact, 4.5, 44.0).unwrap();
            assert_eq!(line.shoulder, 0.0);
            assert!(line.contains_pixel(45, 440, (edge * 45.0) as i32, 2));
            assert!(!line.contains_pixel(45, 440, (inner * 45.0) as i32, 2));
            cfg.appearance.minimal = false;
        }
    }
}
