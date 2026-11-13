use super::{Display, IslandConfig, IslandState, Rect, Result, Surface};
use crate::island::{
    config::{Anchor, IslandShape, PerformanceMode, Theme},
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
    let system_light = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .and_then(|k| k.get_value::<u32, _>("AppsUseLightTheme"))
        .unwrap_or(1)
        != 0;
    let mut cfg = IslandConfig::default();
    cfg.appearance.opacity = 1.0;
    cfg.animation.performance = PerformanceMode::Quality;
    cfg.animation.expand_ms = 100;
    cfg.animation.collapse_ms = 100;
    cfg.animation.content_ms = 80;
    let classic = cfg.appearance.background.clone();
    for (anchor, shape, theme, background, expected_bg, pin) in [
        (
            Anchor::TopCenter,
            IslandShape::Notch,
            Theme::Dark,
            classic.as_str(),
            0x000000,
            true,
        ),
        (
            Anchor::TopCenter,
            IslandShape::Pill,
            Theme::Light,
            "#f5f5f4",
            0xf5f5f4,
            true,
        ),
        (
            Anchor::CenterRight,
            IslandShape::Notch,
            Theme::System,
            "#202326",
            if system_light { 0xf5f5f4 } else { 0x202326 },
            true,
        ),
        (
            Anchor::CenterLeft,
            IslandShape::Notch,
            Theme::Dark,
            "#202326",
            0x202326,
            false,
        ),
    ] {
        cfg.placement.anchor = anchor;
        cfg.appearance.shape = shape;
        let mut cfg = screen.content_config(&cfg, &MediaSnapshot::default(), 0);
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
            || match anchor {
                Anchor::TopCenter => actual.y != compact.y,
                Anchor::CenterRight => actual.x + actual.width != compact.x + compact.width,
                Anchor::CenterLeft => actual.x != compact.x,
                _ => unreachable!(),
            }
        {
            return Err(format!(
                "{anchor:?} 展开区域不符：{compact:?} → {actual:?}，预期 {expected:?}"
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
        println!("appearance {anchor:?} {shape:?} {theme:?} {compact:?} → {actual:?} RGB #{bg:06x}, controls {controls:?}");
    }
    // A known cover and timeline let the probe distinguish content from the island background.
    let track = MediaSnapshot {
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
    };
    cfg.placement.anchor = Anchor::TopCenter;
    window.set_media(track.clone())?;
    for lyrics in [true, false] {
        cfg.appearance.lyrics_enabled = lyrics;
        let layout = screen.content_config(&cfg, &track, 0);
        window.apply(&layout, IslandState::Expanded, screen)?;
        thread::sleep(Duration::from_millis(200));
        let area = window.call(region)?;
        window.call(move |hwnd| {
            let sample = |x, y| {
                pixel(
                    hwnd,
                    POINT {
                        x: area.x + area.width / 2 + area.height * (x - 210) / 140,
                        y: area.y + area.height * y / 140,
                    },
                )
            };
            let (track_x, track_y) = if lyrics { (88, 82) } else { (155, 93) };
            if sample(75, 44)? != 0x2486a0 || sample(track_x, track_y)? != 0xffffff {
                return Err("音乐封面或已播放进度没有出现在预期位置".into());
            }
            let mut regions = vec![("顶栏", 144, 9, 124, 20)];
            if lyrics {
                regions.extend([
                    ("曲名", 99, 28, 69, 16),
                    ("歌手", 99, 44, 65, 15),
                    ("暂无歌词", 246, 53, 100, 18),
                ]);
            } else {
                regions.extend([("曲名", 121, 39, 69, 17), ("歌手", 121, 56, 65, 15)]);
            }
            for (name, x, y, w, h) in regions {
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
    }
    let power = MediaSnapshot {
        battery: Some((78, true)),
        ..Default::default()
    };
    window.set_media(power.clone())?;
    let layout = screen.content_config(&cfg, &power, 0);
    window.apply(&layout, IslandState::Compact, screen)?;
    thread::sleep(Duration::from_millis(220));
    let actual = window.call(region)?;
    let model = crate::island::render::Messages {
        media: power,
        ..Default::default()
    };
    let panel = crate::island::render::Panel::build(
        &model,
        &cfg,
        IslandState::Compact,
        actual,
        screen.scale_factor,
        "12:00",
        "",
    )
    .ok_or("充电常驻栏缺失")?;
    let (battery, _, _) = panel.battery.ok_or("充电环缺失")?;
    let (cx, cy) = (
        battery.x + battery.width / 2,
        battery.y + battery.height / 2,
    );
    let (w, h) = panel.size;
    window.call(move |hwnd| {
        for x in [cx, cx + 8] {
            let c = pixel(
                hwnd,
                POINT {
                    x: actual.x + actual.width * x / w,
                    y: actual.y + actual.height * cy / h,
                },
            )?;
            let green = (c >> 8) & 255;
            if green < 120 || green < 2 * (c & 255) {
                return Err(format!("充电闪电或进度环缺失，采样颜色 #{c:06x}"));
            }
        }
        Ok(())
    })?;
    window.set_media(MediaSnapshot::default())?;
    Ok(())
}
