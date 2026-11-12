use super::{Display, IslandConfig, IslandState, Rect, Result, Surface};
use crate::island::{
    config::{Anchor, ExpandDirection, Orientation, PerformanceMode, Theme},
    media::MediaSnapshot,
    notifications::AppIcon,
};
use std::{ptr::null_mut, thread, time::Duration};
use windows::{
    core::Interface,
    Win32::{
        Foundation::HWND as ComHwnd,
        System::Variant::VARIANT,
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

fn buttons(hwnd: HWND) -> Result<Vec<String>> {
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
        let count = node.accChildCount().map_err(|e| e.to_string())?;
        (1..=count)
            .map(|id| {
                node.get_accName(&VARIANT::from(id))
                    .map(|name| name.to_string())
                    .map_err(|e| e.to_string())
            })
            .collect()
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
    cfg.placement.orientation = Orientation::Horizontal;
    cfg.appearance.opacity = 1.0;
    cfg.animation.performance = PerformanceMode::Quality;
    cfg.animation.expand_ms = 100;
    cfg.animation.collapse_ms = 100;
    cfg.animation.content_ms = 80;
    let classic = cfg.appearance.background.clone();
    for (direction, theme, background, expected_bg, pin) in [
        (Up, Theme::Dark, classic.as_str(), 0x000000, true),
        (Down, Theme::Light, "#f5f5f4", 0xf5f5f4, true),
        (
            Left,
            Theme::System,
            "#202326",
            if system_light { 0xf5f5f4 } else { 0x202326 },
            true,
        ),
        (Right, Theme::Dark, "#202326", 0x202326, false),
    ] {
        cfg.placement.expand_direction = direction;
        cfg.appearance.theme = theme;
        cfg.appearance.background = background.into();
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
            x: actual.x + actual.width * 3 / 5,
            y: actual.y + actual.height * 4 / 7,
        };
        let bg = window.call(move |hwnd| pixel(hwnd, body))?;
        let controls = window.call(buttons)?;
        if bg != expected_bg
            || controls
                != [
                    "打开灵动岛设置",
                    "播放进度",
                    "歌词",
                    "上一首",
                    "播放",
                    "下一首",
                    "音乐应用音量",
                ]
        {
            return Err(format!("{theme:?} 实际背景 #{bg:06x}，控件 {controls:?}"));
        }
        println!("appearance {direction:?} {theme:?} {compact:?} → {actual:?} RGB #{bg:06x}, controls {controls:?}");
    }
    // A known cover and timeline let the probe distinguish content from the island background.
    window.set_media(MediaSnapshot {
        title: "Test Track".into(),
        artist: "Test Artist".into(),
        cover: Some(std::sync::Arc::new(AppIcon {
            width: 1,
            height: 1,
            bgra: vec![160, 134, 36, 255],
        })),
        position_ms: 60_000,
        duration_ms: 120_000,
        can_toggle: true,
        can_seek: true,
        ..Default::default()
    })?;
    thread::sleep(Duration::from_millis(200));
    let area = window.call(region)?;
    window.call(move |hwnd| {
        let sample = |x, y| {
            pixel(
                hwnd,
                POINT {
                    x: area.x + area.width * x / 419,
                    y: area.y + area.height * y / 140,
                },
            )
        };
        if sample(75, 44)? != 0x2486a0 || sample(148, 93)? != 0xffffff {
            return Err("音乐封面或已播放进度没有出现在预期位置".into());
        }
        for (name, x, y, w, h) in [
            ("顶栏", 153, 9, 75, 20),
            ("曲名", 121, 36, 69, 19),
            ("歌手", 121, 57, 65, 15),
        ] {
            let mut found = false;
            'rows: for row in (y..y + h).step_by(2) {
                for column in (x..x + w).step_by(2) {
                    if sample(column, row)? & 255 > 110 {
                        found = true;
                        break 'rows;
                    }
                }
            }
            if !found {
                return Err(format!("音乐页{name}区域没有渲染文字"));
            }
        }
        Ok(())
    })?;
    window.set_media(MediaSnapshot::default())?;
    Ok(())
}
