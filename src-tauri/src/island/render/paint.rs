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

impl Label {
    unsafe fn new(factory: &IDWriteFactory, text: &str, size: f32, bold: bool) -> Result<Self> {
        let weight = if bold {
            DWRITE_FONT_WEIGHT_BOLD
        } else {
            DWRITE_FONT_WEIGHT_NORMAL
        };
        let fmt = factory.CreateTextFormat(
            w!("Segoe UI"),
            None,
            weight,
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            size,
            w!("zh-CN"),
        )?;
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
}

impl Painter {
    pub unsafe fn new(dc: &ID2D1DeviceContext) -> Result<Self> {
        let factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
        Ok(Self {
            brush: dc.CreateSolidColorBrush(&color(0xffffff, 1.0), None)?,
            compact: Label::new(&factory, "zx-once", 16.0, false)?,
            identity: Label::new(&factory, "zx-once", 12.0, false)?,
            title: None,
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
