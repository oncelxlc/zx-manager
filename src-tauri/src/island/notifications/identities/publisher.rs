use super::Source;
use std::{
    ffi::c_void,
    fs::OpenOptions,
    os::windows::{ffi::OsStrExt, fs::OpenOptionsExt, io::AsRawHandle},
    path::{Component, Path, Prefix},
    ptr::null_mut,
};
use windows_sys::Win32::{
    Security::{
        Cryptography::{szOID_COMMON_NAME, CertGetNameStringW, CERT_NAME_ATTR_TYPE},
        WinTrust::*,
    },
    Storage::FileSystem::{
        GetDriveTypeW, GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
        FILE_SHARE_READ,
    },
};

const TENCENT: &str = "Tencent Technology (Shenzhen) Company Limited";

pub(super) fn verified_client(path: &Path, source: Source) -> bool {
    let Some(Component::Prefix(prefix)) = path.components().next() else {
        return false;
    };
    let Prefix::Disk(letter) = prefix.kind() else {
        return false;
    };
    let root = [u16::from(letter), 58, 92, 0];
    // DRIVE_FIXED; never open an arbitrary network target from an installed shortcut.
    if !path.is_absolute() || unsafe { GetDriveTypeW(root.as_ptr()) } != 3 {
        return false;
    }
    let Ok(file) = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(path)
    else {
        return false;
    };
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe {
        let mut info = WINTRUST_FILE_INFO {
            cbStruct: size_of::<WINTRUST_FILE_INFO>() as u32,
            pcwszFilePath: wide.as_ptr(),
            hFile: file.as_raw_handle(),
            ..Default::default()
        };
        let mut trust = WINTRUST_DATA {
            cbStruct: size_of::<WINTRUST_DATA>() as u32,
            dwUIChoice: WTD_UI_NONE,
            dwUnionChoice: WTD_CHOICE_FILE,
            Anonymous: WINTRUST_DATA_0 { pFile: &mut info },
            dwStateAction: WTD_STATEACTION_VERIFY,
            dwProvFlags: WTD_CACHE_ONLY_URL_RETRIEVAL,
            ..Default::default()
        };
        let mut policy = WINTRUST_ACTION_GENERIC_VERIFY_V2;
        let status = WinVerifyTrust(
            null_mut(),
            &mut policy,
            (&mut trust as *mut WINTRUST_DATA).cast(),
        );
        let trusted = status == 0 && tencent_signer(&trust);
        trust.dwStateAction = WTD_STATEACTION_CLOSE;
        WinVerifyTrust(
            null_mut(),
            &mut policy,
            (&mut trust as *mut WINTRUST_DATA).cast(),
        );
        trusted && product_matches(wide.as_ptr(), source)
    }
}

unsafe fn tencent_signer(trust: &WINTRUST_DATA) -> bool {
    let chain = WTHelperProvDataFromStateData(trust.hWVTStateData);
    if chain.is_null() {
        return false;
    }
    let signer = WTHelperGetProvSignerFromChain(chain, 0, 0, 0);
    if signer.is_null() {
        return false;
    }
    let leaf = WTHelperGetProvCertFromChain(signer, 0);
    if leaf.is_null() || (*leaf).pCert.is_null() {
        return false;
    }
    let mut name = [0u16; 256];
    let len = CertGetNameStringW(
        (*leaf).pCert,
        CERT_NAME_ATTR_TYPE,
        0,
        szOID_COMMON_NAME.cast(),
        name.as_mut_ptr(),
        name.len() as u32,
    );
    len > 1 && String::from_utf16_lossy(&name[..len as usize - 1]).eq_ignore_ascii_case(TENCENT)
}

unsafe fn product_matches(path: *const u16, source: Source) -> bool {
    let size = GetFileVersionInfoSizeW(path, null_mut());
    if size == 0 || size > 1024 * 1024 {
        return false;
    }
    let mut block = vec![0u32; (size as usize).div_ceil(4)];
    if GetFileVersionInfoW(path, 0, size, block.as_mut_ptr().cast()) == 0 {
        return false;
    }
    let mut value: *mut c_void = null_mut();
    let mut length = 0;
    let block = block.as_ptr().cast();
    if VerQueryValueW(
        block,
        windows_sys::core::w!("\\VarFileInfo\\Translation"),
        &mut value,
        &mut length,
    ) == 0
        || value.is_null()
    {
        return false;
    }
    let translations = std::slice::from_raw_parts(value.cast::<u16>(), length as usize / 2);
    for [language, codepage] in translations.as_chunks::<2>().0 {
        let key: Vec<u16> = format!("\\StringFileInfo\\{language:04x}{codepage:04x}\\ProductName")
            .encode_utf16()
            .chain(Some(0))
            .collect();
        if VerQueryValueW(block, key.as_ptr(), &mut value, &mut length) == 0
            || value.is_null()
            || length < 2
        {
            continue;
        }
        let product = String::from_utf16_lossy(std::slice::from_raw_parts(
            value.cast::<u16>(),
            length as usize - 1,
        ));
        if match source {
            Source::Wechat => matches!(
                product.to_ascii_lowercase().as_str(),
                "weixin" | "wechat" | "微信"
            ),
            Source::Qq => matches!(
                product.to_ascii_lowercase().as_str(),
                "qq" | "qqnt" | "腾讯qq"
            ),
        } {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsigned_executable_with_the_right_name_cannot_claim_notifications() {
        let dir =
            std::env::temp_dir().join(format!("zx-once-unsigned-source-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let file = dir.join("QQ.exe");
        std::fs::write(&file, b"MZ unsigned test fixture").unwrap();
        let trusted = verified_client(&file, Source::Qq);
        std::fs::remove_file(file).unwrap();
        std::fs::remove_dir(dir).unwrap();
        assert!(!trusted);
    }
}
