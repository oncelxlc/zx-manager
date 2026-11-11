pub(super) mod motion;
mod watch;
use super::backend::Backend;
use super::config::{parse_hotkey, HotkeyConfig};
use super::surface::SurfaceEvent;
use std::{ffi::OsString, io, sync::mpsc};
use tauri::WebviewWindow;
use windows_sys::Win32::Foundation::HWND;
use winreg::{
    enums::{HKEY_CURRENT_USER, KEY_SET_VALUE},
    RegKey,
};

#[derive(Clone, Debug)]
pub enum NativeEvent {
    Input(SurfaceEvent),
    Toggle,
    Show,
    Hide,
    OutsideClick,
    Fullscreen(bool),
    DisplayChanged,
    DragEnded,
    Error(String),
}

fn os_error(operation: &str) -> String {
    format!("{operation}: {}", io::Error::last_os_error())
}

// The manager calls from its worker. Subclasses, hooks and geometry share the HWND's UI thread.
pub(super) fn on_ui<T: Send + 'static>(
    window: &WebviewWindow,
    f: impl FnOnce(HWND) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let raw = window.hwnd().map_err(|e| e.to_string())?.0 as usize;
    let (send, receive) = mpsc::sync_channel(1);
    window
        .run_on_main_thread(move || {
            let _ = send.send(f(raw as HWND));
        })
        .map_err(|e| e.to_string())?;
    receive.recv().map_err(|_| "窗口线程已关闭".to_string())?
}

impl Backend {
    pub fn install_watch(
        &self,
        callback: impl Fn(NativeEvent) + Send + Sync + 'static,
    ) -> Result<(), String> {
        self.on_ui(move |hwnd| watch::install(hwnd, callback))
    }

    pub fn configure_hotkeys(&self, keys: Option<&HotkeyConfig>) -> Result<(), String> {
        let bindings = match keys {
            Some(keys) => [
                parse_hotkey(&keys.toggle)?,
                parse_hotkey(&keys.show)?,
                parse_hotkey(&keys.hide)?,
            ],
            None => [None; 3],
        };
        self.on_ui(move |hwnd| watch::hotkeys(hwnd, bindings))
    }

    pub fn track(&self, fullscreen: bool, outside: bool) -> Result<(), String> {
        self.on_ui(move |hwnd| watch::track(hwnd, fullscreen, outside))
    }

    pub fn pointer_inside(&self) -> Result<bool, String> {
        self.on_ui(watch::pointer_inside)
    }

    pub fn uninstall_watch(&self) -> Result<(), String> {
        if let Self::Native(surface) = self {
            surface.cancel_drag()?;
        }
        self.on_ui(watch::uninstall)
    }

    pub fn begin_drag(&self) -> Result<super::geometry::Rect, String> {
        let origin = match self {
            Self::Webview(window) => window.outer_rect()?,
            Self::Native(surface) => surface.prepare_drag()?,
        };
        self.on_ui(watch::begin_drag)?;
        Ok(origin)
    }
}

pub fn set_autostart(enabled: bool) -> Result<(), String> {
    const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let result = if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let mut command = OsString::from("\"");
        command.push(exe);
        command.push("\"");
        hkcu.create_subkey_with_flags(RUN, KEY_SET_VALUE)
            .and_then(|(key, _)| key.set_value("zx-once", &command))
    } else {
        match hkcu
            .open_subkey_with_flags(RUN, KEY_SET_VALUE)
            .and_then(|key| key.delete_value("zx-once"))
        {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            result => result,
        }
    };
    result.map_err(|e| format!("更新当前用户的登录启动项失败: {e}"))
}
