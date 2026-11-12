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
    let app = builder
        .build(tauri::generate_context!())
        .expect("error while building tauri application");
    app.run(|_, _event| {
        #[cfg(windows)]
        if let tauri::RunEvent::ExitRequested {
            code: None, api, ..
        } = _event
        {
            // Destroying the last WebView releases the UI while the tray remains available.
            api.prevent_exit();
        }
    });
}
