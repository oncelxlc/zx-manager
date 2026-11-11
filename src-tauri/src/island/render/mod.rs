mod animation;
mod device;
mod paint;

use self::{
    animation::{now, Movement, Timing},
    device::{Device, Texture},
    paint::ImageKey,
};
use super::{
    config::{AnimationConfig, IslandConfig, PerformanceMode},
    geometry::Rect,
    state::{Event, IslandState},
};
pub use paint::{action_at, action_rect};
use serde::Serialize;
use std::time::Instant;
use windows::{
    core::{Error, Result as WinResult},
    Win32::{
        Foundation::{D2DERR_RECREATE_TARGET, HWND},
        Graphics::Dxgi::{
            DXGI_ERROR_DEVICE_HUNG, DXGI_ERROR_DEVICE_REMOVED, DXGI_ERROR_DEVICE_RESET,
            DXGI_ERROR_DRIVER_INTERNAL_ERROR,
        },
    },
};
use windows_sys::Win32::{
    System::Performance::QueryPerformanceFrequency,
    UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETCLIENTAREAANIMATION},
};

#[derive(Clone, Copy, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderStats {
    pub presents: u64,
    pub commits: u64,
    pub recoveries: u64,
    pub last_recovery_us: u64,
    pub last_apply_us: u64,
}

#[derive(Clone, Copy)]
struct Fade {
    from: f32,
    to: f32,
    timing: Timing,
}

impl Fade {
    fn at(self, ticks: i64) -> f32 {
        self.from + (self.to - self.from) * self.timing.progress(ticks) as f32
    }
}

struct Scene {
    textures: [Option<Texture>; 4],
    device: Device,
    fades: [Option<Fade>; 4],
    movement: Option<Movement>,
    last: Option<(IslandState, Rect, AnimationConfig)>,
    frequency: i64,
}

impl Scene {
    fn valid(&self) -> bool {
        unsafe { self.device.composition.CheckDeviceState() }.is_ok_and(|v| v.as_bool())
    }

    unsafe fn new(hwnd: HWND) -> WinResult<Self> {
        let mut frequency = 0;
        if QueryPerformanceFrequency(&mut frequency) == 0 {
            return Err(Error::from_win32());
        }
        Ok(Self {
            textures: Default::default(),
            device: Device::new(hwnd)?,
            fades: [None; 4],
            movement: None,
            last: None,
            frequency,
        })
    }

    unsafe fn hide(&mut self, stats: &mut RenderStats) -> WinResult<()> {
        self.movement = None;
        self.fades = [None; 4];
        if self.last.take().is_none() {
            return Ok(());
        }
        self.device.root.SetOffsetX2(0.0)?;
        self.device.root.SetOffsetY2(0.0)?;
        for item in self.textures.iter().flatten() {
            item.fade.SetOpacity2(0.0)?;
            item.scale.SetScaleX2(1.0)?;
            item.scale.SetScaleY2(1.0)?;
            item.reset_content()?;
        }
        self.device.composition.Commit()?;
        stats.commits += 1;
        Ok(())
    }

    unsafe fn apply(&mut self, frame: &Frame<'_>, stats: &mut RenderStats) -> WinResult<()> {
        let Frame {
            cfg,
            state,
            target,
            host,
            scale,
            transition,
            hovered,
        } = *frame;
        let idx = match state {
            IslandState::Compact => 0,
            IslandState::Expanded => 1,
            IslandState::Pinned => 2,
            IslandState::EdgePeek => 3,
            _ => return self.hide(stats),
        };
        let mut key = ImageKey::new(cfg, state, target, scale);
        key.hovered = hovered;
        let dirty = self.textures[idx]
            .as_ref()
            .is_none_or(|item| item.key != key);
        if dirty {
            match &mut self.textures[idx] {
                Some(texture) => self.device.update_texture(texture, key)?,
                slot @ None => *slot = Some(self.device.create_texture(key)?),
            }
            stats.presents += 1;
        }
        let same_state = self
            .last
            .as_ref()
            .is_some_and(|(prev, _, _)| *prev == state);
        // A new buffer on the same swap chain is visible after Present. Leave its
        // existing compositor timelines running when only theme or content changed.
        if same_state
            && self.movement.is_some_and(|m| m.to == target)
            && self
                .last
                .as_ref()
                .is_some_and(|(_, prev, anim)| *prev == host && anim == &cfg.animation)
        {
            return Ok(());
        }
        let begin = now();
        let mut system_motion = 1i32;
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            (&raw mut system_motion).cast(),
            0,
        );
        let anim = &cfg.animation;
        let full = system_motion != 0
            && anim.enabled
            && !anim.reduced_motion
            && anim.performance != PerformanceMode::Performance;
        let edge = state == IslandState::EdgePeek
            || self
                .last
                .as_ref()
                .is_some_and(|(s, _, _)| *s == IslandState::EdgePeek);
        let duration = if !full || !transition || same_state {
            0
        } else if edge {
            anim.edge_ms
        } else if self.last.is_none() {
            200
        } else if matches!(state, IslandState::Expanded | IslandState::Pinned) {
            anim.expand_ms
        } else {
            anim.collapse_ms
        };
        let fade_ms = if !anim.enabled || system_motion == 0 || !transition || same_state {
            0
        } else if !full {
            anim.content_ms.min(60)
        } else {
            anim.content_ms
        };
        let timing = Timing::new(begin, self.frequency, duration, anim);
        let fade_timing = Timing::new(begin, self.frequency, fade_ms, anim);
        let from = self.movement.map(|m| m.at(begin)).unwrap_or_else(|| {
            let shrink = if full { 0.025 * anim.amplitude } else { 0.0 };
            let dx = (target.width as f64 * shrink / 2.0).round() as i32;
            let dy = (target.height as f64 * shrink / 2.0).round() as i32;
            Rect {
                x: target.x + dx,
                y: target.y + dy,
                width: (target.width - 2 * dx).max(1),
                height: (target.height - 2 * dy).max(1),
            }
        });
        let d = &self.device;
        timing.property(
            &d.composition,
            (from.x - host.x) as f32,
            (target.x - host.x) as f32,
            |n| d.root.SetOffsetX2(n),
            |a| d.root.SetOffsetX(a),
        )?;
        timing.property(
            &d.composition,
            (from.y - host.y) as f32,
            (target.y - host.y) as f32,
            |n| d.root.SetOffsetY2(n),
            |a| d.root.SetOffsetY(a),
        )?;
        for (i, item) in self.textures.iter().enumerate() {
            let Some(item) = item else {
                continue;
            };
            let alpha = self.fades[i].map(|f| f.at(begin)).unwrap_or(0.0);
            let next = if i == idx { 1.0 } else { 0.0 };
            let (w, h) = (item.key.size.0 as f32, item.key.size.1 as f32);
            if alpha > 0.0 || next > 0.0 {
                timing.property(
                    &d.composition,
                    from.width as f32 / w,
                    target.width as f32 / w,
                    |n| item.scale.SetScaleX2(n),
                    |a| item.scale.SetScaleX(a),
                )?;
                timing.property(
                    &d.composition,
                    from.height as f32 / h,
                    target.height as f32 / h,
                    |n| item.scale.SetScaleY2(n),
                    |a| item.scale.SetScaleY(a),
                )?;
                item.animate_content(&d.composition, fade_timing, anim, full, alpha, next)?;
            }
            fade_timing.property(
                &d.composition,
                alpha,
                next,
                |n| item.fade.SetOpacity2(n),
                |a| item.fade.SetOpacity(a),
            )?;
            self.fades[i] = Some(Fade {
                from: alpha,
                to: next,
                timing: fade_timing,
            });
        }
        d.composition.Commit()?;
        stats.commits += 1;
        self.movement = Some(Movement {
            from,
            to: target,
            timing,
        });
        self.last = Some((state, host, anim.clone()));
        Ok(())
    }
}

struct Frame<'a> {
    cfg: &'a IslandConfig,
    state: IslandState,
    target: Rect,
    host: Rect,
    scale: f64,
    transition: bool,
    hovered: Option<Event>,
}

fn device_lost(error: &Error) -> bool {
    matches!(
        error.code(),
        DXGI_ERROR_DEVICE_REMOVED
            | DXGI_ERROR_DEVICE_RESET
            | DXGI_ERROR_DEVICE_HUNG
            | DXGI_ERROR_DRIVER_INTERNAL_ERROR
            | D2DERR_RECREATE_TARGET
    )
}

pub struct Renderer {
    hwnd: HWND,
    scene: Option<Scene>,
    stats: RenderStats,
    hovered: Option<Event>,
}

impl Renderer {
    pub fn new(hwnd: windows_sys::Win32::Foundation::HWND) -> Result<Self, String> {
        let hwnd = HWND(hwnd);
        let scene =
            unsafe { Scene::new(hwnd) }.map_err(|e| format!("初始化岛屿图形设备失败：{e}"))?;
        Ok(Self {
            hwnd,
            scene: Some(scene),
            stats: Default::default(),
            hovered: None,
        })
    }

    pub fn valid(&self) -> bool {
        self.scene.as_ref().is_some_and(Scene::valid)
    }

    pub fn apply(
        &mut self,
        cfg: &IslandConfig,
        state: IslandState,
        target: Rect,
        host: Rect,
        scale: f64,
        transition: bool,
    ) -> Result<(), String> {
        let mut frame = Frame {
            cfg,
            state,
            target,
            host,
            scale,
            transition,
            hovered: self.hovered,
        };
        let start = Instant::now();
        let result = unsafe {
            let mut result = match self.scene.as_mut() {
                Some(s) if s.valid() => s.apply(&frame, &mut self.stats),
                _ => Err(Error::from_hresult(DXGI_ERROR_DEVICE_REMOVED)),
            };
            if result.as_ref().is_err_and(device_lost) {
                let retry = Instant::now();
                // Release the original target before binding another device to this HWND.
                self.scene = None;
                frame.transition = false;
                result = Scene::new(self.hwnd).and_then(|mut scene| {
                    scene.apply(&frame, &mut self.stats)?;
                    self.scene = Some(scene);
                    Ok(())
                });
                if result.is_ok() {
                    self.stats.recoveries += 1;
                    self.stats.last_recovery_us = retry.elapsed().as_micros() as u64;
                }
            }
            result
        };
        self.stats.last_apply_us = start.elapsed().as_micros() as u64;
        result.map_err(|e| format!("提交岛屿合成画面失败：{e}"))
    }

    pub fn hide(&mut self) -> Result<(), String> {
        self.hovered = None;
        if let Some(scene) = &mut self.scene {
            unsafe { scene.hide(&mut self.stats) }.map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn visible_rect(&self) -> Option<Rect> {
        self.scene.as_ref()?.movement.map(|m| m.at(now()))
    }

    pub fn stats(&self) -> RenderStats {
        self.stats
    }

    pub fn composition_clock(&self) -> Result<(i64, i64, i64), String> {
        let scene = self.scene.as_ref().ok_or("原生图形设备不可用")?;
        let frame =
            unsafe { scene.device.composition.GetFrameStatistics() }.map_err(|e| e.to_string())?;
        let rate = frame.currentCompositionRate;
        if rate.Numerator == 0
            || rate.Denominator == 0
            || frame.timeFrequency <= 0
            || frame.nextEstimatedFrameTime <= 0
        {
            return Err("无法取得合成器的下一帧时间".into());
        }
        let period = (frame.timeFrequency as f64 * f64::from(rate.Denominator)
            / f64::from(rate.Numerator))
        .round() as i64;
        Ok((
            frame.nextEstimatedFrameTime,
            period.max(1),
            frame.timeFrequency,
        ))
    }

    pub fn set_hovered(&mut self, action: Option<Event>) -> bool {
        std::mem::replace(&mut self.hovered, action) != action
    }

    pub fn freeze(&mut self, rect: Rect) -> Result<(), String> {
        let Some(scene) = self.scene.as_mut() else {
            return Ok(());
        };
        let Some((_, _, anim)) = scene.last.take() else {
            return Ok(());
        };
        let ticks = now();
        let timing = Timing::new(ticks, scene.frequency, 0, &anim);
        let result: WinResult<()> = (|| unsafe {
            scene.device.root.SetOffsetX2(0.0)?;
            scene.device.root.SetOffsetY2(0.0)?;
            for (item, fade) in scene.textures.iter().zip(&mut scene.fades) {
                if let (Some(item), Some(previous)) = (item, *fade) {
                    let alpha = previous.at(ticks);
                    item.fade.SetOpacity2(alpha)?;
                    item.scale
                        .SetScaleX2(rect.width as f32 / item.key.size.0 as f32)?;
                    item.scale
                        .SetScaleY2(rect.height as f32 / item.key.size.1 as f32)?;
                    item.animate_content(
                        &scene.device.composition,
                        timing,
                        &anim,
                        anim.enabled
                            && !anim.reduced_motion
                            && anim.performance != PerformanceMode::Performance,
                        alpha,
                        alpha,
                    )?;
                    *fade = Some(Fade {
                        from: alpha,
                        to: alpha,
                        timing,
                    });
                }
            }
            scene.device.composition.Commit()
        })();
        result.map_err(|e| format!("冻结岛屿动画失败：{e}"))?;
        scene.movement = Some(Movement {
            from: rect,
            to: rect,
            timing,
        });
        self.stats.commits += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Foundation::E_UNEXPECTED;
    #[test]
    fn retries_only_lost_devices_and_render_targets() {
        for hr in [
            DXGI_ERROR_DEVICE_REMOVED,
            DXGI_ERROR_DEVICE_RESET,
            D2DERR_RECREATE_TARGET,
        ] {
            assert!(device_lost(&Error::from_hresult(hr)));
        }
        assert!(!device_lost(&Error::from_hresult(E_UNEXPECTED)));
    }

    #[test]
    #[ignore = "requires an interactive Windows desktop with a hardware D3D11 adapter"]
    fn rebuilds_a_released_device_on_the_same_hwnd() {
        use std::ptr::{null, null_mut};
        use windows_sys::{
            core::w,
            Win32::{
                Foundation::HWND as RawHwnd,
                System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED},
                UI::WindowsAndMessaging::*,
            },
        };
        struct TestWindow(RawHwnd);
        impl Drop for TestWindow {
            fn drop(&mut self) {
                unsafe {
                    DestroyWindow(self.0);
                    CoUninitialize();
                }
            }
        }
        let hr = unsafe { CoInitializeEx(null(), COINIT_APARTMENTTHREADED as u32) };
        assert!(hr >= 0);
        let window = TestWindow(unsafe {
            CreateWindowExW(
                WS_EX_NOREDIRECTIONBITMAP,
                w!("STATIC"),
                w!("zx-once GPU test"),
                WS_POPUP,
                0,
                0,
                180,
                44,
                null_mut(),
                null_mut(),
                null_mut(),
                null(),
            )
        });
        assert!(!window.0.is_null());
        let mut renderer = Renderer::new(window.0).unwrap();
        let cfg = IslandConfig::default();
        let rect = Rect {
            x: 0,
            y: 0,
            width: 180,
            height: 44,
        };
        renderer
            .apply(&cfg, IslandState::Compact, rect, rect, 1.0, false)
            .unwrap();
        renderer.scene.take();
        renderer
            .apply(&cfg, IslandState::Compact, rect, rect, 1.0, false)
            .unwrap();
        let stats = renderer.stats();
        assert_eq!((stats.recoveries, stats.presents, stats.commits), (1, 2, 2));
        assert_eq!(renderer.visible_rect(), Some(rect));
        println!("同一 HWND 受控图形资源重建 {} µs", stats.last_recovery_us);
        for (step, scale) in [1.25, 1.5, 2.0].into_iter().enumerate() {
            let resized = Rect {
                width: (180.0_f64 * scale).round() as i32,
                height: (44.0_f64 * scale).round() as i32,
                ..rect
            };
            for _ in 0..2 {
                renderer
                    .apply(&cfg, IslandState::Compact, resized, resized, scale, false)
                    .unwrap();
            }
            let stats = renderer.stats();
            assert_eq!(
                (stats.presents, stats.commits),
                (3 + step as u64, 3 + step as u64)
            );
            assert_eq!(renderer.visible_rect(), Some(resized));
        }
    }
}
