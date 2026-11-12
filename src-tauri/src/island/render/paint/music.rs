use super::*;
use crate::island::media::MediaSnapshot;
use crate::island::render::PetPose;
use windows_numerics::Matrix3x2;

fn bounds(x: i32, y: i32, width: i32, height: i32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn timestamp(ms: i64) -> String {
    let seconds = ms.max(0) / 1000;
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

impl Painter {
    pub(super) unsafe fn header(&self, dc: &ID2D1DeviceContext, key: &ImageKey, panel: &Panel) {
        self.brush.SetColor(&key.ink);
        if let Some(r) = panel.pet {
            let cx = r.x as f32 + r.width as f32 * 0.5;
            let cy = r.y as f32 + r.height as f32 * 0.5;
            let (width, height) = match panel.pet_pose {
                PetPose::Awake => (3.6, 8.0),
                PetPose::Blink => (3.6, 3.6),
                PetPose::Dozing => (3.6, 4.1),
                PetPose::Sleeping => (6.8, 2.8),
            };
            for offset in [-5.6, 5.6] {
                dc.FillRoundedRectangle(
                    &round(
                        cx + offset - width * 0.5,
                        cy - height * 0.5,
                        width,
                        height,
                        1.8,
                    ),
                    &self.brush,
                );
            }
            if matches!(panel.pet_pose, PetPose::Dozing | PetPose::Sleeping) {
                self.brush.SetColor(&color(
                    if key.ink.r < 0.5 { 0x64569b } else { 0xaaa0ff },
                    1.0,
                ));
                self.message_text(
                    dc,
                    bounds(cx as i32 + 8, cy as i32 - 10, 8, 10),
                    "z",
                    &self.narrow_clock,
                );
                if panel.pet_pose == PetPose::Sleeping {
                    self.message_text(
                        dc,
                        bounds(cx as i32 + 14, cy as i32 - 13, 8, 10),
                        "z",
                        &self.percent,
                    );
                }
                self.brush.SetColor(&key.ink);
            }
        }
        if let Some(r) = panel.waveform {
            for i in 0..5 {
                let phase = f32::from(panel.wave_phase) * 0.8 + i as f32 * 1.6;
                let height = if panel.wave_phase == 0 {
                    3.5
                } else {
                    7.5 + 4.0 * phase.sin()
                };
                let y = r.y as f32 + (r.height as f32 - height) * 0.5;
                dc.FillRoundedRectangle(
                    &round(r.x as f32 + i as f32 * 3.9, y, 1.9, height, 0.95),
                    &self.brush,
                );
            }
        }
        if let Some((r, percent, charging)) = panel.battery {
            let side = r.width.min(r.height) as f32;
            let center = point(
                r.x as f32 + r.width as f32 * 0.5,
                r.y as f32 + r.height as f32 * 0.5,
            );
            let radius = side * 0.39;
            let tone = if charging {
                color(0x21eb54, 1.0)
            } else if percent < 20 {
                color(0xf23046, 1.0)
            } else {
                key.ink
            };
            let line = (side * 0.12).max(1.0);
            self.brush.SetColor(&D2D1_COLOR_F { a: 0.22, ..tone });
            dc.DrawEllipse(
                &D2D1_ELLIPSE {
                    point: center,
                    radiusX: radius,
                    radiusY: radius,
                },
                &self.brush,
                line,
                None,
            );
            self.brush.SetColor(&tone);
            let at = |step: u32| {
                let angle = std::f32::consts::TAU * f32::from(percent.min(100)) / 100.0
                    * step as f32
                    / 32.0;
                point(
                    center.X + radius * angle.sin(),
                    center.Y - radius * angle.cos(),
                )
            };
            for i in 1..=32 {
                dc.DrawLine(at(i - 1), at(i), &self.brush, line, None);
            }
            self.message_text(
                dc,
                r,
                &if charging {
                    "ϟ".into()
                } else {
                    percent.to_string()
                },
                &self.percent,
            );
        }
        self.brush.SetColor(&key.ink);
        if let Some((r, text)) = &panel.time {
            let font = if r.width < 32 {
                &self.narrow_clock
            } else if key.state == IslandState::Compact {
                &self.clock
            } else {
                &self.header_clock
            };
            self.message_text(dc, *r, text, font);
        }
        if let Some((r, date)) = &panel.subtitle {
            self.brush.SetColor(&D2D1_COLOR_F { a: 0.83, ..key.ink });
            self.message_text(dc, *r, date, &self.centered);
        }
    }

    pub(super) unsafe fn music(
        &mut self,
        dc: &ID2D1DeviceContext,
        key: &ImageKey,
        panel: &Panel,
        media: &MediaSnapshot,
    ) -> Result<()> {
        let (x, y, scale) = panel.music_frame();
        let mut original = Matrix3x2::identity();
        dc.GetTransform(&mut original);
        dc.SetTransform(
            &(Matrix3x2::scale(scale, scale) * Matrix3x2::translation(x, y) * original),
        );
        let narrow = panel.narrow_music();
        let has = !media.title.is_empty();
        let cover = if narrow {
            bounds(39, 43, 62, 62)
        } else {
            bounds(44, 13, 62, 62)
        };
        let result = self.music_cover(dc, media, cover);
        self.header(dc, key, panel);
        self.brush.SetColor(&key.ink);
        let title = if narrow {
            bounds(12, 111, 116, 24)
        } else {
            bounds(121, if has { 33 } else { 27 }, 239, 28)
        };
        self.message_text(
            dc,
            title,
            if has { &media.title } else { "暂无音乐" },
            &self.title,
        );
        if has {
            self.brush.SetColor(&D2D1_COLOR_F { a: 0.70, ..key.ink });
            let r = if narrow {
                bounds(12, 133, 116, 18)
            } else {
                bounds(121, 55, 239, 18)
            };
            self.message_text(dc, r, &media.artist, &self.artist);
        }
        let track = panel.music_track();
        let x0 = track.x as f32;
        let y0 = track.y as f32;
        self.brush.SetColor(&D2D1_COLOR_F {
            a: if has { 0.35 } else { 0.15 },
            ..key.ink
        });
        dc.FillRoundedRectangle(&round(x0, y0, track.width as f32, 3.5, 1.75), &self.brush);
        let duration = media.duration_ms.max(0);
        let pos = if duration > 0 {
            media.position_ms.clamp(0, duration)
        } else {
            media.position_ms.max(0)
        };
        if has && duration > 0 && pos > 0 {
            self.brush.SetColor(&key.ink);
            dc.FillRoundedRectangle(
                &round(
                    x0,
                    y0,
                    track.width as f32 * pos as f32 / duration as f32,
                    3.5,
                    1.75,
                ),
                &self.brush,
            );
        }
        self.brush.SetColor(&D2D1_COLOR_F {
            a: if has { 1.0 } else { 0.38 },
            ..key.ink
        });
        let (left, right) = if narrow {
            (bounds(9, 169, 46, 18), bounds(81, 169, 49, 18))
        } else {
            (bounds(39, 81, 48, 22), bounds(321, 81, 60, 22))
        };
        let font = if narrow {
            &self.centered
        } else {
            &self.elapsed
        };
        self.message_text(dc, left, &timestamp(pos), font);
        let remaining = if has && duration == 0 {
            "--:--".into()
        } else {
            format!("-{}", timestamp(duration.saturating_sub(pos)))
        };
        self.message_text(dc, right, &remaining, font);
        for item in &panel.items {
            if item.text.is_empty() {
                continue;
            }
            let r = item.bounds;
            let r = bounds(
                ((r.x as f32 - x) / scale).round() as i32,
                ((r.y as f32 - y) / scale).round() as i32,
                (r.width as f32 / scale).round() as i32,
                (r.height as f32 / scale).round() as i32,
            );
            if item.action.is_some() && item.action == key.message_hover {
                self.brush.SetColor(&D2D1_COLOR_F { a: 0.12, ..key.ink });
                dc.FillRoundedRectangle(
                    &round(
                        r.x as f32,
                        r.y as f32,
                        r.width as f32,
                        r.height as f32,
                        12.0,
                    ),
                    &self.brush,
                );
            }
            let opacity = if item.action == Some(MessageAction::OpenSettings) {
                0.72
            } else if item.action.is_some() {
                1.0
            } else {
                0.37
            };
            self.brush.SetColor(&D2D1_COLOR_F {
                a: opacity,
                ..key.ink
            });
            self.message_text(
                dc,
                r,
                &item.text,
                match item.text.as_str() {
                    "词" => &self.header_clock,
                    "⏪" | "▶" | "⏸" | "⏩" => &self.elapsed,
                    _ => &self.symbol,
                },
            );
        }
        dc.SetTransform(&original);
        result
    }

    unsafe fn music_cover(
        &mut self,
        dc: &ID2D1DeviceContext,
        media: &MediaSnapshot,
        r: Rect,
    ) -> Result<()> {
        let (x, y, side) = (r.x as f32, r.y as f32, r.width as f32);
        let shape = round(
            x,
            y,
            side,
            side,
            if media.title.is_empty() { 13.0 } else { 10.0 },
        );
        if let Some(icon) = &media.cover {
            let bitmap = self.bitmap(dc, icon)?;
            let ratio = side / icon.width.min(icon.height) as f32;
            let brush = dc.CreateBitmapBrush(&bitmap, None, None)?;
            brush.SetTransform(
                &(Matrix3x2::scale(ratio, ratio)
                    * Matrix3x2::translation(
                        x + (side - icon.width as f32 * ratio) * 0.5,
                        y + (side - icon.height as f32 * ratio) * 0.5,
                    )),
            );
            dc.FillRoundedRectangle(&shape, &brush);
            return Ok(());
        }
        if media.title.is_empty() {
            self.brush.SetColor(&color(0x19191b, 1.0));
            dc.FillRoundedRectangle(&shape, &self.brush);
            self.brush.SetColor(&color(0x6e6e72, 1.0));
        } else {
            let hash = media.title.bytes().fold(1469598103934665603u64, |h, b| {
                (h ^ u64::from(b)).wrapping_mul(1099511628211)
            });
            let colors = [
                (0xff5e62, 0xff9966),
                (0x56ccf2, 0x2f80ed),
                (0xa855f7, 0xec4899),
                (0x34d399, 0x1078c8),
                (0xfbbf24, 0xef4444),
            ];
            let (a, b) = colors[(hash % 5) as usize];
            let target: &ID2D1RenderTarget = dc;
            let stops = target.CreateGradientStopCollection(
                &[
                    D2D1_GRADIENT_STOP {
                        position: 0.0,
                        color: color(a, 1.0),
                    },
                    D2D1_GRADIENT_STOP {
                        position: 1.0,
                        color: color(b, 1.0),
                    },
                ],
                D2D1_GAMMA_2_2,
                D2D1_EXTEND_MODE_CLAMP,
            )?;
            let gradient = dc.CreateLinearGradientBrush(
                &D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES {
                    startPoint: point(x, y),
                    endPoint: point(x, y + side),
                },
                None,
                &stops,
            )?;
            dc.FillRoundedRectangle(&shape, &gradient);
            self.brush.SetColor(&color(0xffffff, 0.92));
        }
        self.message_text(dc, r, "\u{e8d6}", &self.note);
        Ok(())
    }
}
