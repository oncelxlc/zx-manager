use super::{error, Context, IslandConfig, IslandState, Rect, Renderer, Result, SurfaceEvent};
use crate::island::{
    config::{Orientation, PerformanceMode},
    render::notch::Notch,
};
use raw_window_handle::{RawWindowHandle, Win32WindowHandle, WindowHandle};
use std::{num::NonZeroIsize, ptr::null_mut};
use windows_sys::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::Gdi::{CreatePolygonRgn, CreateRoundRectRgn, DeleteObject, SetWindowRgn, WINDING},
    UI::Input::KeyboardAndMouse::{GetCapture, ReleaseCapture},
    UI::WindowsAndMessaging::*,
};

pub(super) struct Frame {
    pub cfg: IslandConfig,
    pub state: IslandState,
    pub host: Rect,
    pub target: Rect,
    pub scale: f64,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) struct Mask {
    pub bounds: Rect,
    diameter: i32,
    pub notch: Option<Notch>,
}

fn current(hwnd: HWND) -> Result<Rect> {
    let mut rect = RECT::default();
    if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
        return Err(error("读取原生窗口位置失败"));
    }
    Ok(Rect {
        x: rect.left,
        y: rect.top,
        width: rect.right - rect.left,
        height: rect.bottom - rect.top,
    })
}

pub(super) fn outer_rect(ui: &Context, hwnd: HWND) -> Result<Rect> {
    let outer = current(hwnd)?;
    let frame = ui.frame.borrow();
    let Some(frame) = frame.as_ref() else {
        return Ok(outer);
    };
    let mut rect = ui
        .renderer
        .try_borrow()
        .ok()
        .and_then(|r| r.as_ref()?.visible_rect())
        .unwrap_or(frame.target);
    // The OS move loop moves the host, while the renderer stores the original desktop origin.
    rect.x += outer.x - frame.host.x;
    rect.y += outer.y - frame.host.y;
    Ok(rect)
}

fn union(a: Rect, b: Rect) -> Rect {
    let x = a.x.min(b.x);
    let y = a.y.min(b.y);
    Rect {
        x,
        y,
        width: (a.x + a.width).max(b.x + b.width) - x,
        height: (a.y + a.height).max(b.y + b.height) - y,
    }
}

fn region(
    ui: &Context,
    hwnd: HWND,
    host: Rect,
    r: Rect,
    radius: f64,
    notch: Option<Notch>,
) -> Result<()> {
    let x = r.x - host.x;
    let y = r.y - host.y;
    let diameter = ((2.0 * radius).round() as i32).clamp(1, r.width.min(r.height));
    let mask = Mask {
        bounds: Rect { x, y, ..r },
        diameter,
        notch,
    };
    if ui.mask.get() == Some(mask) {
        return Ok(());
    }
    unsafe {
        let region = if let Some(notch) = notch {
            let points = notch.outline(f64::from(r.width), f64::from(r.height), |px, py| POINT {
                x: x + px.round() as i32,
                y: y + py.round() as i32,
            });
            CreatePolygonRgn(points.as_ptr(), points.len() as i32, WINDING)
        } else {
            CreateRoundRectRgn(x, y, x + r.width + 1, y + r.height + 1, diameter, diameter)
        };
        if region.is_null() {
            return Err(error("创建原生命中区域失败"));
        }
        if SetWindowRgn(hwnd, region, 0) == 0 {
            let e = error("设置原生命中区域失败");
            DeleteObject(region);
            return Err(e);
        }
    }
    ui.mask.set(Some(mask));
    Ok(())
}

fn acrylic(ui: &Context, hwnd: HWND, enabled: bool) {
    if ui.acrylic.get() == enabled {
        return;
    }
    let Some(raw) = NonZeroIsize::new(hwnd as isize) else {
        return;
    };
    // Every call is synchronous on the owner thread and ends before it can destroy this HWND.
    let handle =
        unsafe { WindowHandle::borrow_raw(RawWindowHandle::Win32(Win32WindowHandle::new(raw))) };
    let result = if enabled {
        window_vibrancy::apply_acrylic(handle, None)
    } else {
        window_vibrancy::clear_acrylic(handle)
    };
    match result {
        Ok(()) => ui.acrylic.set(enabled),
        Err(e) => (ui.emit)(SurfaceEvent::Error(format!("灵动岛毛玻璃切换失败: {e}"))),
    }
}

fn material(frame: &Frame) -> bool {
    frame.state != IslandState::EdgePeek
        && frame.cfg.appearance.blur
        && frame.cfg.animation.performance != PerformanceMode::Performance
}

fn position(hwnd: HWND, host: Rect) -> Result<()> {
    if current(hwnd)? != host
        && unsafe {
            SetWindowPos(
                hwnd,
                null_mut(),
                host.x,
                host.y,
                host.width,
                host.height,
                SWP_NOZORDER | SWP_NOACTIVATE,
            )
        } == 0
    {
        return Err(error("放置原生窗口失败"));
    }
    Ok(())
}

pub(super) fn prepare_drag(ui: &Context, hwnd: HWND) -> Result<Rect> {
    let visible = outer_rect(ui, hwnd)?;
    let configured_radius = ui
        .frame
        .borrow()
        .as_ref()
        .map(|f| f.cfg.appearance.radius * f.scale)
        .ok_or("隐藏的岛屿无法拖动")?;
    let radius = ui
        .renderer
        .borrow()
        .as_ref()
        .and_then(Renderer::visible_radius)
        .unwrap_or(configured_radius);
    ui.clock.stop();
    ui.renderer
        .borrow_mut()
        .as_mut()
        .ok_or("原生绘制设备不可用")?
        .freeze(visible)?;
    position(hwnd, visible)?;
    let notch = ui.renderer.borrow().as_ref().and_then(|r| r.notch);
    region(ui, hwnd, visible, visible, radius, notch)?;
    if let Some(frame) = ui.frame.borrow_mut().as_mut() {
        frame.host = visible;
    }
    ui.dragging.set(true);
    Ok(visible)
}

pub(super) fn apply(
    ui: &Context,
    hwnd: HWND,
    cfg: IslandConfig,
    state: IslandState,
    target: Rect,
    scale: f64,
) -> Result<()> {
    ui.dragging.set(false);
    if ui.renderer.borrow().is_none() {
        ui.renderer.replace(Some(Renderer::new(hwnd)?));
    }
    let previous = ui
        .frame
        .borrow()
        .as_ref()
        .map(|f| (f.state, f.target, f.host, f.scale));
    let mut animation_allowed: i32 = 1;
    unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            (&raw mut animation_allowed).cast(),
            0,
        );
    }
    let changed_state = previous.is_none_or(|p| p.0 != state && p.3 == scale);
    let transition = animation_allowed != 0
        && cfg.animation.enabled
        && !cfg.animation.reduced_motion
        && cfg.animation.performance != PerformanceMode::Performance
        && previous.is_some()
        && changed_state;
    let duration_ms = match previous {
        Some((IslandState::EdgePeek, ..)) => cfg.animation.edge_ms,
        _ if state == IslandState::EdgePeek => cfg.animation.edge_ms,
        _ if matches!(state, IslandState::Expanded | IslandState::Pinned) => {
            cfg.animation.expand_ms
        }
        _ => cfg.animation.collapse_ms,
    };
    let same_target = previous.is_some_and(|p| (p.0, p.1, p.3) == (state, target, scale));
    let continuing = same_target && ui.clock.active();
    let host = if continuing {
        previous.unwrap().2
    } else if transition && duration_ms > 0 {
        union(outer_rect(ui, hwnd)?, target)
    } else {
        target
    };
    if !continuing {
        ui.clock.stop();
    }
    position(hwnd, host)?;
    {
        let mut renderer = ui.renderer.borrow_mut();
        let renderer = renderer.as_mut().ok_or("原生绘制设备不可用")?;
        renderer.notch = (target.y == ui.work_top.get()
            && target.width >= target.height
            && cfg.placement.orientation != Orientation::Vertical
            && state != IslandState::EdgePeek)
            .then(|| {
                Notch::new(
                    state,
                    f64::from(target.width) / scale,
                    f64::from(target.height) / scale,
                    cfg.appearance.radius,
                )
            });
        super::messages::prepare(ui, hwnd, renderer, &cfg, state, target, scale)?;
        renderer.apply(&cfg, state, target, host, scale, changed_state)?;
        ui.recovery_pending.set(false);
        if ui.a11y.update_panel(
            &cfg,
            state,
            renderer.visible_rect().unwrap_or(target),
            renderer.hit_panel(),
        ) {
            unsafe {
                windows_sys::Win32::UI::Accessibility::NotifyWinEvent(
                    EVENT_OBJECT_REORDER,
                    hwnd,
                    OBJID_CLIENT,
                    CHILDID_SELF as i32,
                );
            }
        }
    }
    let frame = Frame {
        cfg,
        state,
        target,
        host,
        scale,
    };
    ui.frame.replace(Some(frame));
    update_mask(ui, hwnd)?;
    if unsafe { IsWindowVisible(hwnd) } == 0 {
        unsafe {
            ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
    }
    Ok(())
}

pub(super) fn update_mask(ui: &Context, hwnd: HWND) -> Result<()> {
    let frame = ui.frame.borrow();
    let Some(f) = frame.as_ref() else {
        return Ok(());
    };
    let mut renderer = ui.renderer.borrow_mut();
    let Some(r) = renderer.as_mut() else {
        return Ok(());
    };
    let rect = r.visible_rect().unwrap_or(f.target);
    if r.panel.is_some() {
        ui.a11y.update_panel(&f.cfg, f.state, rect, r.hit_panel());
    }
    let radius = r
        .visible_radius()
        .unwrap_or(f.cfg.appearance.radius * f.scale);
    region(ui, hwnd, f.host, rect, radius, r.notch)?;
    if !r.moving() {
        ui.clock.stop();
    } else {
        let schedule = if ui.clock.active() {
            Ok(())
        } else {
            r.composition_clock().map(|cadence| ui.clock.start(cadence))
        };
        if let Err(e) = schedule.and_then(|()| ui.clock.next()) {
            // An unavailable display clock cannot leave an indefinitely stale input mask.
            ui.clock.stop();
            r.freeze(f.target)?;
            r.apply(&f.cfg, f.state, f.target, f.host, f.scale, false)?;
            region(
                ui,
                hwnd,
                f.host,
                f.target,
                f.cfg.appearance.radius * f.scale,
                r.notch,
            )?;
            (ui.emit)(SurfaceEvent::Error(e));
        }
    }
    acrylic(ui, hwnd, !ui.clock.active() && material(f));
    drop(renderer);
    drop(frame);
    if ui.input.inside.get() && !ui.dragging.get() {
        super::input::refresh_hover(ui, hwnd);
    }
    Ok(())
}

pub(super) fn hide(ui: &Context, hwnd: HWND) -> Result<()> {
    ui.messages.borrow_mut().focus_source = None;
    unsafe {
        if GetCapture() == hwnd {
            ReleaseCapture();
        }
        ShowWindow(hwnd, SW_HIDE);
    }
    ui.clock.stop();
    unsafe {
        KillTimer(hwnd, super::messages::MINUTE_TIMER);
    }
    ui.input.clear();
    ui.dragging.set(false);
    ui.a11y.clear();
    ui.frame.take();
    if let Some(r) = ui.renderer.borrow_mut().as_mut() {
        r.hide()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn envelope_includes_old_and_new_at_negative_origins() {
        let from = Rect {
            x: -450,
            y: -10,
            width: 40,
            height: 80,
        };
        let to = Rect {
            x: -470,
            y: 15,
            width: 60,
            height: 90,
        };
        assert_eq!(
            union(from, to),
            Rect {
                x: -470,
                y: -10,
                width: 60,
                height: 115
            }
        );
    }
}
