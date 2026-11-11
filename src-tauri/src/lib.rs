#[cfg(windows)]
mod island;

#[cfg(windows)]
#[doc(hidden)]
pub use island::surface::run_poc as run_island_poc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(windows)]
    let builder = island::install(builder);
    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
