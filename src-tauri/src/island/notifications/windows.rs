use super::{icon, identities::verified_client, AppIcon, Source, WindowEntry};
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::SystemTime,
};
use windows::Win32::{
    Foundation::HWND as ShellHwnd,
    Storage::EnhancedStorage::PKEY_AppUserModel_ID,
    System::Com::StructuredStorage::{PropVariantClear, PropVariantToString},
    UI::Shell::PropertiesSystem::{IPropertyStore, SHGetPropertyStoreForWindow},
};
use windows_sys::Win32::{
    Foundation::{FILETIME, HANDLE, HWND, INVALID_HANDLE_VALUE, LPARAM},
    Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED},
    Storage::Packaging::Appx::GetApplicationUserModelId,
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
        RemoteDesktop::ProcessIdToSessionId,
        Threading::{
            GetProcessTimes, OpenProcess, QueryFullProcessImageNameW,
            PROCESS_QUERY_LIMITED_INFORMATION,
        },
    },
    UI::WindowsAndMessaging::*,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct Target {
    hwnd: isize,
    tid: u32,
    pid: u32,
    created: u64,
    source: Source,
}

impl Target {
    fn owns_window(self) -> bool {
        let hwnd = self.hwnd as HWND;
        let mut pid = 0;
        !hwnd.is_null()
            && unsafe { GetWindowThreadProcessId(hwnd, &mut pid) } == self.tid
            && pid == self.pid
            && eligible(hwnd)
    }

    pub(super) fn activate(self) -> Result<(), String> {
        let hwnd = self.hwnd as HWND;
        let stale = || "窗口已关闭或已改变，请重新选择。".to_string();
        let (process, created, path) = inspect_process(self.pid).ok_or_else(stale)?;
        unsafe {
            if created != self.created
                || Source::from_filename(&path) != Some(self.source)
                || !self.owns_window()
            {
                return Err(stale());
            }
            if IsIconic(hwnd) != 0 {
                ShowWindowAsync(hwnd, SW_RESTORE);
            } else if IsWindowVisible(hwnd) == 0 {
                ShowWindowAsync(hwnd, SW_SHOW);
            }
            if SetForegroundWindow(hwnd) == 0 {
                return Err("Windows 未允许切换窗口，请从任务栏打开。".into());
            }
        }
        // Holding the process handle until activation completes prevents reuse of its PID.
        drop(process);
        Ok(())
    }
}

fn eligible(hwnd: HWND) -> bool {
    unsafe {
        if !GetWindow(hwnd, GW_OWNER).is_null()
            || GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & (WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE) != 0
        {
            return false;
        }
        let mut cloaked = 0u32;
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED as u32,
            (&raw mut cloaked).cast(),
            size_of::<u32>() as u32,
        ) < 0
            || cloaked == 0
    }
}

fn inspect_process(pid: u32) -> Option<(OwnedHandle, u64, PathBuf)> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }
        let process = OwnedHandle::from_raw_handle(handle);
        let mut name = [0u16; 32768];
        let mut len = name.len() as u32;
        if QueryFullProcessImageNameW(handle, 0, name.as_mut_ptr(), &mut len) == 0 {
            return None;
        }
        let path = PathBuf::from(String::from_utf16(&name[..len as usize]).ok()?);
        let mut times = [FILETIME::default(); 4];
        let [creation, exit, kernel, user] = &mut times;
        if GetProcessTimes(handle, creation, exit, kernel, user) == 0 {
            return None;
        }
        let created = ((creation.dwHighDateTime as u64) << 32) | creation.dwLowDateTime as u64;
        Some((process, created, path))
    }
}

fn process_app_id(handle: HANDLE) -> Option<String> {
    let mut id = [0u16; 256];
    let mut len = id.len() as u32;
    (unsafe { GetApplicationUserModelId(handle, &mut len, id.as_mut_ptr()) } == 0 && len > 1)
        .then(|| String::from_utf16_lossy(&id[..len as usize - 1]))
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
    let mut pid = 0;
    let tid = GetWindowThreadProcessId(hwnd, &mut pid);
    let windows = &mut *(out as *mut HashMap<u32, Vec<(isize, u32)>>);
    if let Some(handles) = windows.get_mut(&pid) {
        if IsWindowVisible(hwnd) != 0 && eligible(hwnd) {
            handles.push((hwnd as isize, tid));
        }
    }
    1
}

struct Client {
    pid: u32,
    parent: u32,
    created: u64,
    path: PathBuf,
    source: Source,
    app_id: Option<String>,
}

fn candidates() -> Vec<Client> {
    let mut clients = vec![];
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        let mut session = 0;
        if snapshot == INVALID_HANDLE_VALUE {
            return clients;
        }
        let snapshot = OwnedHandle::from_raw_handle(snapshot);
        if ProcessIdToSessionId(std::process::id(), &mut session) == 0 {
            return clients;
        }
        let mut entry = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut more = Process32FirstW(snapshot.as_raw_handle(), &mut entry);
        while more != 0 {
            let pid = entry.th32ProcessID;
            let length = entry
                .szExeFile
                .iter()
                .position(|c| *c == 0)
                .unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(&entry.szExeFile[..length]);
            if let Some(source) = Source::from_filename(Path::new(&name)) {
                let mut owner_session = 0;
                if ProcessIdToSessionId(pid, &mut owner_session) != 0 && owner_session == session {
                    if let Some((process, created, path)) = inspect_process(pid) {
                        if Source::from_filename(&path) == Some(source) {
                            clients.push(Client {
                                pid,
                                parent: entry.th32ParentProcessID,
                                created,
                                path,
                                source,
                                app_id: process_app_id(process.as_raw_handle()),
                            });
                        }
                    }
                }
            }
            more = Process32NextW(snapshot.as_raw_handle(), &mut entry);
        }
    }
    clients
}

struct Asset {
    modified: SystemTime,
    len: u64,
    trusted: bool,
    icon: Option<Arc<AppIcon>>,
}

#[derive(Default)]
pub(super) struct Registry {
    entries: Vec<(Target, WindowEntry)>,
    serials: HashMap<Source, u64>,
    assets: HashMap<PathBuf, Asset>,
    pub(super) identities: BTreeSet<(String, PathBuf)>,
}

impl Registry {
    pub(super) fn scan(&mut self) {
        let mut clients = candidates();
        let paths: HashSet<_> = clients.iter().map(|c| c.path.clone()).collect();
        self.assets.retain(|path, _| paths.contains(path));
        clients.retain(|c| {
            let Ok(info) = c.path.metadata() else {
                return false;
            };
            let Ok(modified) = info.modified() else {
                return false;
            };
            let len = info.len();
            if self
                .assets
                .get(&c.path)
                .is_none_or(|a| a.modified != modified || a.len != len)
            {
                let trusted = verified_client(&c.path, c.source);
                let icon = trusted
                    .then(|| icon::from_executable(&c.path))
                    .flatten()
                    .map(Arc::new);
                self.assets.insert(
                    c.path.clone(),
                    Asset {
                        modified,
                        len,
                        trusted,
                        icon,
                    },
                );
            }
            self.assets[&c.path].trusted
        });
        let mut handles: HashMap<u32, Vec<(isize, u32)>> =
            clients.iter().map(|c| (c.pid, vec![])).collect();
        if !handles.is_empty() {
            unsafe { EnumWindows(Some(collect), &raw mut handles as LPARAM) };
        }
        let mut current = vec![];
        self.identities.clear();
        for client in &clients {
            let mut windows = handles.remove(&client.pid).unwrap_or_default();
            if windows.is_empty() {
                // A client can hide its own main window to its tray. Only reuse previously seen targets.
                windows.extend(self.entries.iter().filter_map(|(target, _)| {
                    (target.pid == client.pid
                        && target.created == client.created
                        && target.owns_window())
                    .then_some((target.hwnd, target.tid))
                }));
            }
            if windows.is_empty() {
                // Chromium renderer/GPU children reuse the executable but have no independent window.
                if clients.iter().any(|p| {
                    p.pid == client.parent && p.path == client.path && p.created <= client.created
                }) {
                    continue;
                }
                windows.push((0, 0));
            }
            if let Some(id) = &client.app_id {
                self.identities.insert((id.clone(), client.path.clone()));
            }
            for (hwnd, tid) in windows {
                if hwnd != 0 {
                    if let Some(id) = window_app_id(hwnd as HWND) {
                        self.identities.insert((id, client.path.clone()));
                    }
                }
                let target = Target {
                    tid,
                    pid: client.pid,
                    created: client.created,
                    source: client.source,
                    hwnd,
                };
                let icon = self.assets[&client.path].icon.clone();
                current.push((target, icon));
            }
        }
        self.reconcile(current);
    }

    fn reconcile(&mut self, current: Vec<(Target, Option<Arc<AppIcon>>)>) {
        static NEXT_INSTANCE_ID: AtomicU64 = AtomicU64::new(1);
        for (old, entry) in self.entries.iter_mut().filter(|(t, _)| t.hwnd == 0) {
            let mut matches = current.iter().filter(|(t, _)| {
                t.pid == old.pid
                    && t.created == old.created
                    && t.source == old.source
                    && t.hwnd != 0
            });
            if let (Some((window, _)), None) = (matches.next(), matches.next()) {
                *old = *window;
                entry.can_activate = true;
            }
        }
        self.entries
            .retain(|(old, _)| current.iter().any(|(t, _)| t == old));
        for (target, icon) in current {
            if let Some((_, entry)) = self.entries.iter_mut().find(|(old, _)| *old == target) {
                entry.icon = icon;
                continue;
            }
            let serial = self.serials.entry(target.source).or_default();
            *serial += 1;
            self.entries.push((
                target,
                WindowEntry {
                    id: NEXT_INSTANCE_ID.fetch_add(1, Ordering::Relaxed),
                    label: format!("{} · {}", target.source.label(), serial),
                    source: target.source,
                    can_activate: target.hwnd != 0,
                    icon,
                },
            ));
        }
    }

    pub(super) fn accepts_reminder(&self, source: Source, created: i64) -> bool {
        self.entries
            .iter()
            .any(|(t, _)| t.source == source && i128::from(t.created) <= i128::from(created))
    }

    pub(super) fn empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(super) fn targets(&self) -> HashMap<u64, Target> {
        self.entries
            .iter()
            .filter(|(_, e)| e.can_activate)
            .map(|(target, e)| (e.id, *target))
            .collect()
    }

    pub(super) fn windows(&self) -> Vec<WindowEntry> {
        self.entries
            .iter()
            .map(|(_, entry)| entry.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_are_distinct_and_stale_handles_and_toasts_do_not_survive_restart() {
        let mut registry = Registry::default();
        let first = Target {
            tid: 5,
            pid: 11,
            created: 100,
            source: Source::Wechat,
            hwnd: 7,
        };
        let second = Target { hwnd: 8, ..first };
        let background = Target {
            pid: 12,
            hwnd: 0,
            ..first
        };
        registry.reconcile(vec![(first, None), (second, None), (background, None)]);
        let ids: Vec<_> = registry.windows().iter().map(|e| e.id).collect();
        assert_ne!(ids[0], ids[1]);
        assert!(!registry.targets().contains_key(&ids[2]));
        let appeared = Target {
            hwnd: 10,
            ..background
        };
        registry.reconcile(vec![(first, None), (second, None), (appeared, None)]);
        assert_eq!(registry.targets()[&ids[2]], appeared);
        registry.reconcile(vec![(second, None), (first, None)]);
        assert_eq!(registry.targets()[&ids[0]], first);
        assert_eq!(registry.targets()[&ids[1]], second);
        assert!(registry.accepts_reminder(Source::Wechat, 101));
        assert!(!registry.accepts_reminder(Source::Qq, 101));
        registry.reconcile(vec![(
            Target {
                created: 200,
                ..first
            },
            None,
        )]);
        assert!(!registry.targets().contains_key(&ids[0]));
        assert!(!registry.accepts_reminder(Source::Wechat, 101));
        registry.reconcile(vec![]);
        assert!(registry.empty());
        assert!(!registry.accepts_reminder(Source::Wechat, 300));
    }
}
