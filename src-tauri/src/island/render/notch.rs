use crate::island::state::IslandState;

/// Fractions of the target texture. The compositor scales this same outline
/// to the current animated window size; input uses those coordinates too.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Notch {
    shoulder: f64,
    shoulder_height: f64,
    radius_x: f64,
    radius_y: f64,
}

impl Notch {
    pub fn new(state: IslandState, width: f64, height: f64, radius: f64) -> Self {
        let (wing, depth, radius) = if state == IslandState::Compact {
            (5.0_f64, 8.0_f64, radius * 12.5 / 33.0)
        } else {
            (17.6, 16.0, radius)
        };
        let wing = wing.min(width / 4.0);
        let radius = radius.min(height / 2.0).min(width / 2.0 - wing);
        Self {
            shoulder: wing / width,
            shoulder_height: (depth / height).min(0.5),
            radius_x: radius / width,
            radius_y: radius / height,
        }
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

    pub fn contains(self, x: f64, y: f64) -> bool {
        let inset = self.inset(y);
        (0.0..1.0).contains(&y) && x >= inset && x < 1.0 - inset
    }

    pub fn outline<T>(self, width: f64, height: f64, point: impl Fn(f64, f64) -> T) -> Vec<T> {
        let rows = (0..=14)
            .map(|i| self.shoulder_height * f64::from(i) / 14.0)
            .chain((0..=20).map(|i| {
                let angle = f64::from(i) / 20.0 * std::f64::consts::FRAC_PI_2;
                1.0 - self.radius_y * (1.0 - angle.sin())
            }))
            .map(|y| (self.inset(y) * width, y * height))
            .collect::<Vec<_>>();
        rows.iter()
            .map(|&(x, y)| point(width - x, y))
            .chain(rows.iter().rev().map(|&(x, y)| point(x, y)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classic_shoulders_and_bottom_corners_scale_with_the_compositor() {
        for (state, w, h, wing, depth, radius) in [
            (IslandState::Compact, 163.0, 30.0, 5.0, 8.0, 12.5),
            (IslandState::Expanded, 419.0, 140.0, 17.6, 16.0, 33.0),
        ] {
            let notch = Notch::new(state, w, h, 33.0);
            assert_eq!(notch.inset(0.0), 0.0);
            assert!((notch.inset(depth / h) * w - wing).abs() < 0.001);
            assert!((notch.inset(1.0) * w - wing - radius).abs() < 0.001);
            assert!(!notch.contains((wing - 1.0) / w, depth / h));
            assert!(notch.contains((wing + 1.0) / w, depth / h));
            let points = notch.outline(w * 2.0, h * 1.5, |x, y| (x, y));
            assert_eq!(points[0], (w * 2.0, 0.0));
            assert_eq!(points.last(), Some(&(0.0, 0.0)));
        }
    }
}
