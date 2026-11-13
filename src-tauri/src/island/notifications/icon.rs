use super::AppIcon;
use std::{os::windows::ffi::OsStrExt, path::Path, ptr};
use windows::Win32::{
    Graphics::Imaging::{
        CLSID_WICImagingFactory, GUID_WICPixelFormat32bppPBGRA, IWICImagingFactory,
        WICConvertBitmapSource,
    },
    System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER},
    UI::WindowsAndMessaging::HICON,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{DestroyIcon, PrivateExtractIconsW};

pub(super) fn from_executable(path: &Path) -> Option<AppIcon> {
    let filename: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe {
        let factory: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).ok()?;
        let mut icon = ptr::null_mut();
        if PrivateExtractIconsW(
            filename.as_ptr(),
            0,
            64,
            64,
            &mut icon,
            ptr::null_mut(),
            1,
            0,
        ) != 1
            || icon.is_null()
        {
            return None;
        }
        let bitmap = factory.CreateBitmapFromHICON(HICON(icon));
        DestroyIcon(icon);
        let pixels = WICConvertBitmapSource(&GUID_WICPixelFormat32bppPBGRA, &bitmap.ok()?).ok()?;
        let (mut width, mut height) = (0, 0);
        pixels.GetSize(&mut width, &mut height).ok()?;
        if !(1..=64).contains(&width) || !(1..=64).contains(&height) {
            return None;
        }
        let mut bgra = vec![0; (width * height * 4) as usize];
        pixels.CopyPixels(ptr::null(), width * 4, &mut bgra).ok()?;
        Some(AppIcon {
            width,
            height,
            bgra,
        })
    }
}
