use super::{os_error, watch::notify, NativeEvent};
use crate::island::{config::AnimationPreset, geometry::Rect};
use std::{
    cell::Cell,
    ptr::null_mut,
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::{HWND, RECT},
    Graphics::Gdi::{CreateRoundRectRgn, DeleteObject, SetWindowRgn},
    UI::WindowsAndMessaging::{
        GetWindowRect, IsWindowVisible, KillTimer, SetTimer, SetWindowPos, ShowWindow,
        SystemParametersInfoW, SPI_GETCLIENTAREAANIMATION, SWP_NOACTIVATE, SWP_NOZORDER, SW_HIDE,
        SW_SHOWNOACTIVATE,
    },
};

const FRAME_TIMER: usize = 0x4953;

#[derive(Clone, Copy)]
struct Motion {
    hwnd: HWND,
    from: Rect,
    to: Rect,
    started: Instant,
    duration: Duration,
    radius: i32,
    preset: AnimationPreset,
    amplitude: f64,
}

thread_local! { static MOTION: Cell<Option<Motion>> = const { Cell::new(None) }; }

fn animations_allowed() -> bool {
    let mut enabled: i32 = 1;
    unsafe {
        SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, (&raw mut enabled).cast(), 0) == 0
            || enabled != 0
    }
}

fn current(hwnd: HWND) -> Result<Rect, String> {
    let mut r = RECT::default();
    if unsafe { GetWindowRect(hwnd, &mut r) } == 0 {
        return Err(os_error("获取窗口位置失败"));
    }
    Ok(Rect {
        x: r.left,
        y: r.top,
        width: r.right - r.left,
        height: r.bottom - r.top,
    })
}

fn frame(hwnd: HWND, r: Rect, radius: i32) -> Result<(), String> {
    unsafe {
        if SetWindowPos(
            hwnd,
            null_mut(),
            r.x,
            r.y,
            r.width,
            r.height,
            SWP_NOACTIVATE | SWP_NOZORDER,
        ) == 0
        {
            return Err(os_error("设置窗口位置和尺寸失败"));
        }
        // Region also clips native hit testing at the transparent capsule corners.
        let diameter = (2 * radius).min(r.width).min(r.height).max(1);
        let region = CreateRoundRectRgn(0, 0, r.width + 1, r.height + 1, diameter, diameter);
        if region.is_null() {
            return Err(os_error("创建窗口边界失败"));
        }
        if SetWindowRgn(hwnd, region, 1) == 0 {
            let err = os_error("设置窗口边界失败");
            DeleteObject(region);
            return Err(err);
        }
        // Windows owns the region after successful SetWindowRgn.
    }
    Ok(())
}

pub(in crate::island) fn cancel(hwnd: HWND) {
    if MOTION.take().is_some() {
        unsafe {
            KillTimer(hwnd, FRAME_TIMER);
        }
    }
}

pub(in crate::island) fn hide(hwnd: HWND) {
    cancel(hwnd);
    unsafe {
        ShowWindow(hwnd, SW_HIDE);
    }
}

pub(in crate::island) fn apply(
    hwnd: HWND,
    target: Rect,
    radius: i32,
    ms: u32,
    preset: AnimationPreset,
    amplitude: f64,
    transition: bool,
) -> Result<(), String> {
    let allowed = ms != 0 && animations_allowed();
    if allowed
        && MOTION.get().is_some_and(|m| {
            m.hwnd == hwnd
                && m.to == target
                && m.radius == radius
                && m.preset == preset
                && m.amplitude == amplitude
        })
    {
        return Ok(());
    }
    cancel(hwnd);
    let visible = unsafe { IsWindowVisible(hwnd) != 0 };
    let from = current(hwnd)?;
    if !visible || !transition || from == target || !allowed {
        frame(hwnd, target, radius)?;
        if !visible {
            unsafe {
                ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            }
        }
        return Ok(());
    }
    MOTION.set(Some(Motion {
        hwnd,
        from,
        to: target,
        started: Instant::now(),
        duration: Duration::from_millis(ms.into()),
        radius,
        preset,
        amplitude,
    }));
    if unsafe { SetTimer(hwnd, FRAME_TIMER, 16, Some(tick)) } == 0 {
        let error = os_error("创建窗口动画定时器失败");
        MOTION.set(None);
        frame(hwnd, target, radius)?;
        return Err(error);
    }
    Ok(())
}

unsafe extern "system" fn tick(hwnd: HWND, _: u32, _: usize, _: u32) {
    let Some(animation) = MOTION.get() else {
        return;
    };
    if animation.hwnd != hwnd {
        return;
    }
    let t = (animation.started.elapsed().as_secs_f64() / animation.duration.as_secs_f64()).min(1.0);
    let eased = match animation.preset {
        AnimationPreset::Subtle => 1.0 - (1.0 - t).powi(2),
        AnimationPreset::Spring => {
            // Critically damp the HWND. Decorative spring overshoot belongs inside its hit area.
            let k = 6.0 + animation.amplitude * 2.0;
            (1.0 - (1.0 + k * t) * (-k * t).exp()) / (1.0 - (1.0 + k) * (-k).exp())
        }
        AnimationPreset::Smooth | AnimationPreset::Custom => t * t * (3.0 - 2.0 * t),
    };
    let next = if t >= 1.0 {
        animation.to
    } else {
        animation.from.interpolate(animation.to, eased)
    };
    if t >= 1.0 {
        cancel(hwnd);
    }
    if let Err(error) = frame(hwnd, next, animation.radius) {
        cancel(hwnd);
        notify(NativeEvent::Error(error));
    }
}
