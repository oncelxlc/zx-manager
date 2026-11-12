use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 6;

macro_rules! config_enum {
    ($name:ident { $($item:ident),+ }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub enum $name { $($item),+ }
    };
}

config_enum!(Anchor {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
    Custom
});
config_enum!(IslandShape { Notch, Pill });
config_enum!(EdgeMode {
    Disabled,
    Compact,
    Peek,
    FullHide
});
config_enum!(Theme {
    System,
    Light,
    Dark
});
config_enum!(AnimationPreset {
    Subtle,
    Smooth,
    Spring,
    Custom
});
config_enum!(PerformanceMode {
    Performance,
    Balanced,
    Quality
});
config_enum!(CustomCurve {
    Linear,
    EaseIn,
    EaseOut
});

macro_rules! config_group {
    ($name:ident { $($(#[$attr:meta])* $field:ident : $ty:ty = $default:expr),+ $(,)? }) => {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        #[serde(default, deny_unknown_fields, rename_all = "camelCase")]
        pub struct $name { $($(#[$attr])* pub $field: $ty),+ }
        impl Default for $name {
            fn default() -> Self { Self { $($field: $default),+ } }
        }
    };
}

config_group!(IslandConfig {
    schema_version: u32 = SCHEMA_VERSION,
    revision: u64 = 0,
    enabled: bool = true,
    startup: StartupConfig = StartupConfig::default(),
    placement: PlacementConfig = PlacementConfig::default(),
    edge: EdgeConfig = EdgeConfig::default(),
    appearance: AppearanceConfig = AppearanceConfig::default(),
    animation: AnimationConfig = AnimationConfig::default(),
    interaction: InteractionConfig = InteractionConfig::default(),
    notifications: NotificationConfig = NotificationConfig::default(),
});

config_group!(NotificationConfig {
    enabled: bool = true,
});

config_group!(StartupConfig {
    launch_at_login: bool = false,
    show_on_start: bool = true,
    start_compact: bool = true,
});

// The active axis ranges from -100 to 100 percent of the dock's travel.
config_group!(PlacementConfig {
    anchor: Anchor = Anchor::TopCenter,
    display_id: Option<String> = None,
    offset_x: f64 = 0.0,
    offset_y: f64 = 0.0,
    remember_position: bool = true,
});

config_group!(EdgeConfig {
    mode: EdgeMode = EdgeMode::Compact,
    auto_hide: bool = true,
    hide_delay_ms: u32 = 900,
    wake_delay_ms: u32 = 80,
    hide_on_fullscreen: bool = true,
});

config_group!(AppearanceConfig {
    island_size: u32 = 100,
    minimal: bool = false,
    pet: bool = true,
    show_battery: bool = true,
    lyrics_enabled: bool = true,
    shape: IslandShape = IslandShape::Notch,
    // Per-frame body dimensions in island coordinates, before shoulders and scaling.
    #[serde(skip)]
    compact_width: f64 = 120.0,
    #[serde(skip)]
    compact_height: f64 = 30.0,
    #[serde(skip)]
    expanded_width: f64 = 419.0,
    #[serde(skip)]
    expanded_height: f64 = 140.0,
    #[serde(skip)]
    unread_extent: f64 = 0.0,
    #[serde(skip)]
    radius: f64 = 33.0,
    opacity: f64 = 1.0,
    background: String = "#000000".into(),
    border: String = "#000000".into(),
    shadow: bool = true,
    blur: bool = false,
    theme: Theme = Theme::Dark,
});

config_group!(AnimationConfig {
    enabled: bool = true,
    preset: AnimationPreset = AnimationPreset::Smooth,
    #[serde(skip_serializing_if = "Option::is_none")]
    custom_curve: Option<CustomCurve> = None,
    expand_ms: u32 = 300,
    collapse_ms: u32 = 240,
    edge_ms: u32 = 220,
    content_ms: u32 = 180,
    amplitude: f64 = 1.0,
    reduced_motion: bool = false,
    performance: PerformanceMode = PerformanceMode::Balanced,
});

config_group!(InteractionConfig {
    hover_expand: bool = true,
    click_expand: bool = true,
    pin_enabled: bool = true,
    drag_enabled: bool = true,
});

impl AnimationConfig {
    /// Coefficients of a*t + b*t² + c*t³. An absent curve preserves the original Custom preset.
    pub fn custom_coefficients(&self) -> [f64; 3] {
        match self.custom_curve {
            None => [0.0, 3.0, -2.0],
            Some(CustomCurve::Linear) => [1.0, 0.0, 0.0],
            Some(CustomCurve::EaseIn) => [0.0, 1.0, 0.0],
            Some(CustomCurve::EaseOut) => [2.0, -1.0, 0.0],
        }
    }
}

impl IslandConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(format!("不支持的配置版本 {}", self.schema_version));
        }
        if self.revision > (1 << 53) - 1 {
            return Err("配置版本计数超出客户端可表示的范围".into());
        }
        if self.placement.display_id.as_ref().is_some_and(|id| {
            id.trim().is_empty() || id.len() > 512 || id.chars().any(char::is_control)
        }) {
            return Err("显示器标识无效".into());
        }
        let a = &self.appearance;
        if !(80..=140).contains(&a.island_size) || !a.island_size.is_multiple_of(5) {
            return Err("岛体大小必须为 80% 至 140%，每档 5%".into());
        }
        if !matches!(
            self.placement.anchor,
            Anchor::TopCenter | Anchor::CenterLeft | Anchor::CenterRight
        ) {
            return Err("请选择顶部、左侧或右侧停靠".into());
        }
        for offset in [self.placement.offset_x, self.placement.offset_y] {
            if !(-100.0..=100.0).contains(&offset) || offset.fract() != 0.0 {
                return Err("沿边位置必须是 -100 至 100 之间的整数百分比".into());
            }
        }
        for (name, value, min, max) in [
            ("opacity", a.opacity, 0.0, 1.0),
            ("amplitude", self.animation.amplitude, 0.0, 2.0),
        ] {
            if !(min..=max).contains(&value) {
                return Err(format!("{name} 必须在 {min} 至 {max} 之间"));
            }
        }
        for color in [&a.background, &a.border] {
            if color.len() != 7
                || !color.starts_with('#')
                || !color[1..].bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err("颜色格式应为 #RRGGBB".into());
            }
        }
        for (name, value, max) in [
            ("hideDelayMs", self.edge.hide_delay_ms, 60000),
            ("wakeDelayMs", self.edge.wake_delay_ms, 60000),
            ("expandMs", self.animation.expand_ms, 2000),
            ("collapseMs", self.animation.collapse_ms, 2000),
            ("edgeMs", self.animation.edge_ms, 2000),
            ("contentMs", self.animation.content_ms, 2000),
        ] {
            if value > max {
                return Err(format!("{name} 不得超过 {max} 毫秒"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn javascript_fixture_contains_every_default() {
        let fixture = include_str!("../../../src/features/island/defaults.json");
        let expected: serde_json::Value = serde_json::from_str(fixture).unwrap();
        assert_eq!(
            serde_json::to_value(IslandConfig::default()).unwrap(),
            expected
        );
    }

    #[test]
    fn rejects_nonfinite_geometry() {
        let mut config = IslandConfig::default();
        config.placement.offset_y = f64::NAN;
        assert!(config.validate().is_err());
        config.placement.offset_y = 0.0;
        config.appearance.opacity = f64::INFINITY;
        assert!(config.validate().is_err());
        config.appearance.opacity = 1.0;
        assert!(config.validate().is_ok());
    }
}
