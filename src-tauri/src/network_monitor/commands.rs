use super::dto::{
    ClearNetworkUsageRequest, ClearNetworkUsageResult, NetworkMonitorCapabilities,
    NetworkMonitorStatus, NetworkRealtimeEvent, NetworkSubscription, NetworkUsageQuery,
    NetworkUsageResult,
};
use super::error::NetworkMonitorResult;
use super::manager::NetworkMonitorManager;
use tauri::{ipc::Channel, State};

#[tauri::command]
pub fn get_network_monitor_capabilities(
    manager: State<'_, NetworkMonitorManager>,
) -> NetworkMonitorCapabilities {
    manager.capabilities()
}

#[tauri::command]
pub fn get_network_monitor_status(
    manager: State<'_, NetworkMonitorManager>,
) -> NetworkMonitorStatus {
    manager.status()
}

#[tauri::command]
pub fn prepare_network_monitor(
    manager: State<'_, NetworkMonitorManager>,
) -> NetworkMonitorResult<NetworkMonitorStatus> {
    manager.prepare()
}

#[tauri::command]
pub fn subscribe_network_realtime(
    channel: Channel<NetworkRealtimeEvent>,
    manager: State<'_, NetworkMonitorManager>,
) -> NetworkMonitorResult<NetworkSubscription> {
    manager.subscribe(channel)
}

#[tauri::command]
pub fn unsubscribe_network_realtime(
    subscription_id: u64,
    manager: State<'_, NetworkMonitorManager>,
) {
    manager.unsubscribe(subscription_id);
}

#[tauri::command]
pub async fn query_network_usage(
    request: NetworkUsageQuery,
    manager: State<'_, NetworkMonitorManager>,
) -> NetworkMonitorResult<NetworkUsageResult> {
    let manager = manager.inner().clone();
    tauri::async_runtime::spawn_blocking(move || manager.query(request))
        .await
        .map_err(|error| {
            super::error::NetworkMonitorError::new(
                super::error::NetworkMonitorErrorCode::Internal,
                error.to_string(),
            )
        })?
}

#[tauri::command]
pub fn set_network_monitor_enabled(
    enabled: bool,
    manager: State<'_, NetworkMonitorManager>,
) -> NetworkMonitorResult<NetworkMonitorStatus> {
    manager.set_enabled(enabled)
}

#[tauri::command]
pub fn set_network_monitor_sample_interval(
    sample_interval_seconds: u64,
    manager: State<'_, NetworkMonitorManager>,
) -> NetworkMonitorResult<NetworkMonitorStatus> {
    manager.set_sample_interval(sample_interval_seconds)
}

#[tauri::command]
pub async fn clear_network_usage(
    request: ClearNetworkUsageRequest,
    manager: State<'_, NetworkMonitorManager>,
) -> NetworkMonitorResult<ClearNetworkUsageResult> {
    let manager = manager.inner().clone();
    tauri::async_runtime::spawn_blocking(move || manager.clear(request))
        .await
        .map_err(|error| {
            super::error::NetworkMonitorError::new(
                super::error::NetworkMonitorErrorCode::Internal,
                error.to_string(),
            )
        })?
}
