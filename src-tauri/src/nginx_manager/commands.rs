use super::dto::{
    AuthorizeNginxRootInput, CheckNginxUpdatesInput, DirectorySelection, DirectorySelectionPurpose,
    NginxInspection, NginxInstance, NginxReleaseChannel, NginxReleaseStatus,
    RegisterNginxInstanceInput,
};
use super::error::{NginxError, NginxResult};
use super::manager::NginxManager;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub async fn select_nginx_directory(
    app: AppHandle,
    manager: State<'_, NginxManager>,
    purpose: DirectorySelectionPurpose,
) -> NginxResult<Option<DirectorySelection>> {
    let selected = app.dialog().file().blocking_pick_folder();
    selected
        .map(|path| {
            path.into_path()
                .map_err(|error| NginxError::new("NGINX_SELECTION_INVALID", error.to_string()))
                .and_then(|path| manager.create_selection(path, purpose))
        })
        .transpose()
}

#[tauri::command]
pub async fn inspect_nginx_directory(
    manager: State<'_, NginxManager>,
    selection_id: String,
) -> NginxResult<NginxInspection> {
    manager.inspect(&selection_id)
}

#[tauri::command]
pub async fn register_nginx_instance(
    manager: State<'_, NginxManager>,
    input: RegisterNginxInstanceInput,
) -> NginxResult<NginxInstance> {
    manager.register(input)
}

#[tauri::command]
pub fn list_nginx_instances(manager: State<'_, NginxManager>) -> Vec<NginxInstance> {
    manager.list()
}

#[tauri::command]
pub fn refresh_nginx_instance(
    manager: State<'_, NginxManager>,
    instance_id: String,
) -> NginxResult<NginxInstance> {
    manager.refresh(&instance_id)
}

#[tauri::command]
pub fn authorize_nginx_instance_root(
    manager: State<'_, NginxManager>,
    input: AuthorizeNginxRootInput,
) -> NginxResult<NginxInstance> {
    manager.authorize_root(input)
}

#[tauri::command]
pub fn unregister_nginx_instance(
    manager: State<'_, NginxManager>,
    instance_id: String,
) -> NginxResult<()> {
    manager.unregister(&instance_id)
}

#[tauri::command]
pub fn get_nginx_release_status(
    manager: State<'_, NginxManager>,
    channel: NginxReleaseChannel,
) -> NginxReleaseStatus {
    manager.release_status(channel)
}

#[tauri::command]
pub async fn check_nginx_updates(
    manager: State<'_, NginxManager>,
    input: CheckNginxUpdatesInput,
) -> NginxResult<NginxReleaseStatus> {
    manager.check_updates(input).await
}
