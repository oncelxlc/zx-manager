use super::{
    config::IslandConfig,
    geometry::{Display, Rect},
    native::{self, NativeEvent},
    render::RenderStats,
    state::IslandState,
    surface::Surface,
    window::{self, WindowController},
};
use serde::Serialize;
use tauri::AppHandle;
use windows_sys::Win32::Foundation::HWND;

type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum RendererKind {
    Webview,
    Native,
}

impl RendererKind {
    pub fn from_env() -> Result<Self> {
        match std::env::var("ZX_ISLAND_RENDERER").as_deref() {
            Err(std::env::VarError::NotPresent) | Ok("webview") => Ok(Self::Webview),
            Ok("native") => Ok(Self::Native),
            _ => Err("ZX_ISLAND_RENDERER 只接受 webview 或 native".into()),
        }
    }
}

pub(super) enum Backend {
    Webview(Box<WindowController>),
    Native(Surface),
}

impl Backend {
    pub fn new(
        kind: RendererKind,
        app: &AppHandle,
        notify: impl Fn(NativeEvent) + Send + Sync + 'static,
    ) -> Result<Self> {
        match kind {
            RendererKind::Webview => WindowController::new(app).map(Box::new).map(Self::Webview),
            RendererKind::Native => {
                Surface::start(move |event| notify(NativeEvent::Input(event))).map(Self::Native)
            }
        }
    }

    pub fn kind(&self) -> RendererKind {
        match self {
            Self::Webview(_) => RendererKind::Webview,
            Self::Native(_) => RendererKind::Native,
        }
    }

    pub fn on_ui<T: Send + 'static>(
        &self,
        job: impl FnOnce(HWND) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        match self {
            Self::Webview(window) => native::on_ui(window.webview(), job),
            Self::Native(surface) => surface.call(job),
        }
    }

    pub fn apply(&self, app: &AppHandle, cfg: &IslandConfig, state: IslandState) -> Result<()> {
        if matches!(
            state,
            IslandState::Disabled | IslandState::Hidden | IslandState::Suspended
        ) {
            return match self {
                Self::Webview(w) => w.hide(),
                Self::Native(s) => s.hide(),
            };
        }
        let displays = window::displays(app)?;
        let screen = target_display(&displays, cfg.placement.display_id.as_deref())?;
        match self {
            Self::Webview(w) => w.apply(cfg, state, screen),
            Self::Native(s) => s.apply(cfg, state, screen),
        }
    }

    pub fn outer_rect(&self) -> Result<Rect> {
        match self {
            Self::Webview(w) => w.outer_rect(),
            Self::Native(s) => s.outer_rect(),
        }
    }

    pub fn drag_position(
        &self,
        app: &AppHandle,
        cfg: &IslandConfig,
        state: IslandState,
        at: Rect,
    ) -> Result<(String, f64, f64)> {
        let screen = match self {
            Self::Webview(w) => {
                let monitor = w
                    .webview()
                    .current_monitor()
                    .map_err(|e| e.to_string())?
                    .ok_or("岛屿当前显示器已断开")?;
                window::display(&monitor, None)
            }
            Self::Native(_) => window::displays(app)?
                .into_iter()
                .max_by_key(|d| overlap_and_distance(d.work, at))
                .ok_or("找不到可用显示器")?,
        };
        let (x, y) = screen.drag_offset(cfg, state, at);
        Ok((screen.id, x, y))
    }

    pub fn shutdown(&self) -> Result<()> {
        match self {
            Self::Webview(w) => w.shutdown(),
            Self::Native(s) => s.shutdown(),
        }
    }

    pub fn stats(&self) -> Result<Option<RenderStats>> {
        match self {
            Self::Webview(_) => Ok(None),
            Self::Native(s) => s.stats().map(Some),
        }
    }
}

// A partially off-screen island can have its center in a gap between monitors.
fn overlap_and_distance(work: Rect, island: Rect) -> (i64, std::cmp::Reverse<i128>) {
    let overlap = |a, length_a, b, length_b| {
        let (a, b) = (i64::from(a), i64::from(b));
        (a + i64::from(length_a)).min(b + i64::from(length_b)) - a.max(b)
    };
    let dx = overlap(work.x, work.width, island.x, island.width);
    let dy = overlap(work.y, work.height, island.y, island.height);
    let gap_squared = i128::from(dx.min(0)).pow(2) + i128::from(dy.min(0)).pow(2);
    (dx.max(0) * dy.max(0), std::cmp::Reverse(gap_squared))
}

fn target_display<'a>(screens: &'a [Display], id: Option<&str>) -> Result<&'a Display> {
    screens
        .iter()
        .find(|s| Some(s.id.as_str()) == id)
        .or_else(|| screens.iter().find(|s| s.primary))
        .or_else(|| screens.first())
        .ok_or_else(|| "找不到可用显示器".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dragging_prefers_the_largest_overlap_then_the_nearest_work_area() {
        let left = Rect {
            x: -1000,
            y: 0,
            width: 800,
            height: 600,
        };
        let right = Rect { x: 0, ..left };
        // Center (-75, 120) is in the gap, with 25 px on the left and 75 px on the right.
        let island = Rect {
            x: -225,
            y: 100,
            width: 300,
            height: 40,
        };
        assert!(overlap_and_distance(right, island) > overlap_and_distance(left, island));
        let in_gap = Rect {
            x: -180,
            width: 30,
            ..island
        };
        assert!(overlap_and_distance(left, in_gap) > overlap_and_distance(right, in_gap));
    }

    #[test]
    fn unplugged_display_falls_back_without_changing_the_saved_choice() {
        let display = |id: &str, primary| Display {
            id: id.into(),
            name: id.into(),
            primary,
            scale_factor: 1.0,
            work: Rect {
                x: 0,
                y: 0,
                width: 800,
                height: 600,
            },
        };
        let screens = [display("left", false), display("right", true)];
        assert_eq!(target_display(&screens, Some("left")).unwrap().id, "left");
        assert_eq!(target_display(&screens, Some("gone")).unwrap().id, "right");
        assert_eq!(
            target_display(&screens[..1], Some("gone")).unwrap().id,
            "left"
        );
        assert!(target_display(&[], None).is_err());
    }
}
