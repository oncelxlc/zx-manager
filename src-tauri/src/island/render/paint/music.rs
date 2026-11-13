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

struct TextRun {
    layout: IDWriteTextLayout,
    width: f32,
    baseline: f32,
    height: f32,
}

impl TextRun {
    unsafe fn draw(
        &self,
        dc: &ID2D1DeviceContext,
        brush: &ID2D1SolidColorBrush,
        x: f32,
        baseline: f32,
    ) {
        dc.DrawTextLayout(
            point(x, baseline - self.baseline),
            &self.layout,
            brush,
            D2D1_DRAW_TEXT_OPTIONS_CLIP,
        );
    }
}

fn secondary(key: &ImageKey, classic: u32, opacity: f32) -> D2D1_COLOR_F {
    if key.ink.r > 0.5 {
        color(classic, 1.0)
    } else {
        D2D1_COLOR_F {
            a: opacity,
            ..key.ink
        }
    }
}

unsafe fn polygon(
    dc: &ID2D1DeviceContext,
    brush: &ID2D1SolidColorBrush,
    vertices: &[Vector2],
) -> Result<()> {
    let geometry = dc.GetFactory()?.CreatePathGeometry()?;
    let sink = geometry.Open()?;
    sink.BeginFigure(vertices[0], D2D1_FIGURE_BEGIN_FILLED);
    sink.AddLines(&vertices[1..]);
    sink.EndFigure(D2D1_FIGURE_END_CLOSED);
    sink.Close()?;
    dc.FillGeometry(&geometry, brush, None);
    Ok(())
}

pub(super) unsafe fn charging_bolt(dc: &ID2D1DeviceContext) -> Result<ID2D1PathGeometry> {
    let geometry = dc.GetFactory()?.CreatePathGeometry()?;
    let sink = geometry.Open()?;
    sink.BeginFigure(point(1.6, -5.0), D2D1_FIGURE_BEGIN_FILLED);
    sink.AddLines(&[point(-3.2, 0.9), point(0.3, 0.9)]);
    sink.EndFigure(D2D1_FIGURE_END_CLOSED);
    sink.BeginFigure(point(-0.3, -0.9), D2D1_FIGURE_BEGIN_FILLED);
    sink.AddLines(&[point(3.2, -0.9), point(-1.6, 5.0)]);
    sink.EndFigure(D2D1_FIGURE_END_CLOSED);
    sink.Close()?;
    Ok(geometry)
}

impl Painter {
    unsafe fn run(&self, text: &str, format: &IDWriteTextFormat, width: f32) -> Result<TextRun> {
        let layout = self.write.CreateTextLayout(
            &text
                .encode_utf16()
                .map(|ch| match ch {
                    10..=13 | 0x85 | 0x2028 | 0x2029 => 32,
                    _ => ch,
                })
                .collect::<Vec<_>>(),
            format,
            width.max(1.0),
            64.0,
        )?;
        layout.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_LEADING)?;
        layout.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_NEAR)?;
        let mut metrics = DWRITE_TEXT_METRICS::default();
        let mut line = [DWRITE_LINE_METRICS::default()];
        layout.GetMetrics(&mut metrics)?;
        layout.GetLineMetrics(Some(&mut line), &mut 0)?;
        Ok(TextRun {
            layout,
            width: metrics.width.min(width),
            baseline: line[0].baseline,
            height: metrics.height,
        })
    }

    pub(super) unsafe fn header(
        &self,
        dc: &ID2D1DeviceContext,
        key: &ImageKey,
        panel: &Panel,
    ) -> Result<()> {
        let headline = match (&panel.music, &panel.time, &panel.subtitle) {
            (Some(_), Some((_, time)), Some((_, date))) => {
                let date = self.run(date, &self.centered, 185.0)?;
                let clock = self.run(time, &self.header_clock, 80.0)?;
                let battery = panel.battery.is_some();
                let group = date.width + 8.0 + clock.width + if battery { 24.0 } else { 0.0 };
                let gx = (419.0 - group) / 2.0 + if battery { 6.0 } else { 0.0 };
                Some((date, clock, gx))
            }
            _ => None,
        };
        self.brush.SetColor(&key.ink);
        if let Some(r) = panel.pet {
            let cx = r.x as f32 + r.width as f32 * 0.5;
            let cy = r.y as f32 + r.height as f32 * 0.5 - 0.5;
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
                let phase =
                    f32::from(panel.wave_phase) * 0.16 * (5.0 + i as f32 * 0.7) + i as f32 * 1.6;
                let height = if panel.wave_phase == 0 {
                    3.5
                } else {
                    7.5 + 4.0 * phase.sin()
                };
                let y = r.y as f32 + (r.height as f32 - height) * 0.5;
                dc.FillRoundedRectangle(
                    &round(
                        r.x as f32 + r.width as f32 / 2.0 - 7.8 + i as f32 * 3.9,
                        y,
                        1.9,
                        height,
                        0.95,
                    ),
                    &self.brush,
                );
            }
        }
        if let Some((r, percent, charging)) = panel.battery {
            let side = if key.state == IslandState::Compact && panel.side {
                14.4
            } else {
                r.width.min(r.height) as f32
            };
            let center = headline.as_ref().map_or_else(
                || {
                    point(
                        r.x as f32 + r.width as f32 * 0.5,
                        r.y as f32 + r.height as f32 * 0.5,
                    )
                },
                |(_, _, gx)| point(gx + 8.5, 18.6),
            );
            let radius = side * 0.41;
            let tone = if charging {
                color(0x21eb54, 1.0)
            } else if percent < 20 {
                color(0xf23046, 1.0)
            } else {
                secondary(key, 0xdcdce0, 0.86)
            };
            let line = (side * 0.13).max(1.0);
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
            if charging {
                let mut previous = Matrix3x2::identity();
                dc.GetTransform(&mut previous);
                let scale = side / 20.0;
                dc.SetTransform(
                    &(Matrix3x2::scale(scale, scale)
                        * Matrix3x2::translation(center.X, center.Y)
                        * previous),
                );
                dc.FillGeometry(&self.charging_bolt, &self.brush, None);
                dc.SetTransform(&previous);
            } else {
                let mut original = Matrix3x2::identity();
                dc.GetTransform(&mut original);
                let k = side / 20.0;
                dc.SetTransform(
                    &(Matrix3x2::scale(k, k)
                        * Matrix3x2::translation(center.X, center.Y)
                        * original),
                );
                let run = self.run(&percent.to_string(), &self.percent, 20.0)?;
                run.draw(dc, &self.brush, -run.width / 2.0, 2.4);
                dc.SetTransform(&original);
            }
        }
        self.brush.SetColor(&key.ink);
        if let Some((date, clock, gx)) = headline {
            let x = gx + if panel.battery.is_some() { 24.0 } else { 0.0 };
            self.brush.SetColor(&secondary(key, 0xd2d2d6, 0.82));
            date.draw(dc, &self.brush, x, 22.5);
            self.brush.SetColor(&key.ink);
            clock.draw(dc, &self.brush, x + date.width + 8.0, 22.5);
        } else if let Some((r, text)) = &panel.time {
            let cx = r.x as f32 + r.width as f32 / 2.0;
            let mid = r.y as f32 + r.height as f32 / 2.0;
            if panel.side && key.state == IslandState::Compact {
                let (hour, minute) = text.split_once(':').unwrap_or(("--", "--"));
                for (digits, baseline) in [(hour, mid - 1.0), (minute, mid + 13.0)] {
                    let run = self.run(digits, &self.side_clock, r.width as f32)?;
                    run.draw(dc, &self.brush, cx - run.width / 2.0, baseline);
                }
            } else if key.state == IslandState::Compact {
                let run = self.run(
                    text,
                    if r.width < 32 {
                        &self.narrow_clock
                    } else {
                        &self.clock
                    },
                    r.width as f32,
                )?;
                run.draw(dc, &self.brush, cx - run.width / 2.0, mid + 6.0);
            } else {
                self.message_text(dc, *r, text, &self.header_clock);
                if let Some((r, date)) = &panel.subtitle {
                    self.brush.SetColor(&secondary(key, 0xd2d2d6, 0.82));
                    self.message_text(dc, *r, date, &self.centered);
                }
            }
        }
        Ok(())
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
        let result = self.music_body(dc, key, panel, media);
        dc.SetTransform(&original);
        result
    }

    unsafe fn music_body(
        &mut self,
        dc: &ID2D1DeviceContext,
        key: &ImageKey,
        panel: &Panel,
        media: &MediaSnapshot,
    ) -> Result<()> {
        let has = !media.title.is_empty();
        let (art_y, side, text_x, text_width) = if panel.lyrics {
            (23, 44, 99.0, 123.0)
        } else {
            (13, 62, 121.0, 220.0)
        };
        self.music_cover(dc, media, bounds(44, art_y, side, side))?;
        self.header(dc, key, panel)?;
        self.brush.SetColor(&key.ink);
        self.run(
            if has { &media.title } else { "暂无音乐" },
            if panel.lyrics {
                &self.small_title
            } else {
                &self.title
            },
            text_width,
        )?
        .draw(
            dc,
            &self.brush,
            text_x,
            if panel.lyrics {
                41.0
            } else if has {
                53.0
            } else {
                47.0
            },
        );
        if has {
            self.brush.SetColor(&secondary(key, 0xb2b2ba, 0.70));
            self.run(&media.artist, &self.artist, text_width)?.draw(
                dc,
                &self.brush,
                text_x,
                if panel.lyrics { 57.0 } else { 67.0 },
            );
        }
        if panel.lyrics {
            self.brush.SetColor(&secondary(key, 0x232323, 0.16));
            dc.DrawLine(
                point(233.0, 32.0),
                point(233.0, 98.0),
                &self.brush,
                1.0,
                None,
            );
            self.brush.SetColor(&secondary(key, 0xb2b2ba, 0.7));
            let text = self.run("暂无歌词", &self.tiny, 130.0)?;
            text.draw(
                dc,
                &self.brush,
                246.0,
                61.0 - text.height / 2.0 + text.baseline,
            );
        }
        let track = panel.music_track();
        let duration = media.duration_ms.max(0);
        let pos = if duration > 0 {
            media.position_ms.clamp(0, duration)
        } else {
            media.position_ms.max(0)
        };
        let fraction = if duration > 0 {
            pos as f32 / duration as f32
        } else {
            0.0
        };
        let hover = key.message_hover == Some(MessageAction::Seek) && has;
        let h = if hover { 5.0 } else { 3.5 };
        let y = if panel.lyrics { 82.0 } else { 92.6 };
        let x = track.x as f32;
        let width = track.width as f32;
        self.brush.SetColor(&secondary(
            key,
            if has { 0x5a5a5a } else { 0x242427 },
            if has { 0.35 } else { 0.15 },
        ));
        dc.FillRoundedRectangle(&round(x, y - h / 2.0, width, h, h / 2.0), &self.brush);
        self.brush.SetColor(&key.ink);
        if has && fraction > 0.0 {
            dc.FillRoundedRectangle(
                &round(x, y - h / 2.0, (width * fraction).max(h), h, h / 2.0),
                &self.brush,
            );
        }
        if hover {
            self.circle(dc, x + width * fraction, y, 4.5);
        }
        if !has {
            self.brush.SetColor(&secondary(key, 0x4e4e52, 0.38));
        }
        let (time_font, time_y, time_end) = if panel.lyrics {
            (&self.tiny, 100.0, 220.0)
        } else {
            (&self.elapsed, 97.0, 375.0)
        };
        self.run(&timestamp(pos), time_font, 68.0)?
            .draw(dc, &self.brush, 44.0, time_y);
        let remaining = if has && duration == 0 {
            "--:--".into()
        } else {
            format!("-{}", timestamp(duration.saturating_sub(pos)))
        };
        let run = self.run(&remaining, time_font, 72.0)?;
        run.draw(dc, &self.brush, time_end - run.width, time_y);

        for (item, cx) in panel
            .items
            .iter()
            .skip(1)
            .zip([74.0, 162.0, 211.0, 258.0, 367.0])
        {
            if item.action.is_some() && item.action == key.message_hover {
                self.brush.SetColor(&D2D1_COLOR_F {
                    a: 20.0 / 255.0,
                    ..key.ink
                });
                self.circle(dc, cx, 118.0, 12.5);
            }
            self.brush.SetColor(&if item.action.is_some() {
                key.ink
            } else if has || item.text == "\u{e767}" {
                secondary(key, 0xb2b2ba, 0.70)
            } else {
                secondary(key, 0x58585c, 0.37)
            });
            match item.text.as_str() {
                "词" => {
                    if has && !panel.lyrics {
                        self.brush.SetColor(&secondary(key, 0xb2b2ba, 0.70));
                    }
                    let run = self.run("词", &self.lyrics_label, 26.0)?;
                    run.draw(dc, &self.brush, cx - run.width / 2.0, 123.0);
                }
                "⏪" | "⏩" => {
                    let dir = if item.text == "⏪" { -1.0 } else { 1.0 };
                    for x in [cx - 4.3, cx + 4.3] {
                        polygon(
                            dc,
                            &self.brush,
                            &[
                                point(x - dir * 4.6, 112.0),
                                point(x + dir * 4.6, 118.0),
                                point(x - dir * 4.6, 124.0),
                            ],
                        )?;
                    }
                }
                "▶" => polygon(
                    dc,
                    &self.brush,
                    &[
                        point(cx - 5.418, 110.5),
                        point(cx + 7.482, 118.0),
                        point(cx - 5.418, 125.5),
                    ],
                )?,
                "⏸" => {
                    for x in [cx - 6.45, cx + 2.064] {
                        dc.FillRectangle(
                            &D2D_RECT_F {
                                left: x,
                                top: 110.5,
                                right: x + 4.386,
                                bottom: 125.5,
                            },
                            &self.brush,
                        );
                    }
                }
                _ => {
                    dc.FillRoundedRectangle(&round(cx - 7.0, 115.2, 4.0, 5.6, 0.8), &self.brush);
                    polygon(
                        dc,
                        &self.brush,
                        &[
                            point(cx - 4.0, 115.2),
                            point(cx + 0.5, 112.0),
                            point(cx + 0.5, 124.0),
                            point(cx - 4.0, 120.8),
                        ],
                    )?;
                    for radius in [5.4, 8.5] {
                        if radius > 6.0 {
                            let mut tone = self.brush.GetColor();
                            tone.a *= 0.6;
                            self.brush.SetColor(&tone);
                        }
                        let at = |i: usize| {
                            let angle = -0.7 + i as f32 * 1.4 / 10.0;
                            point(
                                cx - 0.8 + radius * angle.cos(),
                                118.0 + radius * angle.sin(),
                            )
                        };
                        for i in 0..10 {
                            dc.DrawLine(at(i), at(i + 1), &self.brush, 1.4, None);
                        }
                    }
                }
            }
        }
        if has && !panel.side {
            self.brush.SetColor(&secondary(key, 0x161616, 0.09));
            dc.FillRoundedRectangle(&round(279.0, 108.0, 68.0, 20.0, 10.0), &self.brush);
            self.brush.SetColor(&secondary(key, 0x2f2f2f, 0.18));
            dc.DrawRoundedRectangle(
                &round(279.5, 108.5, 67.0, 19.0, 9.5),
                &self.brush,
                1.0,
                None,
            );
            self.brush.SetColor(&secondary(key, 0xb2b2ba, 0.70));
            let text = self.run("岛上歌词", &self.tiny_label, 65.0)?;
            text.draw(dc, &self.brush, 313.0 - text.width / 2.0, 122.0);
        }
        Ok(())
    }

    unsafe fn circle(&self, dc: &ID2D1DeviceContext, x: f32, y: f32, r: f32) {
        dc.FillEllipse(
            &D2D1_ELLIPSE {
                point: point(x, y),
                radiusX: r,
                radiusY: r,
            },
            &self.brush,
        );
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
        let (cx, cy) = (x + side / 2.0, y + side / 2.0);
        let s = if media.title.is_empty() {
            26.0
        } else {
            side * 0.39
        };
        self.circle(dc, cx - s * 0.24, cy + s * 0.28, s * 0.14);
        self.circle(dc, cx + s * 0.26, cy + s * 0.18, s * 0.14);
        dc.DrawLine(
            point(cx - s * 0.11, cy + s * 0.28),
            point(cx - s * 0.11, cy - s * 0.30),
            &self.brush,
            1.6,
            None,
        );
        dc.DrawLine(
            point(cx + s * 0.39, cy + s * 0.18),
            point(cx + s * 0.39, cy - s * 0.42),
            &self.brush,
            1.6,
            None,
        );
        polygon(
            dc,
            &self.brush,
            &[
                point(cx - s * 0.11 - 0.8, cy - s * 0.30),
                point(cx + s * 0.39 + 0.8, cy - s * 0.42),
                point(cx + s * 0.39 + 0.8, cy - s * 0.30),
                point(cx - s * 0.11 - 0.8, cy - s * 0.18),
            ],
        )
    }
}
