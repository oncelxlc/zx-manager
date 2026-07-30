mod network_monitor;
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
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(system_information::SystemInformationCollector::default())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            app.manage(network_monitor::NetworkMonitorManager::new(
                app_data_dir.join("network-usage.sqlite3"),
            ));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            startup::complete_startup,
            system_information::get_system_summary,
            system_information::get_system_information,
            network_monitor::commands::get_network_monitor_capabilities,
            network_monitor::commands::get_network_monitor_status,
            network_monitor::commands::subscribe_network_realtime,
            network_monitor::commands::unsubscribe_network_realtime,
            network_monitor::commands::query_network_usage,
            network_monitor::commands::set_network_monitor_enabled,
            network_monitor::commands::set_network_monitor_sample_interval,
            network_monitor::commands::clear_network_usage
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
        tauri::RunEvent::Exit => app_handle
            .state::<network_monitor::NetworkMonitorManager>()
            .shutdown(),
        _ => {}
    });
}

pub fn try_run_network_monitor_helper() -> bool {
    network_monitor::try_run_helper_from_args()
}
