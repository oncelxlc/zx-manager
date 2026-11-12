use super::{Display, IslandConfig, IslandState, Rect, Result, Surface};
use crate::island::config::{Anchor, ExpandDirection, PerformanceMode, Theme};
use std::{ptr::null_mut, thread, time::Duration};
use windows::{
    core::Interface,
    Win32::{
        Foundation::HWND as ComHwnd,
        UI::{
            Accessibility::{AccessibleObjectFromWindow, IAccessible},
            WindowsAndMessaging::OBJID_CLIENT,
        },
    },
};
use windows_sys::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::Gdi::{GetDC, GetPixel, GetWindowRgnBox, ReleaseDC},
    UI::WindowsAndMessaging::{GetWindowRect, WindowFromPhysicalPoint},
};
use winreg::{enums::HKEY_CURRENT_USER, RegKey};

fn region(hwnd: HWND) -> Result<Rect> {
    unsafe {
        let (mut host, mut shape) = (RECT::default(), RECT::default());
        if GetWindowRect(hwnd, &mut host) == 0 || GetWindowRgnBox(hwnd, &mut shape) == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(Rect {
            x: host.left + shape.left,
            y: host.top + shape.top,
            width: shape.right - shape.left,
            height: shape.bottom - shape.top,
        })
    }
}

fn pixel(hwnd: HWND, point: POINT) -> Result<u32> {
    unsafe {
        if WindowFromPhysicalPoint(point) != hwnd {
            return Err("待采样的岛屿像素被其他窗口遮挡".into());
        }
        let dc = GetDC(null_mut());
        if dc.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let bgr = GetPixel(dc, point.x, point.y);
        ReleaseDC(null_mut(), dc);
        if bgr == u32::MAX {
            return Err("读取测试像素失败".into());
        }
        Ok(((bgr & 255) << 16) | (bgr & 0xff00) | (bgr >> 16))
    }
}

fn buttons(hwnd: HWND) -> Result<i32> {
    unsafe {
        let mut object = None;
        AccessibleObjectFromWindow(
            ComHwnd(hwnd),
            OBJID_CLIENT.0 as u32,
            &IAccessible::IID,
            (&raw mut object).cast(),
        )
        .map_err(|e| e.to_string())?;
        let node: IAccessible = object.ok_or("岛屿辅助功能对象缺失")?;
        node.accChildCount().map_err(|e| e.to_string())
    }
}

pub(super) fn verify(window: &Surface, screen: &Display) -> Result<()> {
    use ExpandDirection::{Down, Left, Right, Up};
    let system_light = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .and_then(|k| k.get_value::<u32, _>("AppsUseLightTheme"))
        .unwrap_or(1)
        != 0;
    let mut cfg = IslandConfig::default();
    cfg.placement.anchor = Anchor::Center;
    cfg.appearance.opacity = 1.0;
    cfg.animation.performance = PerformanceMode::Quality;
    cfg.animation.expand_ms = 100;
    cfg.animation.collapse_ms = 100;
    cfg.animation.content_ms = 80;
    for (direction, theme, pin) in [
        (Up, Theme::Dark, true),
        (Down, Theme::Light, true),
        (Left, Theme::System, true),
        (Right, Theme::Dark, false),
    ] {
        cfg.placement.expand_direction = direction;
        cfg.appearance.theme = theme;
        cfg.appearance.background = if theme == Theme::Light {
            "#f5f5f4"
        } else {
            "#202326"
        }
        .into();
        window.apply(&cfg, IslandState::Compact, screen)?;
        thread::sleep(Duration::from_millis(180));
        let compact = window.call(region)?;
        let state = if pin {
            IslandState::Pinned
        } else {
            IslandState::Expanded
        };
        window.apply(&cfg, state, screen)?;
        thread::sleep(Duration::from_millis(220));
        let actual = window.call(region)?;
        let expected = screen.target(&cfg, state);
        if actual != expected
            || match direction {
                Up => actual.y + actual.height != compact.y + compact.height,
                Down => actual.y != compact.y,
                Left => actual.x + actual.width != compact.x + compact.width,
                Right => actual.x != compact.x,
                _ => unreachable!(),
            }
        {
            return Err(format!(
                "{direction:?} 展开区域不符：{compact:?} → {actual:?}，预期 {expected:?}"
            ));
        }
        let body = POINT {
            x: actual.x + actual.width * 3 / 4,
            y: actual.y + actual.height / 2,
        };
        let bg = window.call(move |hwnd| pixel(hwnd, body))?;
        let light = theme == Theme::Light || theme == Theme::System && system_light;
        let expected_bg = if light { 0xf5f5f4 } else { 0x202326 };
        let count = window.call(buttons)?;
        if bg != expected_bg || count != 0 {
            return Err(format!("{theme:?} 实际背景 #{bg:06x}，按钮 {count}"));
        }
        println!("appearance {direction:?} {theme:?} {compact:?} → {actual:?} RGB #{bg:06x}, buttons {count}");
    }
    Ok(())
}
