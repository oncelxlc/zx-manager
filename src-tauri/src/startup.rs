use serde::Serialize;
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupTransitionError {
    code: &'static str,
}

impl StartupTransitionError {
    fn new(code: &'static str) -> Self {
        Self { code }
    }
}

#[tauri::command]
pub fn complete_startup(app: AppHandle) -> Result<(), StartupTransitionError> {
    let main_window = app
        .get_webview_window("main")
        .ok_or_else(|| StartupTransitionError::new("main_window_missing"))?;

    main_window
        .show()
        .map_err(|_| StartupTransitionError::new("main_window_show_failed"))?;

    // Focus is best-effort. The window is already visible, so a platform-specific
    // focus refusal must not leave the splashscreen blocking the application.
    let _ = main_window.set_focus();

    if let Some(splashscreen) = app.get_webview_window("splashscreen") {
        splashscreen
            .close()
            .map_err(|_| StartupTransitionError::new("splashscreen_close_failed"))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::StartupTransitionError;

    #[test]
    fn startup_error_uses_a_stable_serializable_code() {
        let value = serde_json::to_value(StartupTransitionError::new("main_window_show_failed"))
            .expect("startup error should serialize");

        assert_eq!(value["code"], "main_window_show_failed");
    }
}
