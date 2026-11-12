use super::{
    config::IslandConfig,
    displays,
    geometry::Display,
    manager::{IslandManager, ManagerHandle, PerformanceStats, RuntimeStatus},
    state::Event,
};
use tauri::{Manager, WebviewWindow};

// Window identity comes from the IPC invocation. The payload cannot choose its caller.
async fn request<T: Send + 'static>(
    caller: &WebviewWindow,
    job: impl FnOnce(&mut IslandManager) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    if caller.label() != "main" {
        return Err("当前窗口无权执行该操作".into());
    }
    caller.state::<ManagerHandle>().request(job).await
}

#[tauri::command]
pub(super) async fn island_get_config(window: WebviewWindow) -> Result<IslandConfig, String> {
    request(&window, IslandManager::read_config).await
}

#[tauri::command]
pub(super) async fn island_get_runtime_status(
    window: WebviewWindow,
) -> Result<RuntimeStatus, String> {
    request(&window, |manager| Ok(manager.status())).await
}

#[tauri::command]
pub(super) async fn island_get_performance_stats(
    window: WebviewWindow,
) -> Result<PerformanceStats, String> {
    request(&window, |manager| manager.performance()).await
}

#[tauri::command]
pub(super) async fn island_get_displays(window: WebviewWindow) -> Result<Vec<Display>, String> {
    request(&window, |manager| displays::displays(&manager.app)).await
}

#[tauri::command]
pub(super) async fn island_update_config(
    window: WebviewWindow,
    config: IslandConfig,
) -> Result<IslandConfig, String> {
    request(&window, |manager| manager.save(config)).await
}

#[tauri::command]
pub(super) async fn island_set_enabled(
    window: WebviewWindow,
    enabled: bool,
    revision: u64,
) -> Result<IslandConfig, String> {
    request(&window, move |manager| {
        let mut draft = manager.read_config()?;
        draft.enabled = enabled;
        draft.revision = revision;
        manager.save(draft)
    })
    .await
}

#[tauri::command]
pub(super) async fn island_reset_config(
    window: WebviewWindow,
    revision: u64,
) -> Result<IslandConfig, String> {
    request(&window, move |manager| {
        manager.save(IslandConfig {
            revision,
            ..Default::default()
        })
    })
    .await
}

#[tauri::command]
pub(super) async fn island_show(window: WebviewWindow) -> Result<(), String> {
    request(&window, |manager| manager.transition(Event::Show)).await
}

#[tauri::command]
pub(super) async fn island_hide(window: WebviewWindow) -> Result<(), String> {
    request(&window, |manager| manager.transition(Event::Hide)).await
}

#[tauri::command]
pub(super) async fn island_toggle(window: WebviewWindow) -> Result<(), String> {
    request(&window, |manager| manager.transition(Event::Toggle)).await
}

#[tauri::command]
pub(super) async fn island_restart(window: WebviewWindow) -> Result<(), String> {
    request(&window, IslandManager::restart).await
}

#[tauri::command]
pub(super) async fn island_preview(
    window: WebviewWindow,
    config: IslandConfig,
) -> Result<(), String> {
    request(&window, move |_| config.validate()).await
}

#[tauri::command]
pub(super) async fn island_set_position(
    window: WebviewWindow,
    display_id: String,
    x: f64,
    y: f64,
    revision: u64,
) -> Result<IslandConfig, String> {
    request(&window, move |manager| {
        manager.set_position(display_id, x, y, revision)
    })
    .await
}

#[tauri::command]
pub(super) async fn island_set_pinned(window: WebviewWindow, pinned: bool) -> Result<(), String> {
    request(&window, move |manager| {
        manager.transition(if pinned { Event::Pin } else { Event::Unpin })
    })
    .await
}
