use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SystemWarningCode {
    CpuUsageUnavailable,
    DisksUnavailable,
    GpuUnavailable,
    RuntimeTargetUnavailable,
    UnsupportedSystem,
    WebviewVersionUnavailable,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemSummary {
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub os_long_version: Option<String>,
    pub architecture: Option<String>,
    pub host_name: Option<String>,
    pub cpu_model: Option<String>,
    pub logical_core_count: Option<u32>,
    pub total_memory_bytes: Option<u64>,
    pub collected_at: String,
    pub warnings: Vec<SystemWarningCode>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemDetails {
    pub platform: String,
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub os_long_version: Option<String>,
    pub kernel_version: Option<String>,
    pub architecture: Option<String>,
    pub host_name: Option<String>,
    pub uptime_seconds: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuInformation {
    pub model: Option<String>,
    pub vendor: Option<String>,
    pub physical_core_count: Option<u32>,
    pub logical_core_count: Option<u32>,
    pub frequency_mhz: Option<u64>,
    pub usage_percent: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryInformation {
    pub total_bytes: Option<u64>,
    pub used_bytes: Option<u64>,
    pub available_bytes: Option<u64>,
    pub usage_percent: Option<f32>,
    pub swap_total_bytes: Option<u64>,
    pub swap_used_bytes: Option<u64>,
    pub swap_usage_percent: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuInformation {
    pub name: String,
    pub vendor_id: Option<u32>,
    pub device_id: Option<u32>,
    pub device_type: String,
    pub backend: String,
    pub driver: Option<String>,
    pub driver_info: Option<String>,
    pub dedicated_memory_bytes: Option<u64>,
    pub shared_memory_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskInformation {
    pub name: Option<String>,
    pub file_system: Option<String>,
    pub mount_point: Option<String>,
    pub kind: String,
    pub removable: bool,
    pub total_bytes: Option<u64>,
    pub available_bytes: Option<u64>,
    pub used_bytes: Option<u64>,
    pub usage_percent: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInformation {
    pub app_name: String,
    pub app_version: String,
    pub tauri_version: String,
    pub target_triple: Option<String>,
    pub webview_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataAvailability {
    pub cpu_usage: bool,
    pub memory: bool,
    pub swap: bool,
    pub disks: bool,
    pub gpu_basic: bool,
    pub gpu_memory: bool,
    pub runtime_target: bool,
    pub webview_version: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInformation {
    pub summary: SystemSummary,
    pub system: SystemDetails,
    pub cpu: CpuInformation,
    pub memory: MemoryInformation,
    pub gpus: Vec<GpuInformation>,
    pub disks: Vec<DiskInformation>,
    pub runtime: RuntimeInformation,
    pub availability: DataAvailability,
    pub collected_at: String,
    pub warnings: Vec<SystemWarningCode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: String,
    pub message: String,
}

impl CommandError {
    pub fn collection(message: impl Into<String>) -> Self {
        Self {
            code: "systemInformationCollectionFailed".to_string(),
            message: message.into(),
        }
    }

    pub fn task(message: impl Into<String>) -> Self {
        Self {
            code: "systemInformationTaskFailed".to_string(),
            message: message.into(),
        }
    }
}

pub fn optional_percent(used: Option<u64>, total: Option<u64>) -> Option<f32> {
    match (used, total) {
        (Some(used), Some(total)) if total > 0 => {
            Some(((used as f64 / total as f64) * 100.0).clamp(0.0, 100.0) as f32)
        }
        _ => None,
    }
}

pub fn sanitize_percent(value: f32) -> Option<f32> {
    value.is_finite().then(|| value.clamp(0.0, 100.0))
}

#[cfg(test)]
mod tests {
    use super::{
        optional_percent, sanitize_percent, CommandError, GpuInformation, SystemWarningCode,
    };

    #[test]
    fn percent_helpers_handle_zero_and_invalid_values() {
        assert_eq!(optional_percent(Some(5), Some(10)), Some(50.0));
        assert_eq!(optional_percent(Some(5), Some(0)), None);
        assert_eq!(optional_percent(None, Some(10)), None);
        assert_eq!(sanitize_percent(f32::NAN), None);
        assert_eq!(sanitize_percent(120.0), Some(100.0));
    }

    #[test]
    fn dto_serialization_is_camel_case_and_preserves_nulls() {
        let gpu = GpuInformation {
            name: "Test GPU".to_string(),
            vendor_id: None,
            device_id: Some(7),
            device_type: "integrated".to_string(),
            backend: "dx12".to_string(),
            driver: None,
            driver_info: None,
            dedicated_memory_bytes: None,
            shared_memory_bytes: None,
        };
        let value = serde_json::to_value(gpu).expect("GPU should serialize");

        assert!(value
            .get("vendorId")
            .is_some_and(serde_json::Value::is_null));
        assert_eq!(value["deviceType"], "integrated");
        assert_eq!(value["backend"], "dx12");
        assert!(value.get("vendor_id").is_none());
    }

    #[test]
    fn warning_and_command_error_serialization_are_stable() {
        let warning =
            serde_json::to_value(SystemWarningCode::GpuUnavailable).expect("warning serializes");
        let error =
            serde_json::to_value(CommandError::collection("failed")).expect("error serializes");

        assert_eq!(warning, "gpuUnavailable");
        assert_eq!(error["code"], "systemInformationCollectionFailed");
        assert_eq!(error["message"], "failed");
    }
}
