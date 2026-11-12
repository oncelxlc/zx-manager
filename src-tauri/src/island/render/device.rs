use super::{
    animation::Timing,
    paint::{ImageKey, Painter},
};
use crate::island::config::{AnimationConfig, AnimationPreset, PerformanceMode};
use windows::{
    core::{Error, Interface, Result},
    Win32::{
        Foundation::{E_FAIL, HMODULE, HWND},
        Graphics::{
            Direct2D::{Common::*, *},
            Direct3D::D3D_DRIVER_TYPE_HARDWARE,
            Direct3D11::*,
            DirectComposition::*,
            Dxgi::{Common::*, *},
        },
    },
};

pub(super) struct Texture {
    pub scale: IDCompositionScaleTransform,
    pub fade: IDCompositionEffectGroup,
    content_scale: IDCompositionScaleTransform,
    content: IDCompositionVisual,
    pub(super) outer: IDCompositionVisual,
    surface: IDCompositionVisual,
    viewport: IDCompositionVisual,
    glyphs: IDCompositionVisual,
    swap: IDXGISwapChain1,
    pub key: ImageKey,
}

impl Texture {
    pub unsafe fn reset_content(&self) -> Result<()> {
        self.content.SetOffsetY2(0.0)?;
        self.content_scale.SetScaleX2(1.0)?;
        self.content_scale.SetScaleY2(1.0)
    }

    unsafe fn layout(&self, key: &ImageKey) -> Result<()> {
        let (x, y) = key.content_origin();
        let (w, h) = (key.size.0 as f32, key.size.1 as f32);
        let clip = D2D_RECT_F {
            left: 0.0,
            top: 0.0,
            right: w,
            bottom: h,
        };
        self.outer.SetClip2(&clip)?;
        self.surface.SetClip2(&clip)?;
        self.viewport.SetClip2(&clip)?;
        self.glyphs.SetOffsetX2(-x as f32)?;
        self.glyphs.SetOffsetY2(-y as f32)?;
        self.content_scale.SetCenterX2(w / 2.0)?;
        self.content_scale.SetCenterY2(h / 2.0)
    }

    pub unsafe fn animate_content(
        &self,
        device: &IDCompositionDevice,
        timing: Timing,
        anim: &AnimationConfig,
        motion: bool,
        from_alpha: f32,
        to_alpha: f32,
    ) -> Result<()> {
        if self.key.content_origin() == (0, 0) || self.key.panel.is_some() {
            // Notification controls follow the same outer transform as their hit targets.
            return self.reset_content();
        }
        let amplitude = if motion { anim.amplitude as f32 } else { 0.0 };
        let distance = if anim.preset == AnimationPreset::Subtle {
            2.0
        } else {
            5.0
        };
        let offset =
            amplitude * distance * self.key.scale * if self.key.expanded() { 1.0 } else { -1.0 };
        timing.property(
            device,
            (1.0 - from_alpha) * offset,
            (1.0 - to_alpha) * offset,
            |v| self.content.SetOffsetY2(v),
            |a| self.content.SetOffsetY(a),
        )?;
        let shrink = if self.key.expanded() && anim.performance == PerformanceMode::Quality {
            0.03 * amplitude
        } else {
            0.0
        };
        timing.property(
            device,
            1.0 - (1.0 - from_alpha) * shrink,
            1.0 - (1.0 - to_alpha) * shrink,
            |v| {
                self.content_scale.SetScaleX2(v)?;
                self.content_scale.SetScaleY2(v)
            },
            |a| {
                self.content_scale.SetScaleX(a)?;
                self.content_scale.SetScaleY(a)
            },
        )
    }
}

pub(super) struct Device {
    pub composition: IDCompositionDevice,
    pub root: IDCompositionVisual,
    _target: IDCompositionTarget,
    factory: IDXGIFactory2,
    d3d: ID3D11Device,
    dc: ID2D1DeviceContext,
    painter: Painter,
}

impl Device {
    pub unsafe fn new(hwnd: HWND) -> Result<Self> {
        let mut device = None;
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            None,
        )?;
        let d3d = device.ok_or_else(|| Error::from_hresult(E_FAIL))?;
        let dxgi: IDXGIDevice = d3d.cast()?;
        let factory = dxgi.GetAdapter()?.GetParent()?;
        let dc =
            D2D1CreateDevice(&dxgi, None)?.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;
        dc.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
        let painter = Painter::new(&dc)?;
        let composition: IDCompositionDevice = DCompositionCreateDevice(&dxgi)?;
        let target = composition.CreateTargetForHwnd(hwnd, true)?;
        let root = composition.CreateVisual()?;
        target.SetRoot(&root)?;
        Ok(Self {
            composition,
            root,
            _target: target,
            factory,
            d3d,
            dc,
            painter,
        })
    }

    pub unsafe fn create_texture(&mut self, key: ImageKey) -> Result<Texture> {
        // Two clipped views of one buffer share a Present. Pack along the shorter
        // side, so even an island stretched across a large monitor stays compact.
        let (x, y) = key.content_origin();
        let desc = DXGI_SWAP_CHAIN_DESC1 {
            Width: (key.size.0 + x) as u32,
            Height: (key.size.1 + y) as u32,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 2,
            Scaling: DXGI_SCALING_STRETCH,
            SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
            AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED,
            ..Default::default()
        };
        let swap = self
            .factory
            .CreateSwapChainForComposition(&self.d3d, &desc, None)?;
        self.paint(&swap, &key)?;
        let visual = self.composition.CreateVisual()?;
        let scale = self.composition.CreateScaleTransform()?;
        let fade = self.composition.CreateEffectGroup()?;
        visual.SetEffect(&fade)?;
        visual.SetTransform(&scale)?;
        let surface = self.composition.CreateVisual()?;
        surface.SetContent(&swap)?;
        visual.AddVisual(&surface, true, None)?;
        let content = self.composition.CreateVisual()?;
        let viewport = self.composition.CreateVisual()?;
        let glyphs = self.composition.CreateVisual()?;
        let content_scale = self.composition.CreateScaleTransform()?;
        if (x, y) != (0, 0) {
            glyphs.SetContent(&swap)?;
            viewport.AddVisual(&glyphs, true, None)?;
            content.SetTransform(&content_scale)?;
            content.AddVisual(&viewport, true, None)?;
            visual.AddVisual(&content, true, &surface)?;
        }
        self.root.AddVisual(&visual, true, None)?;
        let image = Texture {
            scale,
            fade,
            swap,
            key,
            content_scale,
            content,
            outer: visual,
            surface,
            viewport,
            glyphs,
        };
        image.layout(&image.key)?;
        Ok(image)
    }

    pub unsafe fn update_texture(&mut self, image: &mut Texture, key: ImageKey) -> Result<()> {
        if image.key.size != key.size {
            let (x, y) = key.content_origin();
            image.swap.ResizeBuffers(
                0,
                (key.size.0 + x) as u32,
                (key.size.1 + y) as u32,
                DXGI_FORMAT_UNKNOWN,
                DXGI_SWAP_CHAIN_FLAG(0),
            )?;
            image.layout(&key)?;
        }
        self.paint(&image.swap, &key)?;
        image.key = key;
        Ok(())
    }

    unsafe fn paint(&mut self, swap: &IDXGISwapChain1, key: &ImageKey) -> Result<()> {
        let surface: IDXGISurface = swap.GetBuffer(0)?;
        let dpi = 96.0 * key.scale;
        let properties = D2D1_BITMAP_PROPERTIES1 {
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: dpi,
            dpiY: dpi,
            bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
            ..Default::default()
        };
        let bitmap = self
            .dc
            .CreateBitmapFromDxgiSurface(&surface, Some(&properties))?;
        self.dc.SetDpi(dpi, dpi);
        self.dc.SetTarget(&bitmap);
        self.dc.BeginDraw();
        let painted = self.painter.paint(&self.dc, key);
        let ended = self.dc.EndDraw(None, None);
        self.dc.SetTarget(None);
        painted?;
        ended?;
        swap.Present(1, DXGI_PRESENT(0)).ok()
    }
}
