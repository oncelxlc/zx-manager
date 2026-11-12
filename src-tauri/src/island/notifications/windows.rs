use super::{Source, WindowEntry};
use crate::island::config::NotificationConfig;
use std::{
    collections::HashMap,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};
use windows::Win32::{
    Foundation::HWND as ShellHwnd,
    Storage::EnhancedStorage::PKEY_AppUserModel_ID,
    System::Com::StructuredStorage::{PropVariantClear, PropVariantToString},
    UI::Shell::PropertiesSystem::{IPropertyStore, SHGetPropertyStoreForWindow},
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, FILETIME, HANDLE, HWND, LPARAM},
    Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED},
    Storage::Packaging::Appx::GetApplicationUserModelId,
    System::Threading::{
        GetProcessTimes, OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    },
    UI::WindowsAndMessaging::*,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Target {
    hwnd: isize,
    pid: u32,
    created: u64,
    source: Source,
}

impl Target {
    pub(super) fn source_enabled(self, config: &NotificationConfig) -> bool {
        self.source.enabled(config)
    }

    fn inspect(hwnd: HWND) -> Option<(Self, Option<String>)> {
        unsafe {
            if IsWindowVisible(hwnd) == 0
                || !GetWindow(hwnd, GW_OWNER).is_null()
                || GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & (WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE)
                    != 0
            {
                return None;
            }
            let mut cloaked = 0u32;
            if DwmGetWindowAttribute(
                hwnd,
                DWMWA_CLOAKED as u32,
                (&mut cloaked as *mut u32).cast(),
                size_of::<u32>() as u32,
            ) >= 0
                && cloaked != 0
            {
                return None;
            }
            let mut pid = 0;
            if GetWindowThreadProcessId(hwnd, &mut pid) == 0 || pid == std::process::id() {
                return None;
            }
            let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if process.is_null() {
                return None;
            }
            let details = process_details(process);
            CloseHandle(process);
            let (source, created, app_id) = details?;
            Some((
                Self {
                    hwnd: hwnd as isize,
                    pid,
                    created,
                    source,
                },
                app_id,
            ))
        }
    }

    pub(super) fn activate(self) -> Result<(), String> {
        let hwnd = self.hwnd as HWND;
        if Self::inspect(hwnd).map(|(target, _)| target) != Some(self) {
            return Err("窗口已关闭或已改变，请重新选择。".into());
        }
        unsafe {
            if IsIconic(hwnd) != 0 {
                ShowWindowAsync(hwnd, SW_RESTORE);
            }
            if SetForegroundWindow(hwnd) == 0 {
                return Err("Windows 未允许切换窗口，请从任务栏打开。".into());
            }
        }
        Ok(())
    }
}

unsafe fn process_details(process: HANDLE) -> Option<(Source, u64, Option<String>)> {
    let mut name = [0u16; 1024];
    let mut len = name.len() as u32;
    if QueryFullProcessImageNameW(process, 0, name.as_mut_ptr(), &mut len) == 0 {
        return None;
    }
    let path = String::from_utf16_lossy(&name[..len as usize]);
    let exe = Path::new(&path).file_name()?.to_str()?;
    let source = match exe.to_ascii_lowercase().as_str() {
        "wechat.exe" | "weixin.exe" => Source::Wechat,
        "qq.exe" => Source::Qq,
        _ => return None,
    };
    let mut times = [FILETIME::default(); 4];
    let [creation, exit, kernel, user] = &mut times;
    if GetProcessTimes(process, creation, exit, kernel, user) == 0 {
        return None;
    }
    let created = ((creation.dwHighDateTime as u64) << 32) | creation.dwLowDateTime as u64;
    let mut id = [0u16; 256];
    let mut len = id.len() as u32;
    let app_id = (GetApplicationUserModelId(process, &mut len, id.as_mut_ptr()) == 0 && len > 1)
        .then(|| String::from_utf16_lossy(&id[..len as usize - 1]));
    Some((source, created, app_id))
}

fn window_app_id(hwnd: HWND) -> Option<String> {
    unsafe {
        let store: IPropertyStore = SHGetPropertyStoreForWindow(ShellHwnd(hwnd)).ok()?;
        let mut value = store.GetValue(&PKEY_AppUserModel_ID).ok()?;
        let mut text = [0u16; 256];
        let converted = PropVariantToString(&value, &mut text);
        let _ = PropVariantClear(&mut value);
        converted.ok()?;
        let size = text.iter().position(|ch| *ch == 0)?;
        (size > 0).then(|| String::from_utf16_lossy(&text[..size]))
    }
}

unsafe extern "system" fn collect(hwnd: HWND, out: LPARAM) -> i32 {
    (*(out as *mut Vec<isize>)).push(hwnd as isize);
    1
}

#[derive(Default)]
pub(super) struct Registry {
    entries: Vec<(Target, WindowEntry)>,
    serials: HashMap<Source, u64>,
    pub(super) sources: HashMap<String, Option<Source>>,
}

impl Registry {
    pub(super) fn scan(&mut self, config: &NotificationConfig) {
        static NEXT_WINDOW_ID: AtomicU64 = AtomicU64::new(1);
        let mut handles: Vec<isize> = Vec::new();
        unsafe { EnumWindows(Some(collect), &mut handles as *mut _ as LPARAM) };
        let mut current = Vec::new();
        for hwnd in handles {
            let Some((target, process_id)) = Target::inspect(hwnd as HWND) else {
                continue;
            };
            if !target.source.enabled(config) {
                continue;
            }
            // AUMIDs come from the OS. Window titles and notification text never identify a source.
            for app_id in [process_id, window_app_id(hwnd as HWND)]
                .into_iter()
                .flatten()
            {
                let entry = self.sources.entry(app_id).or_insert(Some(target.source));
                if *entry != Some(target.source) {
                    *entry = None;
                }
            }
            current.push(target);
        }
        self.entries.retain(|(old, _)| current.contains(old));
        for target in current {
            if self.entries.iter().any(|(old, _)| *old == target) {
                continue;
            }
            let serial = self.serials.entry(target.source).or_default();
            *serial += 1;
            self.entries.push((
                target,
                WindowEntry {
                    id: NEXT_WINDOW_ID.fetch_add(1, Ordering::Relaxed),
                    source: target.source,
                    label: format!("{}窗口 {}", target.source.label(), serial),
                },
            ));
        }
    }

    pub(super) fn targets(&self) -> HashMap<u64, Target> {
        self.entries
            .iter()
            .map(|(target, entry)| (entry.id, *target))
            .collect()
    }

    pub(super) fn windows(&self) -> Vec<WindowEntry> {
        self.entries
            .iter()
            .map(|(_, entry)| entry.clone())
            .collect()
    }
}
