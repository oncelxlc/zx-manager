use super::{
    config::IslandConfig,
    geometry::Display,
    manager::{IslandManager, ManagerHandle, RuntimeStatus},
    state::Event,
    window,
};
use tauri::{Manager, WebviewWindow};

// Window identity comes from the IPC invocation. The payload cannot choose its caller.
async fn request<T: Send + 'static>(
    caller: &WebviewWindow,
    labels: &[&str],
    job: impl FnOnce(&mut IslandManager) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    if !labels.contains(&caller.label()) {
        return Err("当前窗口无权执行该操作".into());
    }
    caller.state::<ManagerHandle>().request(job).await
}

const MAIN: &[&str] = &["main"];
const ISLAND: &[&str] = &["island"];
const BOTH: &[&str] = &["main", "island"];

#[tauri::command]
pub(super) async fn island_get_config(window: WebviewWindow) -> Result<IslandConfig, String> {
    request(&window, BOTH, IslandManager::read_config).await
}

#[tauri::command]
pub(super) async fn island_get_runtime_status(
    window: WebviewWindow,
) -> Result<RuntimeStatus, String> {
    request(&window, BOTH, |manager| Ok(manager.status())).await
}

#[tauri::command]
pub(super) async fn island_get_displays(window: WebviewWindow) -> Result<Vec<Display>, String> {
    request(&window, MAIN, |manager| window::displays(&manager.app)).await
}

#[tauri::command]
pub(super) async fn island_update_config(
    window: WebviewWindow,
    config: IslandConfig,
) -> Result<IslandConfig, String> {
    request(&window, MAIN, |manager| manager.save(config)).await
}

#[tauri::command]
pub(super) async fn island_set_enabled(
    window: WebviewWindow,
    enabled: bool,
    revision: u64,
) -> Result<IslandConfig, String> {
    request(&window, MAIN, move |manager| {
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
    request(&window, MAIN, move |manager| {
        manager.save(IslandConfig {
            revision,
            ..Default::default()
        })
    })
    .await
}

#[tauri::command]
pub(super) async fn island_show(window: WebviewWindow) -> Result<(), String> {
    request(&window, MAIN, |manager| manager.transition(Event::Show)).await
}

#[tauri::command]
pub(super) async fn island_hide(window: WebviewWindow) -> Result<(), String> {
    request(&window, BOTH, |manager| manager.transition(Event::Hide)).await
}

#[tauri::command]
pub(super) async fn island_toggle(window: WebviewWindow) -> Result<(), String> {
    request(&window, MAIN, |manager| manager.transition(Event::Toggle)).await
}

#[tauri::command]
pub(super) async fn island_restart(window: WebviewWindow) -> Result<(), String> {
    request(&window, MAIN, IslandManager::restart).await
}

#[tauri::command]
pub(super) async fn island_preview(
    window: WebviewWindow,
    config: IslandConfig,
) -> Result<(), String> {
    request(&window, MAIN, move |_| config.validate()).await
}

#[tauri::command]
pub(super) async fn island_set_position(
    window: WebviewWindow,
    display_id: String,
    x: f64,
    y: f64,
    revision: u64,
) -> Result<IslandConfig, String> {
    request(&window, MAIN, move |manager| {
        manager.set_position(display_id, x, y, revision)
    })
    .await
}

#[tauri::command]
pub(super) async fn island_set_pinned(window: WebviewWindow, pinned: bool) -> Result<(), String> {
    request(&window, BOTH, move |manager| {
        manager.transition(if pinned { Event::Pin } else { Event::Unpin })
    })
    .await
}

#[tauri::command]
pub(super) async fn island_interact(window: WebviewWindow, event: Event) -> Result<(), String> {
    request(&window, ISLAND, move |manager| match event {
        Event::PointerEnter | Event::PointerLeave | Event::Click => manager.transition(event),
        _ => Err("无效的指针事件".into()),
    })
    .await
}

#[tauri::command]
pub(super) async fn island_ready(window: WebviewWindow) -> Result<(), String> {
    request(&window, ISLAND, IslandManager::mark_ready).await
}

#[tauri::command]
pub(super) async fn island_drag(window: WebviewWindow) -> Result<(), String> {
    request(&window, ISLAND, IslandManager::drag).await
}
