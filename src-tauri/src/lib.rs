mod system_information;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(system_information::SystemInformationCollector::default())
        .invoke_handler(tauri::generate_handler![
            greet,
            system_information::get_system_summary,
            system_information::get_system_information
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
