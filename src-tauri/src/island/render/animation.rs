use crate::island::{
    config::{AnimationConfig, AnimationPreset},
    geometry::Rect,
};
use windows::core::Result;
use windows::Win32::Graphics::DirectComposition::{IDCompositionAnimation, IDCompositionDevice};
use windows_sys::Win32::System::Performance::QueryPerformanceCounter;

pub(super) fn now() -> i64 {
    let mut ticks = 0;
    // QPC is available on every supported Windows version and is DComp's time base.
    unsafe { QueryPerformanceCounter(&mut ticks) };
    ticks
}

#[derive(Clone, Copy)]
pub(super) struct Timing {
    start: i64,
    frequency: i64,
    seconds: f64,
    coefficients: [f64; 3],
}

impl Timing {
    pub fn new(start: i64, frequency: i64, ms: u32, animation: &AnimationConfig) -> Self {
        let coefficients = match animation.preset {
            AnimationPreset::Subtle => [2.0, -1.0, 0.0],
            AnimationPreset::Smooth => [0.0, 3.0, -2.0],
            AnimationPreset::Custom => animation.custom_coefficients(),
            AnimationPreset::Spring => {
                // Front-load movement and settle at the edge without leaving its hit area.
                let slope = 2.0 + animation.amplitude.clamp(0.0, 1.0);
                [slope, 3.0 - 2.0 * slope, slope - 2.0]
            }
        };
        Self {
            start,
            frequency,
            seconds: f64::from(ms) / 1000.0,
            coefficients,
        }
    }

    pub fn progress(self, at: i64) -> f64 {
        if self.seconds == 0.0 {
            return 1.0;
        }
        let t = ((at - self.start) as f64 / self.frequency as f64 / self.seconds).clamp(0.0, 1.0);
        let [a, b, c] = self.coefficients;
        t * (a + t * (b + t * c))
    }

    pub unsafe fn property(
        self,
        device: &IDCompositionDevice,
        from: f32,
        to: f32,
        fixed: impl FnOnce(f32) -> Result<()>,
        animated: impl FnOnce(&IDCompositionAnimation) -> Result<()>,
    ) -> Result<()> {
        if self.seconds == 0.0 || from == to {
            return fixed(to);
        }
        let delta = f64::from(to - from);
        let [a, b, c] = self.coefficients;
        let animation = device.CreateAnimation()?;
        animation.SetAbsoluteBeginTime(self.start)?;
        animation.AddCubic(
            0.0,
            from,
            (delta * a / self.seconds) as f32,
            (delta * b / self.seconds.powi(2)) as f32,
            (delta * c / self.seconds.powi(3)) as f32,
        )?;
        animation.End(self.seconds, to)?;
        animated(&animation)
    }
}

#[derive(Clone, Copy)]
pub(super) struct Movement {
    pub from: Rect,
    pub to: Rect,
    pub timing: Timing,
    pub radii: [f64; 2],
}

impl Movement {
    pub fn radius_at(self, ticks: i64) -> f64 {
        self.radii[0] + (self.radii[1] - self.radii[0]) * self.timing.progress(ticks)
    }

    pub fn remaining_ms(self, target: Rect, at: i64, duration: u32) -> u32 {
        let distance = |r: Rect| {
            [
                r.x.abs_diff(target.x),
                r.y.abs_diff(target.y),
                r.width.abs_diff(target.width),
                r.height.abs_diff(target.height),
            ]
            .into_iter()
            .max()
            .unwrap_or(0)
        };
        let total = distance(self.to);
        if total == 0 {
            return duration;
        }
        (f64::from(duration) * (f64::from(distance(self.at(at))) / f64::from(total)).min(1.0))
            .round() as u32
    }

    pub fn at(self, ticks: i64) -> Rect {
        self.from.interpolate(self.to, self.timing.progress(ticks))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_curves_round_trip_and_keep_the_legacy_default() {
        let old: AnimationConfig = serde_json::from_str(r#"{"preset":"custom"}"#).unwrap();
        assert!(serde_json::to_value(&old)
            .unwrap()
            .get("customCurve")
            .is_none());
        assert_eq!(Timing::new(0, 1000, 1000, &old).progress(250), 0.15625);
        for (name, quarter) in [("linear", 0.25), ("easeIn", 0.0625), ("easeOut", 0.4375)] {
            let encoded = serde_json::json!({"preset":"custom", "customCurve":name});
            let cfg: AnimationConfig = serde_json::from_value(encoded).unwrap();
            let timing = Timing::new(0, 1000, 1000, &cfg);
            assert_eq!(timing.progress(250), quarter);
            assert_eq!(timing.progress(1000), 1.0);
            assert_eq!(serde_json::to_value(cfg).unwrap()["customCurve"], name);
        }
    }

    #[test]
    fn reversal_continues_from_the_compositor_curve() {
        let compact = Rect {
            x: -160,
            y: 0,
            width: 160,
            height: 40,
        };
        let expanded = Rect {
            x: -240,
            y: 0,
            width: 320,
            height: 180,
        };
        for preset in [
            AnimationPreset::Smooth,
            AnimationPreset::Subtle,
            AnimationPreset::Spring,
            AnimationPreset::Custom,
        ] {
            let animation = AnimationConfig {
                preset,
                ..Default::default()
            };
            let forward = Movement {
                from: compact,
                to: expanded,
                timing: Timing::new(0, 1000, 300, &animation),
                radii: [20.0, 24.0],
            };
            let current = forward.at(111);
            assert!(current.width > compact.width && current.width < expanded.width);
            let reverse = Movement {
                from: current,
                to: compact,
                timing: Timing::new(
                    111,
                    1000,
                    forward.remaining_ms(compact, 111, 180),
                    &animation,
                ),
                radii: [forward.radius_at(111), 20.0],
            };
            assert_eq!(reverse.at(111), current);
            assert_eq!(reverse.radius_at(111), forward.radius_at(111));
            assert!(forward.remaining_ms(compact, 111, 180) < 180);
            assert_eq!(reverse.at(300), compact);
        }
    }
}
