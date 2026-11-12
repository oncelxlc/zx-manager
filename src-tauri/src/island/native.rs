mod watch;
use super::config::{parse_hotkey, HotkeyConfig};
use super::surface::{Surface, SurfaceEvent};
use std::{ffi::OsString, io};
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

impl Surface {
    pub fn install_watch(
        &self,
        callback: impl Fn(NativeEvent) + Send + Sync + 'static,
    ) -> Result<(), String> {
        self.call(move |hwnd| watch::install(hwnd, callback))
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
        self.call(move |hwnd| watch::hotkeys(hwnd, bindings))
    }

    pub fn track(&self, fullscreen: bool, outside: bool) -> Result<(), String> {
        self.call(move |hwnd| watch::track(hwnd, fullscreen, outside))
    }

    pub fn pointer_inside(&self) -> Result<bool, String> {
        self.call(watch::pointer_inside)
    }

    pub fn uninstall_watch(&self) -> Result<(), String> {
        self.cancel_drag()?;
        self.call(watch::uninstall)
    }

    pub fn begin_drag(&self) -> Result<super::geometry::Rect, String> {
        let origin = self.prepare_drag()?;
        self.call(watch::begin_drag)?;
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
