use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TrafficLayer {
    Physical,
    Tunnel,
    Application,
}

impl TrafficLayer {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Physical => "physical",
            Self::Tunnel => "tunnel",
            Self::Application => "application",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AttributionQuality {
    Exact,
    InterfaceOnly,
    #[default]
    Unavailable,
}

impl AttributionQuality {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::InterfaceOnly => "interfaceOnly",
            Self::Unavailable => "unavailable",
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
pub enum InterfaceKind {
    Ethernet,
    Wifi,
    Tunnel,
    Loopback,
    Virtual,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InterfaceState {
    Up,
    Down,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RouteMode {
    FullTunnel,
    SplitTunnel,
    Direct,
    Unknown,
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
pub struct CapabilityStatus {
    pub available: bool,
    pub quality: AttributionQuality,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkMonitorCapabilities {
    pub platform: String,
    pub interface_traffic: CapabilityStatus,
    pub application_traffic: CapabilityStatus,
    pub proxy_configuration: CapabilityStatus,
    pub vpn_detection: CapabilityStatus,
    pub route_mode_detection: CapabilityStatus,
    pub history_storage: CapabilityStatus,
    pub retention_days: u8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkMonitorStatus {
    pub enabled: bool,
    pub collector_state: CollectorState,
    pub generation: u64,
    pub subscriber_count: usize,
    pub sample_interval_seconds: u64,
    pub database_created: bool,
    pub last_sampled_at: Option<i64>,
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
pub struct InterfaceTrafficSnapshot {
    pub id: String,
    pub name: String,
    pub kind: InterfaceKind,
    pub state: InterfaceState,
    pub layer: TrafficLayer,
    pub is_virtual: bool,
    pub tunnel_type: Option<String>,
    pub traffic: TrafficValues,
    pub quality: AttributionQuality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationTrafficSnapshot {
    pub application_id: String,
    pub display_name: String,
    pub traffic: TrafficValues,
    pub quality: AttributionQuality,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyVpnSnapshot {
    pub proxy_configured: bool,
    pub proxy_kinds: Vec<String>,
    pub pac_enabled: bool,
    pub vpn_connected: bool,
    pub route_mode: Option<RouteMode>,
    pub virtual_interface_ids: Vec<String>,
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
    pub device: TrafficValues,
    pub interfaces: Vec<InterfaceTrafficSnapshot>,
    pub applications: Vec<ApplicationTrafficSnapshot>,
    pub proxy_vpn: ProxyVpnSnapshot,
    pub warnings: Vec<NetworkMonitorWarning>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkSubscription {
    pub subscription_id: u64,
    pub generation: u64,
    pub initial_events: Vec<NetworkRealtimeEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QueryInterval {
    Auto,
    Second,
    Minute,
    FiveMinutes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QueryGroupBy {
    Time,
    Interface,
    Application,
    ProxySession,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkUsageQuery {
    pub from: i64,
    pub to: i64,
    pub interval: Option<QueryInterval>,
    pub group_by: Option<QueryGroupBy>,
    #[serde(default)]
    pub interface_ids: Vec<String>,
    #[serde(default)]
    pub application_ids: Vec<String>,
    #[serde(default)]
    pub proxy_session_ids: Vec<String>,
    #[serde(default)]
    pub layers: Vec<TrafficLayer>,
    pub time_zone: String,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkUsagePoint {
    pub from: i64,
    pub to: i64,
    pub group_id: String,
    pub layer: TrafficLayer,
    pub download_bytes: u64,
    pub upload_bytes: u64,
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
    pub interval: QueryInterval,
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
    pub application_id: Option<String>,
    pub from: Option<i64>,
    pub to: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ClearScope {
    All,
    Application,
    TimeRange,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearNetworkUsageResult {
    pub generation: u64,
    pub deleted_buckets: usize,
    pub cleared_at: i64,
    pub cleared_from: Option<i64>,
    pub cleared_to: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct RawInterfaceCounters {
    pub stable_id: String,
    pub interface_index: u32,
    pub name: String,
    pub kind: InterfaceKind,
    pub state: InterfaceState,
    pub is_virtual: bool,
    pub tunnel_type: Option<String>,
    pub received_bytes: u64,
    pub transmitted_bytes: u64,
}

#[derive(Debug, Clone, Default)]
pub struct RawProxyState {
    pub configured: bool,
    pub kinds: Vec<String>,
    pub pac_enabled: bool,
}

#[derive(Debug, Clone, Default)]
pub struct RawNetworkSample {
    pub interfaces: Vec<RawInterfaceCounters>,
    pub proxy: RawProxyState,
    pub route_mode: Option<RouteMode>,
    pub warnings: Vec<NetworkMonitorWarning>,
}
