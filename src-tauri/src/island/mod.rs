mod commands;
mod config;
mod config_store;
mod displays;
mod geometry;
mod manager;
mod media;
mod native;
mod notifications;
mod render;
mod state;
pub(crate) mod surface;

use commands::*;
use manager::ManagerHandle;
use state::Event;
use tauri::{
    menu::MenuBuilder,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, WindowEvent,
};

pub fn install(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    builder
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            let handle = ManagerHandle::start(app.handle().clone(), config_dir)?;
            app.manage(handle);
            install_tray(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            let Some(handle) = window.try_state::<ManagerHandle>() else {
                return;
            };
            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    let _ = handle.post(|manager| manager.close_main());
                }
                WindowEvent::Destroyed => {
                    let _ = handle.post(|manager| manager.main_destroyed());
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            main_window_restore,
            island_get_config,
            island_update_config,
            island_set_enabled,
            island_show,
            island_hide,
            island_toggle,
            island_restart,
            island_preview,
            island_reset_config,
            island_get_runtime_status,
            island_get_performance_stats,
            island_get_notifications,
            island_get_media,
            island_request_notification_access,
            island_open_notification_settings,
            island_activate_notification_window,
            island_get_displays,
            island_set_position,
            island_set_pinned
        ])
}

fn install_tray(app: &AppHandle) -> tauri::Result<()> {
    let menu = MenuBuilder::new(app)
        .text("open", "打开主界面")
        .text("settings", "灵动岛设置")
        .separator()
        .text("show", "显示灵动岛")
        .text("hide", "隐藏灵动岛")
        .separator()
        .text("quit", "退出应用")
        .build()?;
    let mut tray = TrayIconBuilder::with_id("island-tray")
        .tooltip("zx-once")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                let _ = tray
                    .app_handle()
                    .state::<ManagerHandle>()
                    .post(|manager| manager.open_main(false));
            }
        })
        .on_menu_event(|app, item| {
            let handle = app.state::<ManagerHandle>();
            match item.id().as_ref() {
                "open" => {
                    let _ = handle.post(|manager| manager.open_main(false));
                }
                "settings" => {
                    let _ = handle.post(|manager| manager.open_main(true));
                }
                "show" => {
                    let _ = handle.post(|manager| manager.transition(Event::Show));
                }
                "hide" => {
                    let _ = handle.post(|manager| manager.transition(Event::Hide));
                }
                "quit" => handle
                    .post(|manager| manager.exit())
                    .unwrap_or_else(|_| app.exit(0)),
                _ => {}
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

#[cfg(test)]
mod contract_tests;
