mod collector;
mod dto;
mod gpu;

use tauri::{AppHandle, Manager};

pub use collector::SystemInformationCollector;
use dto::{CommandError, SystemInformation, SystemSummary};

#[tauri::command]
pub async fn get_system_summary(app: AppHandle) -> Result<SystemSummary, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<SystemInformationCollector>().collect_summary()
    })
    .await
    .map_err(|error| CommandError::task(error.to_string()))?
}

#[tauri::command]
pub async fn get_system_information(app: AppHandle) -> Result<SystemInformation, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<SystemInformationCollector>()
            .collect_information(&app)
    })
    .await
    .map_err(|error| CommandError::task(error.to_string()))?
}
