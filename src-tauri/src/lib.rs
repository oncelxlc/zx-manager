mod legacy_cleanup;
mod nginx_manager;
mod startup;
mod system_information;

use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(system_information::SystemInformationCollector::default())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            legacy_cleanup::cleanup_retired_data(&app_data_dir);
            app.manage(nginx_manager::NginxManager::new(
                app_data_dir.join("nginx"),
            )?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            startup::complete_startup,
            system_information::get_system_summary,
            system_information::get_system_information,
            nginx_manager::commands::select_nginx_directory,
            nginx_manager::commands::inspect_nginx_directory,
            nginx_manager::commands::register_nginx_instance,
            nginx_manager::commands::get_nginx_registry_state,
            nginx_manager::commands::resolve_nginx_registry_migration,
            nginx_manager::commands::subscribe_nginx_status,
            nginx_manager::commands::unsubscribe_nginx_status,
            nginx_manager::commands::refresh_nginx_instance,
            nginx_manager::commands::authorize_nginx_instance_root,
            nginx_manager::commands::unregister_nginx_instance,
            nginx_manager::commands::get_nginx_release_status,
            nginx_manager::commands::check_nginx_updates,
            nginx_manager::commands::get_nginx_configuration,
            nginx_manager::commands::control_nginx_instance,
            nginx_manager::commands::upgrade_nginx_instance,
            nginx_manager::commands::get_nginx_operation_history,
            nginx_manager::commands::get_nginx_runtime_details,
            nginx_manager::commands::list_nginx_system_services,
            nginx_manager::commands::inspect_nginx_system_service,
            nginx_manager::commands::register_nginx_system_service
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application");
    app.run(|app_handle, event| match event {
        tauri::RunEvent::WindowEvent { label, event, .. }
            if label == "splashscreen"
                && matches!(event, tauri::WindowEvent::CloseRequested { .. }) =>
        {
            let main_window_visible = app_handle
                .get_webview_window("main")
                .and_then(|window| window.is_visible().ok())
                .unwrap_or(false);

            if !main_window_visible {
                app_handle.exit(0);
            }
        }
        tauri::RunEvent::Exit => app_handle.state::<nginx_manager::NginxManager>().shutdown(),
        _ => {}
    });
}
