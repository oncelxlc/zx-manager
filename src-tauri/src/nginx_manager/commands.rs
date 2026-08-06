use super::dto::{
    AuthorizeNginxRootInput, CheckNginxUpdatesInput, ControlNginxInstanceInput, DirectorySelection,
    DirectorySelectionPurpose, GetNginxOperationHistoryInput, InspectNginxSystemServiceInput,
    NginxConfiguration, NginxInspection, NginxInstance, NginxOperationRecord, NginxRegistryState,
    NginxReleaseChannel, NginxReleaseStatus, NginxRuntimeDetails, NginxStatusEvent,
    NginxStatusSubscription, NginxSystemServiceCandidate, NginxSystemServiceInspection,
    NginxUpgradeProgress, NginxUpgradeResult, RegisterNginxInstanceInput,
    RegisterNginxSystemServiceInput, ResolveNginxRegistryMigrationInput, UpgradeNginxInstanceInput,
};
use super::error::{NginxError, NginxResult};
use super::manager::NginxManager;
use tauri::{ipc::Channel, AppHandle, State};
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
pub fn get_nginx_registry_state(manager: State<'_, NginxManager>) -> NginxRegistryState {
    manager.registry_state()
}

#[tauri::command]
pub fn resolve_nginx_registry_migration(
    manager: State<'_, NginxManager>,
    input: ResolveNginxRegistryMigrationInput,
) -> NginxResult<NginxRegistryState> {
    manager.resolve_registry_migration(input)
}

#[tauri::command]
pub fn subscribe_nginx_status(
    manager: State<'_, NginxManager>,
    channel: Channel<NginxStatusEvent>,
) -> NginxStatusSubscription {
    manager.subscribe_status(channel)
}

#[tauri::command]
pub fn unsubscribe_nginx_status(manager: State<'_, NginxManager>, subscription_id: u64) {
    manager.unsubscribe_status(subscription_id);
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

#[tauri::command]
pub fn get_nginx_configuration(
    manager: State<'_, NginxManager>,
    instance_id: String,
) -> NginxResult<NginxConfiguration> {
    manager.configuration(&instance_id)
}

#[tauri::command]
pub async fn control_nginx_instance(
    manager: State<'_, NginxManager>,
    input: ControlNginxInstanceInput,
) -> NginxResult<NginxOperationRecord> {
    manager.control(input)
}

#[tauri::command]
pub async fn upgrade_nginx_instance(
    manager: State<'_, NginxManager>,
    input: UpgradeNginxInstanceInput,
    progress_channel: Channel<NginxUpgradeProgress>,
) -> NginxResult<NginxUpgradeResult> {
    manager.upgrade(input, progress_channel).await
}

#[tauri::command]
pub fn get_nginx_operation_history(
    manager: State<'_, NginxManager>,
    input: GetNginxOperationHistoryInput,
) -> Vec<NginxOperationRecord> {
    manager.operation_history(input)
}

#[tauri::command]
pub fn get_nginx_runtime_details(
    manager: State<'_, NginxManager>,
    instance_id: String,
) -> NginxResult<NginxRuntimeDetails> {
    manager.runtime_details(&instance_id)
}

#[tauri::command]
pub fn list_nginx_system_services(
    manager: State<'_, NginxManager>,
) -> NginxResult<Vec<NginxSystemServiceCandidate>> {
    manager.list_system_services()
}

#[tauri::command]
pub fn inspect_nginx_system_service(
    manager: State<'_, NginxManager>,
    input: InspectNginxSystemServiceInput,
) -> NginxResult<NginxSystemServiceInspection> {
    manager.inspect_system_service(input)
}

#[tauri::command]
pub fn register_nginx_system_service(
    manager: State<'_, NginxManager>,
    input: RegisterNginxSystemServiceInput,
) -> NginxResult<NginxInstance> {
    manager.register_system_service(input)
}
