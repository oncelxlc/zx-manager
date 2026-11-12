use super::{error, Display, IslandConfig, IslandState, Rect, Result, Surface, SurfaceEvent};
mod appearance;
use std::{mem::size_of, sync::mpsc, thread, time::Duration};
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
    Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, MonitorFromPoint, HDC, HMONITOR, MONITORINFO,
        MONITOR_DEFAULTTOPRIMARY,
    },
    UI::{
        HiDpi::{GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI},
        WindowsAndMessaging::*,
    },
};

fn accessible_expand(hwnd: HWND) -> Result<()> {
    unsafe {
        let mut object = None;
        AccessibleObjectFromWindow(
            ComHwnd(hwnd),
            OBJID_CLIENT.0 as u32,
            &IAccessible::IID,
            (&raw mut object).cast(),
        )
        .map_err(|e| e.to_string())?;
        let node: IAccessible = object.ok_or("未获得岛屿辅助功能对象")?;
        if node.accChildCount().map_err(|e| e.to_string())? != 1 {
            return Err("收起态辅助功能控件数量错误".into());
        }
        let body = VARIANT::from(1);
        if node.get_accName(&body).map_err(|e| e.to_string())? != "展开灵动岛" {
            return Err("收起态辅助功能名称错误".into());
        }
        node.accDoDefaultAction(&body).map_err(|e| e.to_string())
    }
}

fn foreground_at(point: POINT) -> HWND {
    unsafe { WindowFromPhysicalPoint(point) }
}

fn verify_displays(surface: &Surface, primary: &Display) -> Result<()> {
    unsafe extern "system" fn collect(
        monitor: HMONITOR,
        _: HDC,
        _: *mut RECT,
        param: isize,
    ) -> i32 {
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let (mut x, mut y) = (0, 0);
        if GetMonitorInfoW(monitor, &mut info) != 0
            && info.dwFlags & MONITORINFOF_PRIMARY == 0
            && GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut x, &mut y) >= 0
        {
            (*(param as *mut Vec<Display>)).push(Display {
                id: "secondary".into(),
                name: "secondary".into(),
                primary: false,
                work: Rect {
                    x: info.rcWork.left,
                    y: info.rcWork.top,
                    width: info.rcWork.right - info.rcWork.left,
                    height: info.rcWork.bottom - info.rcWork.top,
                },
                scale_factor: f64::from(x) / 96.0,
            });
        }
        1
    }
    let mut cfg = IslandConfig::default();
    cfg.animation.enabled = false;
    let screens = surface.call(|_| {
        let mut screens: Vec<Display> = Vec::new();
        if unsafe {
            EnumDisplayMonitors(
                std::ptr::null_mut(),
                std::ptr::null(),
                Some(collect),
                (&raw mut screens) as isize,
            )
        } == 0
        {
            return Err(error("枚举副显示器失败"));
        }
        Ok(screens)
    })?;
    for screen in screens {
        for state in [
            IslandState::Compact,
            IslandState::Expanded,
            IslandState::EdgePeek,
        ] {
            surface.apply(&cfg, state, &screen)?;
            if surface.outer_rect()? != screen.target(&cfg, state) {
                return Err("副屏定位错误".into());
            }
        }
        let dpi = surface.call(|hwnd| Ok(unsafe { GetDpiForWindow(hwnd) }))?;
        if dpi != (screen.scale_factor * 96.0).round() as u32 {
            return Err("副屏 HWND DPI 不符".into());
        }
        println!("secondary work {:?}, actual DPI {dpi}", screen.work);
    }
    for scale in [1.25, 1.5, 2.0] {
        let mut screen = primary.clone();
        screen.scale_factor = scale;
        surface.apply(&cfg, IslandState::Expanded, &screen)?;
        if surface.outer_rect()? != screen.target(&cfg, IslandState::Expanded) {
            return Err("模拟 DPI 矩形错误".into());
        }
    }
    Ok(())
}

/// Standalone diagnostic; does not start Tauri, install hooks or load a user's configuration.
pub fn run_poc() -> Result<()> {
    let (events, input) = mpsc::channel();
    let window = Surface::start(move |event| {
        let _ = events.send(event);
    })?;
    let (raw, screen) = window.call(|hwnd| unsafe {
        let monitor = MonitorFromPoint(POINT::default(), MONITOR_DEFAULTTOPRIMARY);
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return Err(error("读取测试显示器失败"));
        }
        let style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        let required =
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_NOREDIRECTIONBITMAP;
        if style & required != required {
            return Err("原生窗口样式与预期不符".into());
        }
        Ok((
            hwnd as usize,
            Display {
                id: "primary".into(),
                name: "primary".into(),
                primary: true,
                scale_factor: f64::from(GetDpiForWindow(hwnd)) / 96.0,
                work: Rect {
                    x: info.rcWork.left,
                    y: info.rcWork.top,
                    width: info.rcWork.right - info.rcWork.left,
                    height: info.rcWork.bottom - info.rcWork.top,
                },
            },
        ))
    })?;
    let cfg = IslandConfig::default();
    let compact = screen.target(&cfg, IslandState::Compact);
    let expanded = screen.target(&cfg, IslandState::Expanded);
    let corner = POINT {
        x: compact.x,
        y: compact.y,
    };
    let envelope = POINT {
        x: expanded.x + 4,
        y: expanded.y + expanded.height / 2,
    };
    let beneath_corner = foreground_at(corner);
    let beneath_transition = foreground_at(envelope);
    let mut pid = 0;
    if beneath_transition.is_null()
        || unsafe { GetWindowThreadProcessId(beneath_transition, &mut pid) } == 0
        || pid == std::process::id()
    {
        return Err("缺少可供验证透传的外部窗口".into());
    }
    window.apply(&cfg, IslandState::Compact, &screen)?;
    if foreground_at(corner) != beneath_corner {
        return Err("稳定态的圆角遮住下层窗口".into());
    }
    let down = |v: i32| v as u16 as u32;
    let point = down(compact.x + compact.width / 2) | (down(compact.y + compact.height / 2) << 16);
    let hit = window
        .call(move |hwnd| Ok(unsafe { SendMessageW(hwnd, WM_NCHITTEST, 0, point as isize) }))?;
    if hit != HTCLIENT as isize {
        return Err("收起态中心未能命中".into());
    }
    let local_minute = || {
        let mut time = windows_sys::Win32::Foundation::SYSTEMTIME::default();
        unsafe { windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut time) };
        time.wMinute
    };
    let minute = local_minute();
    let idle = window.stats()?;
    thread::sleep(Duration::from_millis(500));
    let after_idle = window.stats()?;
    if idle.presents == 0
        || after_idle.presents.saturating_sub(idle.presents) > u64::from(minute != local_minute())
        || idle.commits != after_idle.commits
    {
        return Err("空闲合成提交或绘制计数异常".into());
    }
    for (state, delay) in [
        (IslandState::Expanded, 65),
        (IslandState::Compact, 65),
        (IslandState::Pinned, 350),
        (IslandState::EdgePeek, 300),
        (IslandState::Compact, 300),
    ] {
        window.apply(&cfg, state, &screen)?;
        if state == IslandState::Expanded {
            let rect = window.outer_rect()?;
            if rect.y + rect.height <= envelope.y {
                if foreground_at(envelope) != beneath_transition {
                    return Err("过渡期间透明包络命中了岛屿窗口".into());
                }
                println!("Cross-process hit test passed during expansion");
            } else {
                println!("Cross-process mid-animation hit test skipped: transition already passed sample point");
            }
        }
        thread::sleep(Duration::from_millis(delay));
        if state == IslandState::Compact && delay == 65 {
            let origin = window.prepare_drag()?;
            thread::sleep(Duration::from_millis(45));
            if window.outer_rect()? != origin {
                return Err("开始拖动后几何仍在运动".into());
            }
        }
        if state == IslandState::Compact && delay == 300 {
            window.call(accessible_expand)?;
        }
    }
    if window.outer_rect()? != compact {
        return Err("反向动画未收敛到目标位置".into());
    }
    verify_displays(&window, &screen)?;
    appearance::verify(&window, &screen)?;
    window.hide()?;
    let stats = window.stats()?;
    window.shutdown()?;
    if unsafe { IsWindow(raw as HWND) } != 0 || window.call(|_| Ok(())).is_ok() {
        return Err("原生窗口或消息泵未释放".into());
    }
    let mut expanded = false;
    for event in input.try_iter() {
        expanded |= event == SurfaceEvent::Click;
        if let SurfaceEvent::Error(e) = event {
            return Err(e);
        }
    }
    if !expanded {
        return Err("读屏按钮未发出展开操作".into());
    }
    println!(
        "Native Island PoC: {}",
        serde_json::to_string(&stats).map_err(|e| e.to_string())?
    );
    Ok(())
}
