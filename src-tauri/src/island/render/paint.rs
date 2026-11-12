use super::{MessageAction, Panel};
use crate::island::notifications::AppIcon;
use crate::island::{
    config::{IslandConfig, Theme},
    geometry::Rect,
    state::IslandState,
};
use std::sync::{Arc, Weak};
use windows::{
    core::{w, Result},
    Win32::Graphics::{
        Direct2D::{Common::*, *},
        DirectWrite::*,
        Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
    },
};
use windows_numerics::Vector2;
use winreg::{enums::HKEY_CURRENT_USER, RegKey};

#[derive(PartialEq)]
pub(super) struct ImageKey {
    pub panel: Option<Panel>,
    pub message_hover: Option<MessageAction>,
    pub size: (i32, i32),
    pub scale: f32,
    state: IslandState,
    radius: f32,
    background: D2D1_COLOR_F,
    border: D2D1_COLOR_F,
    ink: D2D1_COLOR_F,
    shadow: bool,
    peek_size: f32,
}

fn color(rgb: u32, a: f32) -> D2D1_COLOR_F {
    let channel = |shift: u32| ((rgb >> shift) & 255) as f32 / 255.0;
    D2D1_COLOR_F {
        r: channel(16),
        g: channel(8),
        b: channel(0),
        a,
    }
}

fn parse_rgb(hex: &str) -> u32 {
    u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or_default()
}

impl ImageKey {
    pub fn content_origin(&self) -> (i32, i32) {
        let (w, h) = self.size;
        if self.state == IslandState::EdgePeek {
            (0, 0)
        } else if w < h {
            (w, 0)
        } else {
            (0, h)
        }
    }

    pub fn new(cfg: &IslandConfig, state: IslandState, rect: Rect, scale: f64) -> Self {
        let a = &cfg.appearance;
        let light = match a.theme {
            Theme::Light => true,
            Theme::Dark => false,
            Theme::System => {
                RegKey::predef(HKEY_CURRENT_USER)
                    .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
                    .and_then(|k| k.get_value::<u32, _>("AppsUseLightTheme"))
                    .unwrap_or(1)
                    != 0
            }
        };
        let rgb = parse_rgb(&a.background);
        let auto = a.theme == Theme::System && matches!(rgb, 0x202326 | 0x101214);
        let background = color(if auto && light { 0xf5f5f4 } else { rgb }, a.opacity as f32);
        let linear = |v: f32| {
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        let luminance = 0.2126 * linear(background.r)
            + 0.7152 * linear(background.g)
            + 0.0722 * linear(background.b);
        let ink = color(
            if auto && light {
                0x181817
            } else if luminance > 0.179 {
                0x111111
            } else {
                0xffffff
            },
            1.0,
        );
        Self {
            panel: None,
            message_hover: None,
            size: (rect.width, rect.height),
            scale: scale as f32,
            state,
            radius: a.radius as f32,
            background,
            border: color(parse_rgb(&a.border), 1.0),
            ink,
            shadow: a.shadow,
            peek_size: if state == IslandState::EdgePeek {
                cfg.edge.peek_size as f32
            } else {
                0.0
            },
        }
    }
}

fn round(x: f32, y: f32, w: f32, h: f32, r: f32) -> D2D1_ROUNDED_RECT {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    D2D1_ROUNDED_RECT {
        rect: D2D_RECT_F {
            left: x,
            top: y,
            right: x + w,
            bottom: y + h,
        },
        radiusX: r,
        radiusY: r,
    }
}

fn point(x: f32, y: f32) -> Vector2 {
    Vector2 { X: x, Y: y }
}

unsafe fn font(factory: &IDWriteFactory, size: f32, bold: bool) -> Result<IDWriteTextFormat> {
    let weight = if bold {
        DWRITE_FONT_WEIGHT_BOLD
    } else {
        DWRITE_FONT_WEIGHT_NORMAL
    };
    factory.CreateTextFormat(
        w!("Segoe UI"),
        None,
        weight,
        DWRITE_FONT_STYLE_NORMAL,
        DWRITE_FONT_STRETCH_NORMAL,
        size,
        w!("zh-CN"),
    )
}

pub(super) struct Painter {
    brush: ID2D1SolidColorBrush,
    icons: Vec<(Weak<AppIcon>, ID2D1Bitmap1)>,
    clock: IDWriteTextFormat,
    detail: IDWriteTextFormat,
    centered: IDWriteTextFormat,
}

impl Painter {
    pub unsafe fn new(dc: &ID2D1DeviceContext) -> Result<Self> {
        let factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
        let message_font = |size, bold, center| -> Result<IDWriteTextFormat> {
            let fmt = font(&factory, size, bold)?;
            fmt.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)?;
            fmt.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
            if center {
                fmt.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
            }
            let trim = DWRITE_TRIMMING {
                granularity: DWRITE_TRIMMING_GRANULARITY_CHARACTER,
                ..Default::default()
            };
            fmt.SetTrimming(&trim, &factory.CreateEllipsisTrimmingSign(&fmt)?)?;
            Ok(fmt)
        };
        Ok(Self {
            brush: dc.CreateSolidColorBrush(&color(0xffffff, 1.0), None)?,
            icons: Vec::new(),
            clock: message_font(16.0, true, true)?,
            detail: message_font(13.0, false, false)?,
            centered: message_font(12.0, false, true)?,
        })
    }

    pub unsafe fn paint(&mut self, dc: &ID2D1DeviceContext, key: &ImageKey) -> Result<()> {
        let w = key.size.0 as f32 / key.scale;
        let h = key.size.1 as f32 / key.scale;
        let b = &self.brush;
        dc.Clear(None);
        if key.state == IslandState::EdgePeek {
            let (sw, sh) = if w < h {
                (key.peek_size.min(w), h)
            } else {
                (w, key.peek_size.min(h))
            };
            b.SetColor(&key.ink);
            dc.FillRoundedRectangle(
                &round((w - sw) / 2.0, (h - sh) / 2.0, sw, sh, sw.min(sh) / 2.0),
                b,
            );
            return Ok(());
        }
        let outline = round(0.5, 0.5, w - 1.0, h - 1.0, key.radius);
        b.SetColor(&key.background);
        dc.FillRectangle(
            &D2D_RECT_F {
                left: 0.0,
                top: 0.0,
                right: w,
                bottom: h,
            },
            b,
        );
        if key.shadow {
            // The WebView inset shadow is approximated by nested antialiased strokes.
            for inset in 1..=6 {
                let i = inset as f32;
                if w.min(h) <= 2.0 * i {
                    break;
                }
                b.SetColor(&color(0, (7.0 - i) * 0.014));
                dc.DrawRoundedRectangle(
                    &round(i, i, w - 2.0 * i, h - 2.0 * i, key.radius - i),
                    b,
                    1.0,
                    None,
                );
            }
        }
        b.SetColor(&key.border);
        dc.DrawRoundedRectangle(&outline, b, 1.0, None);
        let (x, y) = key.content_origin();
        dc.SetTransform(&windows_numerics::Matrix3x2::translation(
            x as f32 / key.scale,
            y as f32 / key.scale,
        ));
        let clip = D2D_RECT_F {
            left: 0.0,
            top: 0.0,
            right: w,
            bottom: h,
        };
        dc.PushAxisAlignedClip(&clip, D2D1_ANTIALIAS_MODE_ALIASED);
        let painted = match &key.panel {
            Some(panel) => self.messages(dc, key, panel),
            None => Ok(()),
        };
        dc.PopAxisAlignedClip();
        dc.SetTransform(&windows_numerics::Matrix3x2::identity());
        painted
    }

    unsafe fn message_text(
        &self,
        dc: &ID2D1DeviceContext,
        bounds: Rect,
        text: &str,
        font: &IDWriteTextFormat,
    ) {
        let clip = D2D_RECT_F {
            left: bounds.x as f32,
            top: bounds.y as f32,
            right: (bounds.x + bounds.width) as f32,
            bottom: (bounds.y + bounds.height) as f32,
        };
        dc.DrawText(
            &text.encode_utf16().collect::<Vec<_>>(),
            font,
            &clip,
            &self.brush,
            D2D1_DRAW_TEXT_OPTIONS_CLIP,
            DWRITE_MEASURING_MODE_NATURAL,
        );
    }

    unsafe fn bitmap(
        &mut self,
        dc: &ID2D1DeviceContext,
        icon: &Arc<AppIcon>,
    ) -> Result<ID2D1Bitmap1> {
        let reference = Arc::downgrade(icon);
        self.icons.retain(|(key, _)| key.strong_count() > 0);
        if let Some((_, bitmap)) = self.icons.iter().find(|(key, _)| key.ptr_eq(&reference)) {
            return Ok(bitmap.clone());
        }
        let properties = D2D1_BITMAP_PROPERTIES1 {
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0,
            dpiY: 96.0,
            ..Default::default()
        };
        let pitch = icon
            .width
            .checked_mul(4)
            .filter(|p| *p > 0)
            .ok_or(windows::Win32::Foundation::E_INVALIDARG)?;
        if icon.height == 0
            || (pitch as usize).checked_mul(icon.height as usize) != Some(icon.bgra.len())
        {
            return Err(windows::Win32::Foundation::E_INVALIDARG.into());
        }
        let bitmap = dc.CreateBitmap(
            D2D_SIZE_U {
                width: icon.width,
                height: icon.height,
            },
            Some(icon.bgra.as_ptr().cast()),
            pitch,
            &properties,
        )?;
        self.icons.push((reference, bitmap.clone()));
        Ok(bitmap)
    }

    unsafe fn reminder_dot(&self, dc: &ID2D1DeviceContext, key: &ImageKey, x: f32, y: f32) {
        self.brush.SetColor(&key.background);
        dc.FillRoundedRectangle(&round(x - 1.0, y - 1.0, 8.0, 8.0, 4.0), &self.brush);
        self.brush.SetColor(&color(
            if key.ink.r < 0.5 { 0xb32938 } else { 0xff7178 },
            1.0,
        ));
        dc.FillRoundedRectangle(&round(x, y, 6.0, 6.0, 3.0), &self.brush);
    }

    unsafe fn messages(
        &mut self,
        dc: &ID2D1DeviceContext,
        key: &ImageKey,
        panel: &Panel,
    ) -> Result<()> {
        if let Some(r) = panel.ring {
            let (cx, cy) = (
                (r.x as f32 + r.width as f32 / 2.0),
                (r.y as f32 + r.height as f32 / 2.0),
            );
            let radius = (r.width.min(r.height) as f32 / 2.0 - 1.0).max(0.0);
            self.brush.SetColor(&color(
                if key.ink.r < 0.5 { 0x177154 } else { 0x69ddb2 },
                1.0,
            ));
            dc.DrawEllipse(
                &D2D1_ELLIPSE {
                    point: point(cx, cy),
                    radiusX: radius,
                    radiusY: radius,
                },
                &self.brush,
                2.0,
                None,
            );
            dc.FillEllipse(
                &D2D1_ELLIPSE {
                    point: point(cx, cy),
                    radiusX: 2.0,
                    radiusY: 2.0,
                },
                &self.brush,
            );
        }
        self.brush.SetColor(&key.ink);
        if let Some((bounds, time)) = &panel.time {
            self.message_text(dc, *bounds, time, &self.clock);
        }
        if let Some((bounds, subtitle)) = &panel.subtitle {
            let mut tone = key.ink;
            tone.a = 0.76;
            self.brush.SetColor(&tone);
            self.message_text(dc, *bounds, subtitle, &self.centered);
        }
        for item in &panel.items {
            let r = item.bounds;
            let (x, y, w, h) = (r.x as f32, r.y as f32, r.width as f32, r.height as f32);
            if item.action.is_some() && key.message_hover == item.action {
                let mut tint = key.ink;
                tint.a = 0.12;
                self.brush.SetColor(&tint);
                dc.FillRoundedRectangle(&round(x, y, w, h, 8.0), &self.brush);
            }
            self.brush.SetColor(&key.ink);
            let Some(source) = item.source else {
                self.message_text(dc, r, &item.text, &self.centered);
                continue;
            };
            let narrow = r.width < 68 && !item.text.is_empty();
            let side = (if narrow { 14.0_f32 } else { 20.0 }).min(w).min(h);
            let left = if item.text.is_empty() {
                x + (w - side) / 2.0
            } else if narrow {
                x + 1.0
            } else {
                x + 5.0
            };
            let top = y + (h - side) / 2.0;
            if let Some(icon) = &item.icon {
                let bmp = self.bitmap(dc, icon)?;
                let ratio = side / icon.width.max(icon.height) as f32;
                let (iw, ih) = (icon.width as f32 * ratio, icon.height as f32 * ratio);
                let (l, t) = (left + (side - iw) / 2.0, top + (side - ih) / 2.0);
                let bounds = D2D_RECT_F {
                    left: l,
                    top: t,
                    right: l + iw,
                    bottom: t + ih,
                };
                dc.DrawBitmap(
                    &bmp,
                    Some(&bounds),
                    1.0,
                    D2D1_INTERPOLATION_MODE_LINEAR,
                    None,
                    None,
                );
            } else if item.text.is_empty() {
                self.message_text(dc, r, source.label(), &self.centered);
            }
            if !item.text.is_empty() {
                let inset = if item.icon.is_none() {
                    3
                } else if narrow {
                    16
                } else {
                    32
                };
                let mut tone = key.ink;
                if item.action.is_none() {
                    tone.a = 0.70;
                }
                self.brush.SetColor(&tone);
                self.message_text(
                    dc,
                    Rect {
                        x: r.x + inset,
                        width: r.width - inset - 3,
                        ..r
                    },
                    &item.text,
                    &self.detail,
                );
            }
            if item.reminder {
                let right = if item.icon.is_none() && !item.text.is_empty() {
                    x + w - 9.0
                } else {
                    left + side - 4.0
                };
                self.reminder_dot(dc, key, right, top - 1.0);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn texture_cache_ignores_revision_and_screen_position_but_tracks_dpi() {
        let mut cfg = IslandConfig::default();
        let rect = Rect {
            x: -65,
            y: 28,
            width: 328,
            height: 168,
        };
        let key = ImageKey::new(&cfg, IslandState::Expanded, rect, 1.0);
        cfg.revision += 1;
        assert!(key == ImageKey::new(&cfg, IslandState::Expanded, Rect { x: 0, ..rect }, 1.0));
        assert!(key != ImageKey::new(&cfg, IslandState::Expanded, rect, 1.5));
    }
}
