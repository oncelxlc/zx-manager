use super::{MessageAction, Panel};
use crate::island::notifications::Source;
use crate::island::{
    config::{IslandConfig, Theme},
    geometry::Rect,
    state::{Event, IslandState},
};
use windows::{
    core::{w, Result},
    Win32::Graphics::{
        Direct2D::{
            Common::{D2D1_COLOR_F, D2D_RECT_F},
            *,
        },
        DirectWrite::*,
    },
};
use windows_numerics::Vector2;
use winreg::{enums::HKEY_CURRENT_USER, RegKey};

#[derive(PartialEq)]
pub(super) struct ImageKey {
    pub hovered: Option<Event>,
    pub panel: Option<Panel>,
    pub message_hover: Option<MessageAction>,
    pub size: (i32, i32),
    pub scale: f32,
    state: IslandState,
    radius: f32,
    background: D2D1_COLOR_F,
    border: D2D1_COLOR_F,
    ink: D2D1_COLOR_F,
    light: bool,
    shadow: bool,
    pin_enabled: bool,
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

    pub fn expanded(&self) -> bool {
        matches!(self.state, IslandState::Expanded | IslandState::Pinned)
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
        let auto = a.theme == Theme::System && rgb == 0x202326;
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
            hovered: None,
            panel: None,
            message_hover: None,
            size: (rect.width, rect.height),
            scale: scale as f32,
            state,
            radius: a.radius as f32,
            background,
            border: color(parse_rgb(&a.border), 1.0),
            ink,
            light,
            shadow: a.shadow,
            pin_enabled: matches!(state, IslandState::Expanded | IslandState::Pinned)
                && cfg.interaction.pin_enabled,
            peek_size: if state == IslandState::EdgePeek {
                cfg.edge.peek_size as f32
            } else {
                0.0
            },
        }
    }
}

struct Layout {
    left: f32,
    top: f32,
    title: f32,
    buttons_y: f32,
}

impl Layout {
    fn new(w: f32, h: f32) -> Self {
        let shortest = w.min(h);
        let title = if h >= 84.0 {
            (shortest * 0.16).clamp(14.0, 25.0)
        } else {
            0.0
        };
        let group = 16.0 + 8.0 + if title > 0.0 { title * 1.3 + 8.0 } else { 0.0 } + 32.0;
        let top = (h - group) * 0.5;
        Self {
            left: (0.07 * shortest).clamp(4.0, 20.0) + 1.0,
            top,
            title,
            buttons_y: top + group - 32.0,
        }
    }
}

pub fn action_rect(
    cfg: &IslandConfig,
    state: IslandState,
    rect: Rect,
    scale: f64,
    action: Event,
) -> Option<Rect> {
    if !matches!(state, IslandState::Expanded | IslandState::Pinned) {
        return None;
    }
    let s = scale as f32;
    let layout = Layout::new(rect.width as f32 / s, rect.height as f32 / s);
    let can_pin = cfg.interaction.pin_enabled;
    let offset = match action {
        Event::Pin if can_pin && state == IslandState::Expanded => 0.0,
        Event::Unpin if can_pin && state == IslandState::Pinned => 0.0,
        Event::Hide => {
            if can_pin {
                37.0
            } else {
                0.0
            }
        }
        _ => return None,
    };
    let side = (32.0 * s).round() as i32;
    Some(Rect {
        x: rect.x + ((layout.left + offset) * s).round() as i32,
        y: rect.y + (layout.buttons_y * s).round() as i32,
        width: side,
        height: side,
    })
}

pub fn action_at(
    cfg: &IslandConfig,
    state: IslandState,
    rect: Rect,
    scale: f64,
    x: i32,
    y: i32,
) -> Option<Event> {
    [Event::Pin, Event::Unpin, Event::Hide]
        .into_iter()
        .find(|&action| {
            action_rect(cfg, state, rect, scale, action)
                .is_some_and(|r| x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height)
        })
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

struct Label {
    layout: IDWriteTextLayout,
    width: f32,
    height: f32,
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

impl Label {
    unsafe fn new(factory: &IDWriteFactory, text: &str, size: f32, bold: bool) -> Result<Self> {
        let fmt = font(factory, size, bold)?;
        let layout = factory.CreateTextLayout(
            &text.encode_utf16().collect::<Vec<_>>(),
            &fmt,
            300.0,
            80.0,
        )?;
        let mut metrics = DWRITE_TEXT_METRICS::default();
        layout.GetMetrics(&mut metrics)?;
        Ok(Self {
            layout,
            width: metrics.width,
            height: metrics.height,
        })
    }
}

pub(super) struct Painter {
    brush: ID2D1SolidColorBrush,
    factory: IDWriteFactory,
    compact: Label,
    identity: Label,
    title: Option<(f32, Label)>,
    heading: IDWriteTextFormat,
    detail: IDWriteTextFormat,
    button: IDWriteTextFormat,
    qq: IDWriteTextFormat,
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
            compact: Label::new(&factory, "zx-once", 16.0, false)?,
            identity: Label::new(&factory, "zx-once", 12.0, false)?,
            title: None,
            heading: message_font(14.0, true, false)?,
            detail: message_font(12.0, false, false)?,
            button: message_font(12.0, false, true)?,
            qq: message_font(10.0, true, true)?,
            factory,
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
        dc.FillRoundedRectangle(&outline, b);
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
        let painted = self.content(dc, key, w, h);
        dc.PopAxisAlignedClip();
        dc.SetTransform(&windows_numerics::Matrix3x2::identity());
        painted
    }

    unsafe fn content(
        &mut self,
        dc: &ID2D1DeviceContext,
        key: &ImageKey,
        w: f32,
        h: f32,
    ) -> Result<()> {
        if let Some(panel) = &key.panel {
            return self.messages(dc, key, panel);
        }
        let b = &self.brush;
        b.SetColor(&key.ink);
        let line = |x: f32, y: f32, path: &[(f32, f32)]| {
            for pair in path.windows(2) {
                dc.DrawLine(
                    point(x + pair[0].0, y + pair[0].1),
                    point(x + pair[1].0, y + pair[1].1),
                    b,
                    1.5,
                    None,
                );
            }
        };
        let dot = |x: f32, y: f32| dc.FillRoundedRectangle(&round(x, y, 10.0, 10.0, 5.0), b);
        let text = |label: &Label, x: f32, y: f32| {
            dc.DrawTextLayout(point(x, y), &label.layout, b, D2D1_DRAW_TEXT_OPTIONS_CLIP)
        };
        if key.state == IslandState::Compact {
            if w < 90.0 {
                dot((w - 10.0) / 2.0, (h - 10.0) / 2.0);
            } else if w < h {
                let top = (h - 39.0 - self.compact.height) / 2.0;
                dot((w - 10.0) / 2.0, top);
                text(&self.compact, (w - self.compact.width) / 2.0, top + 18.0);
                line(
                    w / 2.0 - 5.0,
                    top + 29.0 + self.compact.height,
                    &[(0., 0.), (5., 5.), (10., 0.)],
                );
            } else {
                let left = (w - 39.0 - self.compact.width) / 2.0;
                dot(left, (h - 10.0) / 2.0);
                text(&self.compact, left + 18.0, (h - self.compact.height) / 2.0);
                line(
                    left + 27.0 + self.compact.width,
                    h / 2.0 - 2.0,
                    &[(0., 0.), (5., 5.), (10., 0.)],
                );
            }
            return Ok(());
        }
        let l = Layout::new(w, h);
        dot(l.left, l.top + 3.0);
        text(&self.identity, l.left + 17.0, l.top);
        if l.title > 0.0 {
            if self.title.as_ref().is_none_or(|(size, _)| *size != l.title) {
                self.title = Some((l.title, Label::new(&self.factory, "灵动岛", l.title, true)?));
            }
            if let Some((_, label)) = &self.title {
                text(label, l.left, l.top + 24.0);
            }
        }
        line(w - 26.0, 16.0, &[(0., 6.), (6., 0.), (12., 6.)]);
        let hover = |x| {
            b.SetColor(&if key.light {
                color(0, 0.067)
            } else {
                color(0xffffff, 0.133)
            });
            dc.FillRoundedRectangle(&round(x, l.buttons_y, 32.0, 32.0, 8.0), b);
            b.SetColor(&key.ink);
        };
        if key.pin_enabled {
            if key.state == IslandState::Pinned {
                b.SetColor(&color(if key.light { 0xcdece3 } else { 0x275447 }, 1.0));
                dc.FillRoundedRectangle(&round(l.left, l.buttons_y, 32.0, 32.0, 8.0), b);
                b.SetColor(&key.ink);
            } else if matches!(key.hovered, Some(Event::Pin)) {
                hover(l.left);
            }
            let x = l.left + 8.0;
            let y = l.buttons_y + 8.0;
            line(
                x,
                y,
                &[
                    (3., 1.),
                    (13., 1.),
                    (11., 3.),
                    (11., 8.),
                    (14., 11.),
                    (2., 11.),
                    (5., 8.),
                    (5., 3.),
                    (3., 1.),
                ],
            );
            line(x, y, &[(8., 11.), (8., 16.)]);
        }
        let hide_left = l.left + if key.pin_enabled { 37.0 } else { 0.0 };
        if key.hovered == Some(Event::Hide) {
            hover(hide_left);
        }
        let x = hide_left + 8.0;
        let y = l.buttons_y + 8.0;
        line(x, y, &[(1., 1.), (15., 15.)]);
        line(
            x,
            y,
            &[
                (3., 4.),
                (0.5, 8.),
                (3.5, 11.),
                (7., 13.),
                (10., 13.),
                (12., 12.),
            ],
        );
        line(
            x,
            y,
            &[(6., 3.5), (9., 3.), (12., 4.), (15.5, 8.), (13.5, 10.5)],
        );
        line(x, y, &[(6., 8.), (6.5, 9.5), (8.5, 10.)]);
        Ok(())
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

    unsafe fn messages(
        &self,
        dc: &ID2D1DeviceContext,
        key: &ImageKey,
        panel: &Panel,
    ) -> Result<()> {
        let b = &self.brush;
        for caption in &panel.captions {
            b.SetColor(&key.ink);
            self.message_text(
                dc,
                caption.bounds,
                &caption.text,
                if caption.primary {
                    &self.heading
                } else {
                    &self.detail
                },
            );
        }
        for item in &panel.items {
            let r = item.bounds;
            let (x, y, w, h) = (r.x as f32, r.y as f32, r.width as f32, r.height as f32);
            if item.selected || key.message_hover == Some(item.action) {
                let mut tint = key.ink;
                tint.a = if item.selected { 0.18 } else { 0.10 };
                b.SetColor(&tint);
                dc.FillRoundedRectangle(&round(x, y, w, h, 7.0), b);
            }
            if let Some(source) = item.source {
                let side = 21.0_f32.min(w).min(h);
                let left = if item.text.is_empty() {
                    x + (w - side) * 0.5
                } else {
                    x + 5.0
                };
                let top = y + (h - side) * 0.5;
                let fill = match source {
                    Source::Wechat => 0x107c54,
                    Source::Qq => 0x2262a4,
                };
                b.SetColor(&color(fill, 1.0));
                dc.FillRoundedRectangle(&round(left, top, side, side, 6.0), b);
                b.SetColor(&color(0xffffff, 1.0));
                match source {
                    Source::Wechat => {
                        dc.DrawRoundedRectangle(
                            &round(left + 3.0, top + 4.0, 11.0, 8.0, 4.0),
                            b,
                            1.4,
                            None,
                        );
                        dc.FillRoundedRectangle(&round(left + 8.5, top + 9.5, 9.0, 6.5, 3.25), b);
                        dc.DrawLine(
                            point(left + 14.0, top + 15.0),
                            point(left + 16.5, top + 17.0),
                            b,
                            1.5,
                            None,
                        );
                    }
                    Source::Qq => self.message_text(
                        dc,
                        Rect {
                            x: left.round() as i32,
                            y: top.round() as i32,
                            width: side as i32,
                            height: side as i32,
                        },
                        "QQ",
                        &self.qq,
                    ),
                }
                if !item.text.is_empty() {
                    b.SetColor(&key.ink);
                    self.message_text(
                        dc,
                        Rect {
                            x: r.x + 31,
                            width: r.width - 36,
                            ..r
                        },
                        &item.text,
                        &self.detail,
                    );
                }
            } else {
                b.SetColor(&key.ink);
                self.message_text(dc, r, &item.text, &self.button);
            }
            if item.reminder {
                let right = x + w * 0.5 + (w * 0.3).min(8.0);
                let top = y + h * 0.5 - (h * 0.35).min(10.0);
                let glow = panel
                    .pulse
                    .filter(|(s, _)| Some(*s) == item.source)
                    .map_or(0.0, |(_, v)| f32::from(v) / 8.0);
                let d = 6.5 + 2.5 * glow;
                b.SetColor(&key.background);
                dc.FillRoundedRectangle(
                    &round(right - 1.5, top - 1.5, d + 3.0, d + 3.0, (d + 3.0) / 2.0),
                    b,
                );
                b.SetColor(&color(
                    if key.ink.r < 0.5 { 0xb32938 } else { 0xff7178 },
                    1.0,
                ));
                dc.FillRoundedRectangle(&round(right, top, d, d, d / 2.0), b);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_ignores_revision_and_screen_position_but_tracks_dpi_and_pinning() {
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
        cfg.interaction.pin_enabled = false;
        assert!(key != ImageKey::new(&cfg, IslandState::Expanded, rect, 1.0));
        assert_eq!(
            action_at(
                &cfg,
                IslandState::Expanded,
                rect,
                1.0,
                rect.x + 20,
                rect.y + 111
            ),
            Some(Event::Hide)
        );
        cfg.interaction.pin_enabled = true;
        assert_eq!(
            action_at(
                &cfg,
                IslandState::Pinned,
                rect,
                1.0,
                rect.x + 20,
                rect.y + 111
            ),
            Some(Event::Unpin)
        );
    }
}
