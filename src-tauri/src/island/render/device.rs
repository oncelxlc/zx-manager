use super::{
    animation::Timing,
    paint::{ImageKey, Painter},
};
use crate::island::geometry::Rect;
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
    pub shell_fade: IDCompositionEffectGroup,
    surface: IDCompositionVisual,
    viewport: IDCompositionVisual,
    glyphs: IDCompositionVisual,
    swap: IDXGISwapChain1,
    pub key: ImageKey,
}

impl Texture {
    unsafe fn layout(&self, key: &ImageKey) -> Result<()> {
        let (x, y) = key.content_origin();
        let (w, h) = (key.size.0 as f32, key.size.1 as f32);
        let clip = D2D_RECT_F {
            left: 0.0,
            top: 0.0,
            right: w,
            bottom: h,
        };
        self.surface.SetClip2(&clip)?;
        self.viewport.SetClip2(&clip)?;
        self.glyphs.SetOffsetX2(-x as f32)?;
        self.glyphs.SetOffsetY2(-y as f32)
    }
}

pub(super) struct Device {
    pub composition: IDCompositionDevice,
    pub root: IDCompositionVisual,
    shells: IDCompositionVisual,
    content: IDCompositionVisual,
    clip: IDCompositionRectangleClip,
    _target: IDCompositionTarget,
    factory: IDXGIFactory2,
    d3d: ID3D11Device,
    dc: ID2D1DeviceContext,
    painter: Painter,
}

impl Device {
    pub unsafe fn animate_clip(
        &self,
        timing: Timing,
        from: Rect,
        to: Rect,
        radii: [f64; 2],
    ) -> Result<()> {
        let d = &self.composition;
        let c = &self.clip;
        timing.property(
            d,
            from.width as f32,
            to.width as f32,
            |v| c.SetRight2(v),
            |a| c.SetRight(a),
        )?;
        timing.property(
            d,
            from.height as f32,
            to.height as f32,
            |v| c.SetBottom2(v),
            |a| c.SetBottom(a),
        )?;
        timing.property(
            d,
            radii[0] as f32,
            radii[1] as f32,
            |v| {
                c.SetTopLeftRadiusX2(v)?;
                c.SetTopLeftRadiusY2(v)?;
                c.SetTopRightRadiusX2(v)?;
                c.SetTopRightRadiusY2(v)?;
                c.SetBottomLeftRadiusX2(v)?;
                c.SetBottomLeftRadiusY2(v)?;
                c.SetBottomRightRadiusX2(v)?;
                c.SetBottomRightRadiusY2(v)
            },
            |a| {
                c.SetTopLeftRadiusX(a)?;
                c.SetTopLeftRadiusY(a)?;
                c.SetTopRightRadiusX(a)?;
                c.SetTopRightRadiusY(a)?;
                c.SetBottomLeftRadiusX(a)?;
                c.SetBottomLeftRadiusY(a)?;
                c.SetBottomRightRadiusX(a)?;
                c.SetBottomRightRadiusY(a)
            },
        )
    }

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
        let clip = composition.CreateRectangleClip()?;
        clip.SetLeft2(0.0)?;
        clip.SetTop2(0.0)?;
        root.SetClip(&clip)?;
        let shells = composition.CreateVisual()?;
        let content = composition.CreateVisual()?;
        root.AddVisual(&shells, true, None)?;
        root.AddVisual(&content, true, &shells)?;
        target.SetRoot(&root)?;
        Ok(Self {
            composition,
            root,
            shells,
            content,
            clip,
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
        let scale = self.composition.CreateScaleTransform()?;
        let shell_fade = self.composition.CreateEffectGroup()?;
        let fade = self.composition.CreateEffectGroup()?;
        let surface = self.composition.CreateVisual()?;
        surface.SetContent(&swap)?;
        surface.SetTransform(&scale)?;
        surface.SetEffect(&shell_fade)?;
        self.shells.AddVisual(&surface, true, None)?;
        let viewport = self.composition.CreateVisual()?;
        let glyphs = self.composition.CreateVisual()?;
        if (x, y) != (0, 0) {
            glyphs.SetContent(&swap)?;
            viewport.AddVisual(&glyphs, true, None)?;
            viewport.SetTransform(&scale)?;
            viewport.SetEffect(&fade)?;
            self.content.AddVisual(&viewport, true, None)?;
        }
        let image = Texture {
            scale,
            fade,
            shell_fade,
            swap,
            key,
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
