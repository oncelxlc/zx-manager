use super::{error, placement, Context, IslandState, Rect, SurfaceEvent};
use crate::island::render::MessageAction;
use std::{cell::Cell, mem::size_of};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT},
    Graphics::Gdi::{ClientToScreen, ScreenToClient},
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
    Disabled,
    Message(MessageAction),
}

#[derive(Default)]
pub(super) struct Pointer {
    pub(super) inside: Cell<bool>,
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
    let radius = ui
        .renderer
        .try_borrow()
        .ok()
        .and_then(|r| r.as_ref()?.visible_radius())
        .unwrap_or(f.cfg.appearance.radius * f.scale);
    let inside = if let Some(placement::Mask {
        bounds: mask,
        notch: Some(notch),
        ..
    }) = ui.mask.get()
    {
        let mut local = point;
        (unsafe { ScreenToClient(hwnd, &mut local) != 0 })
            && notch.contains_pixel(mask.width, mask.height, local.x - mask.x, local.y - mask.y)
    } else {
        rounded(rect, radius, point)
    };
    if !inside {
        return None;
    }
    if let Ok(renderer) = ui.renderer.try_borrow() {
        if let Some(panel) = renderer.as_ref().and_then(|r| r.hit_panel()) {
            let hit = panel.items.iter().find(|i| {
                let b = i.screen_bounds(rect, panel.size);
                point.x >= b.x
                    && point.x < b.x + b.width
                    && point.y >= b.y
                    && point.y < b.y + b.height
            });
            return Some(hit.map_or(Action::Body, |item| {
                item.action.map_or(Action::Disabled, Action::Message)
            }));
        }
    }
    Some(Action::Body)
}

fn update_hover(ui: &Context, hwnd: HWND, hit: Option<Action>) {
    let inside = hit.is_some();
    if ui.renderer.try_borrow_mut().ok().is_some_and(|mut r| {
        r.as_mut().is_some_and(|r| {
            let msg = match hit {
                Some(Action::Message(a)) => Some(a),
                _ => None,
            };
            r.hover_message(msg)
        })
    }) {
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

pub(super) fn refresh_hover(ui: &Context, hwnd: HWND) {
    let mut point = POINT::default();
    let hit = if unsafe { GetCursorPos(&mut point) != 0 && WindowFromPoint(point) == hwnd } {
        action(ui, hwnd, point)
    } else {
        None
    };
    update_hover(ui, hwnd, hit);
}

pub(super) unsafe fn message(ui: &Context, hwnd: HWND, msg: u32, lp: LPARAM) -> Option<LRESULT> {
    match msg {
        WM_CAPTURECHANGED | WM_CANCELMODE => {
            ui.input.down.set(None);
            return None;
        }
        WM_MOUSELEAVE => {
            ui.input.tracking.set(false);
            refresh_hover(ui, hwnd);
            return Some(0);
        }
        WM_SETCURSOR if lp as u16 == HTCLIENT as u16 => {
            let mut point = POINT::default();
            let clickable = GetCursorPos(&mut point) != 0
                && matches!(action(ui, hwnd, point), Some(Action::Message(_)));
            SetCursor(LoadCursorW(
                std::ptr::null_mut(),
                if clickable { IDC_HAND } else { IDC_ARROW },
            ));
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
            if matches!(down, Some((_, _, Action::Message(MessageAction::Seek)))) {
                let visible = placement::outer_rect(ui, hwnd).ok()?;
                let renderer = ui.renderer.borrow();
                let panel = renderer.as_ref()?.hit_panel()?;
                if !panel
                    .items
                    .iter()
                    .any(|item| item.action == Some(MessageAction::Seek))
                {
                    return Some(0);
                }
                let fraction = panel.seek_fraction(visible, point.x);
                (ui.emit)(SurfaceEvent::Transport(
                    crate::island::media::Transport::Seek(fraction),
                ));
                return Some(0);
            }
            if let Some((_, _, action)) = down.filter(|(x, y, action)| {
                if hit == Some(*action) {
                    return true;
                }
                // An expanding panel can move an icon underneath a stationary press.
                matches!(
                    action,
                    Action::Message(MessageAction::Window(_) | MessageAction::OpenInstances(_))
                ) && hit.is_some()
                    && ui.frame.borrow().as_ref().is_some_and(|f| {
                        f64::from(point.x - x).hypot(f64::from(point.y - y)) < 5.0 * f.scale
                    })
            }) {
                if let Action::Message(MessageAction::NextPage) = action {
                    PostMessageW(hwnd, super::NEXT_PAGE, 0, 0);
                    return Some(0);
                }
                (ui.emit)(match action {
                    Action::Body => SurfaceEvent::Click,
                    Action::Disabled => return Some(0),
                    Action::Message(a) => a.surface_event()?,
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
