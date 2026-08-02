use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AttributionQuality {
    Exact,
    #[default]
    Partial,
}

impl AttributionQuality {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Partial => "partial",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NetworkPath {
    Proxy,
    Direct,
    #[default]
    Unknown,
}

impl NetworkPath {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Proxy => "proxy",
            Self::Direct => "direct",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NetworkPathFilter {
    #[default]
    All,
    Proxy,
    Direct,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NetworkUsageSortBy {
    Application,
    Download,
    Upload,
    #[default]
    Total,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SortDirection {
    Asc,
    #[default]
    Desc,
}

impl NetworkPathFilter {
    pub fn includes(self, path: NetworkPath) -> bool {
        match self {
            Self::All => true,
            Self::Proxy => path == NetworkPath::Proxy,
            Self::Direct => path == NetworkPath::Direct,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SampleState {
    Sample,
    Gap,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CollectorState {
    Disabled,
    Starting,
    Running,
    Degraded,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HelperState {
    Stopped,
    Starting,
    Running,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkMonitorWarning {
    pub code: String,
    pub message: Option<String>,
}

impl NetworkMonitorWarning {
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkMonitorCapabilities {
    pub platform: String,
    pub platform_supported: bool,
    pub requires_elevation: bool,
    pub application_traffic: bool,
    pub proxy_classification: bool,
    pub history_storage: bool,
    pub retention_days: u8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkMonitorStatus {
    pub platform_supported: bool,
    pub requires_elevation: bool,
    pub authorization_ready: bool,
    pub enabled: bool,
    pub collector_state: CollectorState,
    pub helper_state: HelperState,
    pub generation: u64,
    pub subscriber_count: usize,
    pub sample_interval_seconds: u64,
    pub database_created: bool,
    pub last_sampled_at: Option<i64>,
    pub lost_events: u64,
    pub unresolved_events: u64,
    pub partial_data: bool,
    pub last_error: Option<NetworkMonitorWarning>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficValues {
    pub download_bytes_per_second: Option<f64>,
    pub upload_bytes_per_second: Option<f64>,
    pub session_download_bytes: u64,
    pub session_upload_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationTrafficSnapshot {
    pub application_id: String,
    pub display_name: String,
    pub network_path: NetworkPath,
    pub traffic: TrafficValues,
    pub quality: AttributionQuality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkRealtimeEvent {
    pub generation: u64,
    pub sequence: u64,
    pub sampled_at: i64,
    pub elapsed_ms: Option<u64>,
    pub sample_state: SampleState,
    pub applications: Vec<ApplicationTrafficSnapshot>,
    pub unknown_traffic: TrafficValues,
    pub lost_events: u64,
    pub unresolved_events: u64,
    pub warnings: Vec<NetworkMonitorWarning>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkSubscription {
    pub subscription_id: u64,
    pub generation: u64,
    pub initial_events: Vec<NetworkRealtimeEvent>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkUsageQuery {
    pub from: i64,
    pub to: i64,
    #[serde(default)]
    pub network_path: NetworkPathFilter,
    pub time_zone: String,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
    #[serde(default)]
    pub sort_by: NetworkUsageSortBy,
    #[serde(default)]
    pub sort_direction: SortDirection,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkUsagePoint {
    pub application_id: String,
    pub display_name: String,
    pub download_bytes: u64,
    pub upload_bytes: u64,
    pub total_bytes: u64,
    pub includes_unknown: bool,
    pub quality: AttributionQuality,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkUsageResult {
    pub generation: u64,
    pub requested_from: i64,
    pub requested_to: i64,
    pub actual_from: i64,
    pub actual_to: i64,
    pub points: Vec<NetworkUsagePoint>,
    pub total_count: u64,
    pub next_cursor: Option<String>,
    pub partial: bool,
    pub warnings: Vec<NetworkMonitorWarning>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearNetworkUsageRequest {
    pub scope: ClearScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ClearScope {
    All,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearNetworkUsageResult {
    pub generation: u64,
    pub deleted_buckets: usize,
    pub cleared_at: i64,
}

#[derive(Debug, Clone)]
pub struct RawApplicationDelta {
    pub application_id: String,
    pub display_name: String,
    pub network_path: NetworkPath,
    pub download_bytes: u64,
    pub upload_bytes: u64,
    pub quality: AttributionQuality,
}

#[derive(Debug, Clone, Default)]
pub struct RawApplicationSample {
    pub applications: Vec<RawApplicationDelta>,
    pub lost_events: u64,
    pub unresolved_events: u64,
    pub warnings: Vec<NetworkMonitorWarning>,
}
