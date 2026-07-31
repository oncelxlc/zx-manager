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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterNginxInstanceInput {
    pub inspection_id: String,
    pub name: String,
    pub authorization_level: NginxAuthorizationLevel,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizeNginxRootInput {
    pub instance_id: String,
    pub selection_id: String,
}
