use super::{error, Context, IslandConfig, IslandState, Rect, Renderer, Result};
use crate::island::{
    config::PerformanceMode,
    media::MediaSnapshot,
    notifications::NotificationSnapshot,
    render::{Panel, PetPose},
};
use windows_sys::Win32::{
    Foundation::{HWND, SYSTEMTIME},
    System::SystemInformation::{GetLocalTime, GetTickCount},
    UI::{
        Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO},
        WindowsAndMessaging::*,
    },
};

pub(super) const MINUTE_TIMER: usize = 0x4e50;

pub(super) fn update_media(ui: &Context, hwnd: HWND, media: MediaSnapshot) -> Result<()> {
    let mut model = ui.messages.borrow_mut();
    if model.media == media {
        return Ok(());
    }
    model.media = media;
    drop(model);
    redraw(ui, hwnd)
}

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
    let weekday = ["日", "一", "二", "三", "四", "五", "六"][usize::from(local.wDayOfWeek) % 7];
    let date = format!("{}月{}日 周{}", local.wMonth, local.wDay, weekday);
    if !matches!(state, IslandState::Expanded | IslandState::Pinned)
        && ui
            .frame
            .borrow()
            .as_ref()
            .is_some_and(|f| matches!(f.state, IslandState::Expanded | IslandState::Pinned))
    {
        ui.messages.borrow_mut().focus_source = None;
    }
    let messages = ui.messages.borrow();
    renderer.panel = Panel::build(&messages, cfg, state, target, scale, &time, &date);
    if let Some(panel) = renderer.panel.as_mut() {
        let ms = u32::from(local.wSecond) * 1000 + u32::from(local.wMilliseconds);
        let mut delay = 60_000 - ms;
        let mut system_motion = 1i32;
        unsafe {
            SystemParametersInfoW(
                SPI_GETCLIENTAREAANIMATION,
                0,
                (&raw mut system_motion).cast(),
                0,
            );
        }
        let motion = system_motion != 0
            && cfg.animation.enabled
            && !cfg.animation.reduced_motion
            && cfg.animation.performance != PerformanceMode::Performance;
        if panel.pet.is_some() {
            let mut input = LASTINPUTINFO {
                cbSize: size_of::<LASTINPUTINFO>() as _,
                ..Default::default()
            };
            let idle = if !messages.media.playing && unsafe { GetLastInputInfo(&mut input) } != 0 {
                unsafe { GetTickCount() }.wrapping_sub(input.dwTime)
            } else {
                0
            };
            let phase = ms % 5600;
            let (pose, next) = if idle >= 180_000 {
                (PetPose::Sleeping, 1500)
            } else if idle >= 45_000 {
                (PetPose::Dozing, (180_000 - idle).min(1500))
            } else if !motion {
                (PetPose::Awake, 45_000 - idle)
            } else if phase >= 5420 {
                (PetPose::Blink, 5600 - phase)
            } else {
                (PetPose::Awake, (5420 - phase).min(45_000 - idle))
            };
            panel.pet_pose = pose;
            delay = delay.min(next);
        }
        if motion && panel.waveform.is_some() && messages.media.playing {
            panel.wave_phase = (ms / 160 % 32 + 1) as u8;
            delay = delay.min(160 - ms % 160);
        }
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
