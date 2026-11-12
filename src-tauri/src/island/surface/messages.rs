use super::{Context, IslandConfig, IslandState, Rect, Renderer, Result};
use crate::island::{
    config::PerformanceMode,
    notifications::{NotificationSnapshot, Source},
    render::Panel,
};
use std::time::Instant;
use windows_sys::Win32::{Foundation::HWND, UI::WindowsAndMessaging::*};

pub(super) const PULSE_TIMER: usize = 0x4e50;

pub(super) fn stop_pulse(ui: &Context, hwnd: HWND) {
    if ui.pulse.take().is_some() {
        unsafe {
            KillTimer(hwnd, PULSE_TIMER);
        }
    }
}

pub(super) fn clear_previews(ui: &Context) -> Result<()> {
    if let Some(s) = ui.messages.borrow_mut().snapshot.as_mut() {
        for reminder in &mut s.reminders {
            reminder.preview = None;
        }
    }
    if let Some(renderer) = ui.renderer.borrow_mut().as_mut() {
        renderer.purge_previews()?;
    }
    Ok(())
}

pub(super) fn update(
    ui: &Context,
    hwnd: HWND,
    mut snapshot: NotificationSnapshot,
    selected_source: Option<Source>,
    selected_window: Option<u64>,
) -> Result<()> {
    let frame = ui.frame.borrow();
    let preview = ui.preview_allowed.get()
        && frame
            .as_ref()
            .is_some_and(|f| f.cfg.notifications.show_preview && f.cfg.notifications.enabled);
    if !preview {
        for r in &mut snapshot.reminders {
            r.preview = None;
        }
    }
    let mut messages = ui.messages.borrow_mut();
    if messages.snapshot.as_ref() == Some(&snapshot)
        && messages.selected_source == selected_source
        && messages.selected_window == selected_window
    {
        return Ok(());
    }
    let motion = frame.as_ref().is_some_and(|f| {
        f.cfg.notifications.animate_new_message
            && f.cfg.animation.enabled
            && !f.cfg.animation.reduced_motion
            && f.cfg.animation.performance != PerformanceMode::Performance
            && f.state != IslandState::EdgePeek
    });
    let new = messages
        .snapshot
        .as_ref()
        .filter(|old| old.epoch == snapshot.epoch)
        .and_then(|old| {
            snapshot
                .reminders
                .iter()
                .find(|r| {
                    old.reminders
                        .iter()
                        .find(|v| v.source == r.source)
                        .is_none_or(|v| {
                            r.created_ticks > v.created_ticks
                                || (r.created_ticks == v.created_ticks && r.latest_id > v.latest_id)
                        })
                })
                .map(|r| r.source)
        });
    if messages.selected_source != selected_source {
        messages.page = 0;
    }
    messages.snapshot = Some(snapshot);
    messages.selected_source = selected_source;
    messages.selected_window = selected_window;
    drop(messages);
    drop(frame);
    if let Some(source) = new.filter(|_| motion) {
        let mut enabled: i32 = 0;
        if unsafe {
            SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, (&raw mut enabled).cast(), 0)
        } != 0
            && enabled != 0
            && unsafe { SetTimer(hwnd, PULSE_TIMER, 24, None) } != 0
        {
            ui.pulse.set(Some((source, Instant::now())));
        }
    }
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
    let mut panel = Panel::build(&ui.messages.borrow(), cfg, state, target, scale);
    if let Some((source, start)) = ui.pulse.get() {
        let elapsed = start.elapsed().as_millis();
        if elapsed < 192
            && cfg.notifications.animate_new_message
            && cfg.animation.enabled
            && !cfg.animation.reduced_motion
            && cfg.animation.performance != PerformanceMode::Performance
        {
            if let Some(panel) = panel.as_mut() {
                panel.pulse = Some((source, (8 - elapsed / 24) as u8));
            }
        } else {
            stop_pulse(ui, hwnd);
        }
    }
    if renderer.panel.as_ref().is_some_and(|old| {
        old.contains_preview
            && panel
                .as_ref()
                .is_none_or(|p| !p.contains_preview || p.captions != old.captions)
    }) {
        renderer.purge_previews()?;
    }
    renderer.panel = panel;
    Ok(())
}

pub(super) fn redraw(ui: &Context, hwnd: HWND) -> Result<()> {
    let frame = ui.frame.borrow();
    let Some(f) = frame.as_ref() else {
        stop_pulse(ui, hwnd);
        return Ok(());
    };
    let mut render = ui.renderer.borrow_mut();
    let Some(r) = render.as_mut() else {
        return Ok(());
    };
    if ui.dragging.get() {
        // The OS owns geometry during SC_MOVE. Clear old pixels now, paint the new model on drop.
        stop_pulse(ui, hwnd);
        return r.purge_previews();
    }
    prepare(ui, hwnd, r, &f.cfg, f.state, f.target, f.scale)?;
    r.apply(&f.cfg, f.state, f.target, f.host, f.scale, false)?;
    let visible = r.visible_rect().unwrap_or(f.target);
    if ui
        .a11y
        .update_panel(&f.cfg, f.state, visible, f.scale, r.hit_panel())
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
