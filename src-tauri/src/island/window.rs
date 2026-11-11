use super::config::{IslandConfig, PerformanceMode};
use super::geometry::{Display, Rect};
use super::state::IslandState;
use std::cell::Cell;
use tauri::window::{Effect, EffectsBuilder};
use tauri::{AppHandle, Monitor, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub(super) fn display(monitor: &Monitor, primary: Option<&Monitor>) -> Display {
    let pos = monitor.position();
    let name = monitor.name().cloned().unwrap_or_else(|| {
        format!(
            "{} × {} @ {},{}",
            monitor.size().width,
            monitor.size().height,
            pos.x,
            pos.y
        )
    });
    let work = monitor.work_area();
    Display {
        id: name.clone(),
        name,
        primary: primary.is_some_and(|p| p.position() == pos),
        scale_factor: monitor.scale_factor(),
        work: Rect {
            x: work.position.x,
            y: work.position.y,
            width: work.size.width.max(1) as i32,
            height: work.size.height.max(1) as i32,
        },
    }
}

pub fn displays(app: &AppHandle) -> Result<Vec<Display>, String> {
    let primary = app.primary_monitor().map_err(|e| e.to_string())?;
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    Ok(monitors
        .iter()
        .map(|m| display(m, primary.as_ref()))
        .collect())
}

pub struct WindowController {
    window: WebviewWindow,
    previous: Cell<IslandState>,
    acrylic: Cell<bool>,
}

impl WindowController {
    pub fn new(app: &AppHandle) -> Result<Self, String> {
        let window =
            WebviewWindowBuilder::new(app, "island", WebviewUrl::App("index.html#/island".into()))
                .title("zx-once 灵动岛")
                .inner_size(160.0, 40.0)
                .visible(false)
                .focused(false)
                .focusable(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .resizable(false)
                .maximizable(false)
                .minimizable(false)
                .decorations(false)
                .shadow(false)
                .transparent(true)
                .background_color(tauri::window::Color(0, 0, 0, 0))
                .disable_drag_drop_handler()
                .build()
                .map_err(|e| e.to_string())?;
        Ok(Self {
            window,
            previous: Cell::new(IslandState::Hidden),
            acrylic: Cell::new(false),
        })
    }

    pub fn webview(&self) -> &WebviewWindow {
        &self.window
    }

    pub fn apply(
        &self,
        cfg: &IslandConfig,
        state: IslandState,
        screen: &Display,
    ) -> Result<(), String> {
        let target = screen.target(cfg, state);
        let acrylic =
            cfg.appearance.blur && cfg.animation.performance != PerformanceMode::Performance;
        if self.acrylic.replace(acrylic) != acrylic {
            let effect = acrylic.then(|| EffectsBuilder::new().effect(Effect::Acrylic).build());
            // The WebView's tinted background remains visible when Windows omits the material.
            if let Err(error) = self.window.set_effects(effect) {
                eprintln!("灵动岛毛玻璃切换失败：{error}");
            }
        }
        let previous = self.previous.get();
        let duration = if previous == IslandState::EdgePeek || state == IslandState::EdgePeek {
            cfg.animation.edge_ms
        } else if matches!(state, IslandState::Expanded | IslandState::Pinned) {
            cfg.animation.expand_ms
        } else {
            cfg.animation.collapse_ms
        };
        let animate = cfg.animation.enabled
            && !cfg.animation.reduced_motion
            && cfg.animation.performance != PerformanceMode::Performance;
        let animation = cfg.animation.clone();
        let radius = (cfg.appearance.radius * screen.scale_factor).round() as i32;
        super::native::on_ui(&self.window, move |hwnd| {
            super::native::motion::apply(
                hwnd,
                target,
                radius,
                if animate { duration } else { 0 },
                &animation,
                state != previous,
            )
        })?;
        self.previous.set(state);
        Ok(())
    }

    pub fn hide(&self) -> Result<(), String> {
        super::native::on_ui(&self.window, |hwnd| {
            super::native::motion::hide(hwnd);
            Ok(())
        })?;
        self.previous.set(IslandState::Hidden);
        Ok(())
    }

    pub fn outer_rect(&self) -> Result<Rect, String> {
        let position = self.window.outer_position().map_err(|e| e.to_string())?;
        let size = self.window.outer_size().map_err(|e| e.to_string())?;
        Ok(Rect {
            x: position.x,
            y: position.y,
            width: size.width as i32,
            height: size.height as i32,
        })
    }

    pub fn shutdown(&self) -> Result<(), String> {
        let hidden = self.hide();
        self.window.destroy().map_err(|e| e.to_string())?;
        hidden
    }
}
