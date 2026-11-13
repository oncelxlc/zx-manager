use super::*;
use crate::island::{
    config::{Anchor, IslandConfig, IslandShape},
    state::IslandState,
    surface::Surface,
};
use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Graphics::Gdi::{EnumDisplayMonitors, HDC, HMONITOR},
    UI::HiDpi::{
        SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT,
        DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    },
    UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
        MOUSEINPUT,
    },
};

fn left_button(flags: u32) -> bool {
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dwFlags: flags,
                ..Default::default()
            },
        },
    };
    unsafe { SendInput(1, &input, size_of::<INPUT>() as i32) == 1 }
}

struct Pointer(POINT, bool, DPI_AWARENESS_CONTEXT);

impl Drop for Pointer {
    fn drop(&mut self) {
        if self.1 {
            left_button(MOUSEEVENTF_LEFTUP);
        }
        unsafe {
            SetCursorPos(self.0.x, self.0.y);
            SetThreadDpiAwarenessContext(self.2);
        }
    }
}

fn wait_for(mut condition: impl FnMut() -> bool) -> bool {
    let until = Instant::now() + Duration::from_secs(2);
    while !condition() {
        if Instant::now() >= until {
            return false;
        }
        thread::sleep(Duration::from_millis(10));
    }
    true
}

#[test]
#[ignore = "moves the pointer on the interactive Windows desktop, restoring it afterwards"]
fn held_native_drag_visits_each_work_area_edge_before_release() {
    for shape in [IslandShape::Notch, IslandShape::Pill] {
        exercise_shape(shape);
    }
}

fn exercise_shape(shape: IslandShape) {
    unsafe extern "system" fn collect(
        monitor: HMONITOR,
        _: HDC,
        _: *mut RECT,
        list: LPARAM,
    ) -> i32 {
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut info) != 0 {
            let r = info.rcMonitor;
            (*(list as *mut Vec<Rect>)).push(
                Rect {
                    x: r.left,
                    y: r.top,
                    width: r.right - r.left,
                    height: r.bottom - r.top,
                }
                .placement_bounds(info.rcWork.top),
            );
        }
        1
    }
    let (start_drag, drag_requested) = mpsc::channel();
    let window = Surface::start(move |e| {
        if matches!(e, crate::island::surface::SurfaceEvent::DragStart) {
            let _ = start_drag.send(());
        }
    })
    .unwrap();
    let (send, end) = mpsc::channel();
    window
        .install_watch(move |e| {
            if matches!(e, NativeEvent::DragEnded) {
                let _ = send.send(());
            }
        })
        .unwrap();
    let (hwnd, screens, start) = window
        .call(|hwnd| {
            let mut screens: Vec<Rect> = Vec::new();
            let mut p = POINT::default();
            unsafe {
                if EnumDisplayMonitors(
                    null_mut(),
                    std::ptr::null(),
                    Some(collect),
                    (&raw mut screens) as LPARAM,
                ) == 0
                    || GetCursorPos(&mut p) == 0
                    || GetAsyncKeyState(i32::from(VK_LBUTTON)) < 0
                {
                    return Err("无法独占测试鼠标".into());
                }
            }
            Ok((hwnd as usize, screens, p))
        })
        .unwrap();
    let hwnd = hwnd as HWND;
    let old_dpi =
        unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    let mut restore = Pointer(start, false, old_dpi);
    let display = Display {
        id: "probe".into(),
        name: "probe".into(),
        primary: true,
        scale_factor: 1.0,
        work: screens[0],
    };
    let mut cfg = IslandConfig::default();
    cfg.appearance.shape = shape;
    cfg.placement.offset_x = -24.0;
    window.apply(&cfg, IslandState::Compact, &display).unwrap();
    let original = window.outer_rect().unwrap();
    let (dx, dy) = (original.width / 2, original.height / 2);
    let grab = POINT {
        x: original.x + dx,
        y: original.y + dy,
    };
    assert_eq!(unsafe { WindowFromPhysicalPoint(grab) }, hwnd);
    assert_ne!(unsafe { SetCursorPos(grab.x, grab.y) }, 0);
    assert!(left_button(MOUSEEVENTF_LEFTDOWN));
    restore.1 = true;
    assert!(
        wait_for(|| window
            .call(|hwnd| Ok(unsafe { GetCapture() == hwnd }))
            .unwrap()),
        "left press did not reach the test island"
    );
    assert_ne!(unsafe { SetCursorPos(grab.x + 12, grab.y) }, 0);
    drag_requested
        .recv_timeout(Duration::from_secs(2))
        .expect("island did not request a drag after pointer motion");
    window.begin_drag().unwrap();
    assert!(
        wait_for(|| window
            .call(|_| Ok(WATCH.with_borrow(|w| w.as_ref().is_some_and(|w| w.drag_grip.is_some()))))
            .unwrap()),
        "move loop did not start, held={}, ended={:?}",
        unsafe { GetAsyncKeyState(i32::from(VK_LBUTTON)) },
        end.try_recv()
    );
    for w in &screens {
        let mut dpi = 96;
        let mut dpi_y = 96;
        unsafe {
            GetDpiForMonitor(
                MonitorFromPoint(
                    POINT {
                        x: w.x + w.width / 2,
                        y: w.y + w.height / 2,
                    },
                    MONITOR_DEFAULTTONEAREST,
                ),
                MDT_EFFECTIVE_DPI,
                &mut dpi,
                &mut dpi_y,
            );
        }
        let gap = if shape == IslandShape::Pill {
            (6.0 * f64::from(dpi) / 96.0).round() as i32
        } else {
            0
        };
        for (x, y, edge) in [
            (w.width / 2, 8, Anchor::TopCenter),
            (w.width - 8, w.height / 2, Anchor::CenterRight),
            (8, w.height / 2, Anchor::CenterLeft),
        ] {
            assert_ne!(unsafe { SetCursorPos(w.x + x, w.y + y) }, 0);
            assert!(
                wait_for(|| {
                    let r = window.outer_rect().unwrap();
                    match edge {
                        Anchor::TopCenter => r.y == w.y + gap && r.width > r.height,
                        Anchor::CenterRight => {
                            r.x + r.width == w.x + w.width - gap && r.height > r.width
                        }
                        _ => r.x == w.x + gap && r.height > r.width,
                    }
                }),
                "while held: expected {edge:?} on {w:?}, got {:?}",
                window.outer_rect()
            );
            assert!(unsafe { GetAsyncKeyState(i32::from(VK_LBUTTON)) < 0 });
        }
    }
    assert!(left_button(MOUSEEVENTF_LEFTUP));
    restore.1 = false;
    end.recv_timeout(Duration::from_secs(2)).unwrap();
    window.uninstall_watch().unwrap();
    window.shutdown().unwrap();
    println!(
        "Verified {shape:?} held drag at all three docks of {} monitor(s): {screens:?}",
        screens.len()
    );
}
