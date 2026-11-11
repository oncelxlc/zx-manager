use super::{error, placement, Context, IslandState, Rect, SurfaceEvent};
use crate::island::{render::action_at, state::Event};
use std::{cell::Cell, mem::size_of};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT},
    Graphics::Gdi::ClientToScreen,
    UI::{
        Controls::WM_MOUSELEAVE,
        Input::KeyboardAndMouse::{
            GetCapture, ReleaseCapture, SetCapture, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT,
        },
        WindowsAndMessaging::*,
    },
};

#[derive(Clone, Copy, PartialEq)]
enum Action {
    Body,
    Pin(bool),
    Hide,
}

#[derive(Default)]
pub(super) struct Pointer {
    inside: Cell<bool>,
    tracking: Cell<bool>,
    down: Cell<Option<(i32, i32, Action)>>,
}

impl Pointer {
    pub fn clear(&self) {
        self.inside.set(false);
        self.tracking.set(false);
        self.down.set(None);
    }
}

fn rounded(r: Rect, radius: f64, p: POINT) -> bool {
    let x = f64::from(p.x - r.x) + 0.5;
    let y = f64::from(p.y - r.y) + 0.5;
    let w = f64::from(r.width);
    let h = f64::from(r.height);
    if x < 0.0 || y < 0.0 || x >= w || y >= h {
        return false;
    }
    let radius = radius.clamp(0.0, w.min(h) / 2.0);
    let dx = x - x.clamp(radius, w - radius);
    let dy = y - y.clamp(radius, h - radius);
    dx * dx + dy * dy <= radius * radius
}

fn action(ui: &Context, hwnd: HWND, point: POINT) -> Option<Action> {
    let rect = placement::outer_rect(ui, hwnd).ok()?;
    let frame = ui.frame.try_borrow().ok()?;
    let f = frame.as_ref()?;
    let target = f.target;
    let sample = POINT {
        x: target.x
            + (f64::from(point.x - rect.x) * f64::from(target.width) / f64::from(rect.width))
                .floor() as i32,
        y: target.y
            + (f64::from(point.y - rect.y) * f64::from(target.height) / f64::from(rect.height))
                .floor() as i32,
    };
    if !rounded(target, f.cfg.appearance.radius * f.scale, sample) {
        return None;
    }
    Some(
        match action_at(&f.cfg, f.state, target, f.scale, sample.x, sample.y) {
            Some(Event::Pin) => Action::Pin(true),
            Some(Event::Unpin) => Action::Pin(false),
            Some(Event::Hide) => Action::Hide,
            _ => Action::Body,
        },
    )
}

fn update_hover(ui: &Context, hwnd: HWND, hit: Option<Action>) {
    let inside = hit.is_some();
    let control = match hit {
        Some(Action::Pin(true)) => Some(Event::Pin),
        Some(Action::Pin(false)) => Some(Event::Unpin),
        Some(Action::Hide) => Some(Event::Hide),
        _ => None,
    };
    if ui
        .renderer
        .try_borrow_mut()
        .ok()
        .is_some_and(|mut r| r.as_mut().is_some_and(|r| r.set_hovered(control)))
    {
        ui.redraw_pending.set(true);
    }
    if inside && !ui.input.tracking.get() {
        let mut track = TRACKMOUSEEVENT {
            cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
            dwFlags: TME_LEAVE,
            hwndTrack: hwnd,
            dwHoverTime: 0,
        };
        if unsafe { TrackMouseEvent(&mut track) } != 0 {
            ui.input.tracking.set(true);
        } else {
            (ui.emit)(SurfaceEvent::Error(error("注册鼠标离开通知失败")));
        }
    }
    if ui.input.inside.replace(inside) != inside {
        (ui.emit)(if inside {
            SurfaceEvent::PointerEnter
        } else {
            SurfaceEvent::PointerLeave
        });
    }
}

pub(super) unsafe fn message(ui: &Context, hwnd: HWND, msg: u32, lp: LPARAM) -> Option<LRESULT> {
    match msg {
        WM_CAPTURECHANGED | WM_CANCELMODE => {
            ui.input.down.set(None);
            return None;
        }
        WM_MOUSELEAVE => {
            ui.input.tracking.set(false);
            update_hover(ui, hwnd, None);
            return Some(0);
        }
        WM_SETCURSOR if lp as u16 == HTCLIENT as u16 => {
            SetCursor(LoadCursorW(std::ptr::null_mut(), IDC_HAND));
            return Some(1);
        }
        WM_NCHITTEST | WM_MOUSEMOVE | WM_LBUTTONDOWN | WM_LBUTTONUP => (),
        _ => return None,
    }
    let mut point = POINT {
        x: i32::from(lp as i16),
        y: i32::from((lp >> 16) as i16),
    };
    if msg != WM_NCHITTEST && ClientToScreen(hwnd, &mut point) == 0 {
        return None;
    }
    let hit = action(ui, hwnd, point);
    match msg {
        WM_NCHITTEST => {
            return Some(if hit.is_some() {
                HTCLIENT as LRESULT
            } else {
                HTTRANSPARENT as LRESULT
            })
        }
        WM_MOUSEMOVE => {
            update_hover(ui, hwnd, hit);
            if let Some((x, y, Action::Body)) = ui.input.down.get() {
                let drag = ui.frame.borrow().as_ref().is_some_and(|f| {
                    f.state != IslandState::EdgePeek
                        && f.cfg.interaction.drag_enabled
                        && f64::from(point.x - x).hypot(f64::from(point.y - y)) >= 5.0 * f.scale
                });
                if drag {
                    ui.input.down.set(None);
                    (ui.emit)(SurfaceEvent::DragStart);
                }
            }
        }
        WM_LBUTTONDOWN => {
            if let Some(hit) = hit {
                ui.input.down.set(Some((point.x, point.y, hit)));
                SetCapture(hwnd);
            }
        }
        WM_LBUTTONUP => {
            let down = ui.input.down.take();
            if GetCapture() == hwnd {
                ReleaseCapture();
            }
            if let Some((_, _, action)) = down.filter(|(_, _, action)| hit == Some(*action)) {
                (ui.emit)(match action {
                    Action::Body => SurfaceEvent::Click,
                    Action::Pin(pinned) => SurfaceEvent::Pin(pinned),
                    Action::Hide => SurfaceEvent::Hide,
                });
            }
        }
        _ => unreachable!(),
    }
    Some(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rounded_shape_skips_corners_at_negative_origin() {
        let r = Rect {
            x: -200,
            y: -100,
            width: 180,
            height: 42,
        };
        assert!(!rounded(r, 24.0, POINT { x: -200, y: -100 }));
        assert!(rounded(r, 24.0, POINT { x: -200, y: -79 }));
        assert!(!rounded(r, 24.0, POINT { x: -20, y: -79 }));
    }
}
