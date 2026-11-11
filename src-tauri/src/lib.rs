#[cfg(windows)]
mod island;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(windows)]
    let builder = island::install(builder);
    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
