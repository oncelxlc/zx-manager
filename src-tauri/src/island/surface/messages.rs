use super::{error, Context, IslandConfig, IslandState, Rect, Renderer, Result};
use crate::island::{notifications::NotificationSnapshot, render::Panel};
use windows_sys::Win32::{
    Foundation::{HWND, SYSTEMTIME},
    System::SystemInformation::GetLocalTime,
    UI::WindowsAndMessaging::*,
};

pub(super) const MINUTE_TIMER: usize = 0x4e50;

pub(super) fn update(ui: &Context, hwnd: HWND, snapshot: NotificationSnapshot) -> Result<()> {
    let mut model = ui.messages.borrow_mut();
    if model.snapshot == snapshot {
        return Ok(());
    }
    if model.snapshot.windows != snapshot.windows {
        model.page = 0;
    }
    model.snapshot = snapshot;
    drop(model);
    redraw(ui, hwnd)
}

pub(super) fn prepare(
    ui: &Context,
    hwnd: HWND,
    renderer: &mut Renderer,
    cfg: &IslandConfig,
    state: IslandState,
    target: Rect,
    scale: f64,
) -> Result<()> {
    let mut local = SYSTEMTIME::default();
    unsafe {
        GetLocalTime(&mut local);
    }
    let time = format!("{:02}:{:02}", local.wHour, local.wMinute);
    let date = format!("{}月{}日", local.wMonth, local.wDay);
    renderer.panel = Panel::build(
        &ui.messages.borrow(),
        cfg,
        state,
        target,
        scale,
        &time,
        &date,
    );
    if renderer.panel.as_ref().is_some_and(|p| p.time.is_some()) {
        let delay = 60_000 - u32::from(local.wSecond) * 1000 - u32::from(local.wMilliseconds);
        if unsafe { SetTimer(hwnd, MINUTE_TIMER, delay.max(1), None) } == 0 {
            return Err(error("安排时钟更新失败"));
        }
    } else {
        unsafe {
            KillTimer(hwnd, MINUTE_TIMER);
        }
    }
    Ok(())
}

pub(super) fn redraw(ui: &Context, hwnd: HWND) -> Result<()> {
    let frame = ui.frame.borrow();
    let Some(f) = frame.as_ref() else {
        return Ok(());
    };
    if ui.dragging.get() {
        return Ok(());
    }
    let mut render = ui.renderer.borrow_mut();
    let Some(r) = render.as_mut() else {
        return Ok(());
    };
    prepare(ui, hwnd, r, &f.cfg, f.state, f.target, f.scale)?;
    r.apply(&f.cfg, f.state, f.target, f.host, f.scale, false)?;
    let visible = r.visible_rect().unwrap_or(f.target);
    if ui
        .a11y
        .update_panel(&f.cfg, f.state, visible, r.hit_panel())
    {
        unsafe {
            windows_sys::Win32::UI::Accessibility::NotifyWinEvent(
                EVENT_OBJECT_REORDER,
                hwnd,
                OBJID_CLIENT,
                CHILDID_SELF as i32,
            );
        }
    }
    Ok(())
}
