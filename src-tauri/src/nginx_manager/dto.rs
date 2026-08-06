use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DirectorySelectionPurpose {
    InspectInstance,
    AuthorizeAdditionalRoot,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectorySelection {
    pub selection_id: String,
    pub display_path: String,
    pub expires_at: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NginxAuthorizationLevel {
    ReadOnly,
    Full,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NginxLifecycleState {
    Available,
    Changed,
    Missing,
    Uninstalled,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NginxRuntimeStatus {
    Running,
    Stopped,
    Conflict,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NginxControlBackend {
    Portable,
    WindowsScm,
    Systemd,
    LaunchAgent,
    LaunchDaemonReadOnly,
    None,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxCapabilities {
    pub can_read: bool,
    pub can_edit: bool,
    pub can_control: bool,
    pub can_unregister: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxProviderIdentity {
    pub provider: String,
    pub external_id: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxInspection {
    pub inspection_id: String,
    pub display_root: String,
    pub display_binary: String,
    pub version: String,
    pub configure_arguments: Vec<String>,
    pub config_path: Option<String>,
    pub binary_fingerprint: String,
    pub warnings: Vec<String>,
    pub expires_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxInstanceRecord {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub root_path: String,
    pub binary_path: String,
    pub config_path: Option<String>,
    pub authorized_roots: Vec<String>,
    pub authorization_level: NginxAuthorizationLevel,
    pub version: String,
    pub configure_arguments: Vec<String>,
    pub binary_fingerprint: String,
    pub provider_identity: NginxProviderIdentity,
    pub control_backend: NginxControlBackend,
    pub lifecycle_state: NginxLifecycleState,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxInstance {
    #[serde(flatten)]
    pub record: NginxInstanceRecord,
    pub runtime_status: NginxRuntimeStatus,
    pub capabilities: NginxCapabilities,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NginxRegistryStatus {
    Empty,
    Ready,
    MigrationRequired,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxRegistryState {
    pub status: NginxRegistryStatus,
    pub instance: Option<NginxInstance>,
    pub migration_candidates: Vec<NginxInstance>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterNginxInstanceInput {
    pub inspection_id: String,
    pub authorization_level: NginxAuthorizationLevel,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveNginxRegistryMigrationInput {
    pub keep_instance_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizeNginxRootInput {
    pub instance_id: String,
    pub selection_id: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NginxReleaseChannel {
    Stable,
    Mainline,
}

impl NginxReleaseChannel {
    pub fn as_key(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Mainline => "mainline",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxRelease {
    pub version: String,
    pub download_url: String,
    pub signature_url: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxReleaseStatus {
    pub channel: NginxReleaseChannel,
    pub latest_release: Option<NginxRelease>,
    pub checked_at: Option<String>,
    pub stale: bool,
    pub source: String,
    pub update_available_count: usize,
    pub outdated_instance_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckNginxUpdatesInput {
    pub channel: NginxReleaseChannel,
    pub force: bool,
    pub max_age_hours: u16,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxSourceLocation {
    pub source_id: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxDirective {
    pub name: String,
    pub arguments: Vec<String>,
    pub raw: String,
    pub location: NginxSourceLocation,
    pub children: Vec<NginxDirective>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxConfigSource {
    pub id: String,
    pub display_path: String,
    pub include_chain: Vec<String>,
    pub text: String,
    pub directives: Vec<NginxDirective>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxConfigDiagnostic {
    pub code: String,
    pub severity: String,
    pub message: String,
    pub location: Option<NginxSourceLocation>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxSite {
    pub id: String,
    pub context: String,
    pub listens: Vec<String>,
    pub server_names: Vec<String>,
    pub root: Option<String>,
    pub proxy_pass: Vec<String>,
    pub locations: Vec<String>,
    pub source: NginxSourceLocation,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxUpstream {
    pub name: String,
    pub servers: Vec<String>,
    pub source: NginxSourceLocation,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxTopologyNode {
    pub id: String,
    pub kind: String,
    pub label: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxTopologyEdge {
    pub from: String,
    pub to: String,
    pub label: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxConfiguration {
    pub instance_id: String,
    pub entry_source_id: String,
    pub sources: Vec<NginxConfigSource>,
    pub diagnostics: Vec<NginxConfigDiagnostic>,
    pub sites: Vec<NginxSite>,
    pub upstreams: Vec<NginxUpstream>,
    pub topology_nodes: Vec<NginxTopologyNode>,
    pub topology_edges: Vec<NginxTopologyEdge>,
    pub pid_path: Option<String>,
    pub access_logs: Vec<String>,
    pub error_logs: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxConfigRevision {
    pub value: String,
    pub modified_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NginxGlobalConfigurationPatch {
    pub worker_processes: Option<String>,
    pub worker_rlimit_nofile: Option<String>,
    pub pid: Option<String>,
    pub error_log: Option<String>,
    pub top_level_includes: Vec<String>,
    pub worker_connections: Option<String>,
    pub multi_accept: Option<String>,
    pub accept_mutex: Option<String>,
    pub accept_mutex_delay: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxGlobalConfiguration {
    pub instance_id: String,
    pub revision: NginxConfigRevision,
    #[serde(flatten)]
    pub values: NginxGlobalConfigurationPatch,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateNginxGlobalConfigurationPatchInput {
    pub instance_id: String,
    pub expected_revision: String,
    pub patch: NginxGlobalConfigurationPatch,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NginxGlobalConfigApplyMode {
    Save,
    Reload,
    Restart,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyNginxGlobalConfigurationPatchInput {
    pub instance_id: String,
    pub expected_revision: String,
    pub patch: NginxGlobalConfigurationPatch,
    pub mode: NginxGlobalConfigApplyMode,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxGlobalConfigFieldError {
    pub field: String,
    pub code: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxGlobalConfigPatchValidation {
    pub current_revision: NginxConfigRevision,
    pub proposed_revision: Option<NginxConfigRevision>,
    pub field_errors: Vec<NginxGlobalConfigFieldError>,
    pub parser_valid: bool,
    pub native_valid: bool,
    pub native_error_code: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxGlobalConfigApplyResult {
    pub revision: NginxConfigRevision,
    pub mode: NginxGlobalConfigApplyMode,
    pub success: bool,
    pub rolled_back: bool,
    pub rollback_succeeded: Option<bool>,
    pub error_code: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxConfigGraphSource {
    pub id: String,
    pub display_path: String,
    pub include_chain: Vec<String>,
    pub node_count: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxConfigGraphNode {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub arguments: Vec<String>,
    pub label: String,
    pub source_id: String,
    pub include_chain: Vec<String>,
    pub location: NginxSourceLocation,
    pub child_ids: Vec<String>,
    pub reference_ids: Vec<String>,
    pub known: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxConfigGraph {
    pub instance_id: String,
    pub entry_source_id: String,
    pub revision: NginxConfigRevision,
    pub sources: Vec<NginxConfigGraphSource>,
    pub nodes: Vec<NginxConfigGraphNode>,
    pub diagnostics: Vec<NginxConfigDiagnostic>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxConfigNodeDetail {
    pub node: NginxConfigGraphNode,
    pub raw: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxConfigValidationResult {
    pub revision: NginxConfigRevision,
    pub parser_valid: bool,
    pub native_valid: bool,
    pub native_error_code: Option<String>,
    pub diagnostics: Vec<NginxConfigDiagnostic>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NginxControlAction {
    Start,
    Stop,
    Reload,
    Restart,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NginxOperationOutcome {
    Executed,
    Noop,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NginxOperationPhase {
    Starting,
    Stopping,
    Reloading,
    Restarting,
    Downloading,
    Verifying,
    BackingUp,
    Replacing,
    RestoringRuntime,
    RollingBack,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlNginxInstanceInput {
    pub instance_id: String,
    pub action: NginxControlAction,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxOperationRecord {
    pub id: String,
    pub instance_id: String,
    pub action: NginxControlAction,
    pub backend: NginxControlBackend,
    pub started_at: String,
    pub completed_at: String,
    pub success: bool,
    pub outcome: NginxOperationOutcome,
    pub resulting_status: NginxRuntimeStatus,
    pub error_code: Option<String>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NginxProcessRole {
    Master,
    Worker,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NginxRuntimeMetricAvailability {
    Available,
    Unavailable,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxRuntimeProcess {
    pub pid: u32,
    pub parent_pid: Option<u32>,
    pub role: NginxProcessRole,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
    pub started_at: Option<String>,
    pub uptime_seconds: u64,
    pub executable_verified: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxRuntimeDetails {
    pub instance_id: String,
    pub observed_at: String,
    pub status: NginxRuntimeStatus,
    pub master_pid: Option<u32>,
    pub worker_count: usize,
    pub total_cpu_usage: f32,
    pub total_memory_bytes: u64,
    pub started_at: Option<String>,
    pub uptime_seconds: Option<u64>,
    pub processes: Vec<NginxRuntimeProcess>,
    pub process_metrics: NginxRuntimeMetricAvailability,
    pub listeners: Vec<String>,
    pub listener_metrics: NginxRuntimeMetricAvailability,
    pub connection_metrics: NginxRuntimeMetricAvailability,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxStatusEvent {
    pub generation: u64,
    pub sequence: u64,
    pub observed_at: String,
    pub instance: Option<NginxInstance>,
    pub runtime_details: Option<NginxRuntimeDetails>,
    pub operation_phase: Option<NginxOperationPhase>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxStatusSubscription {
    pub subscription_id: u64,
    pub initial_event: NginxStatusEvent,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeNginxInstanceInput {
    pub instance_id: String,
    pub channel: NginxReleaseChannel,
    pub target_version: String,
    pub backup_retention_count: u8,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxUpgradeProgress {
    pub phase: NginxOperationPhase,
    pub progress: u8,
    pub message_code: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxUpgradeResult {
    pub from_version: String,
    pub target_version: String,
    pub backup_id: Option<String>,
    pub success: bool,
    pub rolled_back: bool,
    pub rollback_succeeded: Option<bool>,
    pub error_code: Option<String>,
    pub resulting_status: NginxRuntimeStatus,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetNginxOperationHistoryInput {
    pub instance_id: Option<String>,
    pub limit: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxSystemServiceCandidate {
    pub discovery_id: String,
    pub display_name: String,
    pub backend: NginxControlBackend,
    pub domain: String,
    pub read_only: bool,
    pub expires_at: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectNginxSystemServiceInput {
    pub discovery_id: String,
    pub instance_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxSystemServiceInspection {
    pub inspection_id: String,
    pub display_name: String,
    pub backend: NginxControlBackend,
    pub read_only: bool,
    pub executable_matches: bool,
    pub expires_at: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterNginxSystemServiceInput {
    pub inspection_id: String,
}
