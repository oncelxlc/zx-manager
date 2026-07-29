use std::sync::Mutex;
use std::thread;

use chrono::{SecondsFormat, Utc};
use sysinfo::{CpuRefreshKind, Disks, System, MINIMUM_CPU_UPDATE_INTERVAL};
use tauri::AppHandle;

use super::dto::{
    optional_percent, sanitize_percent, CommandError, CpuInformation, DataAvailability,
    DiskInformation, MemoryInformation, RuntimeInformation, SystemDetails, SystemInformation,
    SystemSummary, SystemWarningCode,
};
use super::gpu::collect_gpus;

#[derive(Default)]
pub struct SystemInformationCollector {
    refresh_lock: Mutex<()>,
}

impl SystemInformationCollector {
    pub fn collect_summary(&self) -> Result<SystemSummary, CommandError> {
        let _refresh_guard = self
            .refresh_lock
            .lock()
            .map_err(|_| CommandError::collection("system information refresh lock is poisoned"))?;
        Ok(collect_summary_snapshot())
    }

    pub fn collect_information(&self, app: &AppHandle) -> Result<SystemInformation, CommandError> {
        let _refresh_guard = self
            .refresh_lock
            .lock()
            .map_err(|_| CommandError::collection("system information refresh lock is poisoned"))?;

        let collected_at = collected_at();
        let mut system = System::new();
        system.refresh_cpu_list(CpuRefreshKind::everything());
        system.refresh_cpu_usage();
        thread::sleep(MINIMUM_CPU_UPDATE_INTERVAL);
        system.refresh_cpu_all();
        system.refresh_memory();

        let summary = summary_from_system(&system, collected_at.clone());
        let cpu_usage = sanitize_percent(system.global_cpu_usage());
        let total_memory = non_zero(system.total_memory());
        let used_memory = non_zero_or_zero(system.used_memory(), total_memory);
        let available_memory = non_zero_or_zero(system.available_memory(), total_memory);
        let swap_total = non_zero(system.total_swap());
        let swap_used = non_zero_or_zero(system.used_swap(), swap_total);
        let first_cpu = system.cpus().first();
        let gpus = collect_gpus();
        let disks = collect_disks();
        let target_triple = tauri::utils::platform::target_triple().ok();
        let webview_version = tauri::webview_version().ok();
        let mut warnings = summary.warnings.clone();

        if cpu_usage.is_none() {
            warnings.push(SystemWarningCode::CpuUsageUnavailable);
        }
        if disks.is_empty() {
            warnings.push(SystemWarningCode::DisksUnavailable);
        }
        if gpus.is_empty() {
            warnings.push(SystemWarningCode::GpuUnavailable);
        }
        if target_triple.is_none() {
            warnings.push(SystemWarningCode::RuntimeTargetUnavailable);
        }
        if webview_version.is_none() {
            warnings.push(SystemWarningCode::WebviewVersionUnavailable);
        }
        warnings.sort_by_key(|warning| format!("{warning:?}"));
        warnings.dedup();

        let package = app.package_info();
        let availability = DataAvailability {
            cpu_usage: cpu_usage.is_some(),
            memory: total_memory.is_some(),
            swap: swap_total.is_some(),
            disks: !disks.is_empty(),
            gpu_basic: !gpus.is_empty(),
            gpu_memory: false,
            runtime_target: target_triple.is_some(),
            webview_version: webview_version.is_some(),
        };

        Ok(SystemInformation {
            system: SystemDetails {
                platform: platform_name().to_string(),
                os_name: System::name(),
                os_version: System::os_version(),
                os_long_version: System::long_os_version(),
                kernel_version: System::kernel_version(),
                architecture: non_empty(System::cpu_arch()),
                host_name: System::host_name(),
                uptime_seconds: Some(System::uptime()),
            },
            cpu: CpuInformation {
                model: first_cpu.and_then(|cpu| non_empty(cpu.brand().to_string())),
                vendor: first_cpu.and_then(|cpu| non_empty(cpu.vendor_id().to_string())),
                physical_core_count: System::physical_core_count().and_then(to_u32),
                logical_core_count: to_u32(system.cpus().len()),
                frequency_mhz: first_cpu.and_then(|cpu| non_zero(cpu.frequency())),
                usage_percent: cpu_usage,
            },
            memory: MemoryInformation {
                total_bytes: total_memory,
                used_bytes: used_memory,
                available_bytes: available_memory,
                usage_percent: optional_percent(used_memory, total_memory),
                swap_total_bytes: swap_total,
                swap_used_bytes: swap_used,
                swap_usage_percent: optional_percent(swap_used, swap_total),
            },
            gpus,
            disks,
            runtime: RuntimeInformation {
                app_name: package.name.clone(),
                app_version: package.version.to_string(),
                tauri_version: tauri::VERSION.to_string(),
                target_triple,
                webview_version,
            },
            availability,
            summary,
            collected_at,
            warnings,
        })
    }
}

fn collect_summary_snapshot() -> SystemSummary {
    let mut system = System::new();
    system.refresh_cpu_list(CpuRefreshKind::everything());
    system.refresh_memory();
    summary_from_system(&system, collected_at())
}

fn summary_from_system(system: &System, collected_at: String) -> SystemSummary {
    let mut warnings = Vec::new();
    if platform_name() == "unknown" {
        warnings.push(SystemWarningCode::UnsupportedSystem);
    }

    SystemSummary {
        os_name: System::name(),
        os_version: System::os_version(),
        os_long_version: System::long_os_version(),
        architecture: non_empty(System::cpu_arch()),
        host_name: System::host_name(),
        cpu_model: system
            .cpus()
            .first()
            .and_then(|cpu| non_empty(cpu.brand().to_string())),
        logical_core_count: to_u32(system.cpus().len()),
        total_memory_bytes: non_zero(system.total_memory()),
        collected_at,
        warnings,
    }
}

fn collect_disks() -> Vec<DiskInformation> {
    let disks = Disks::new_with_refreshed_list();
    disks
        .list()
        .iter()
        .map(|disk| {
            let total = non_zero(disk.total_space());
            let available = non_zero_or_zero(disk.available_space(), total);
            let used = match (total, available) {
                (Some(total), Some(available)) => Some(total.saturating_sub(available)),
                _ => None,
            };

            DiskInformation {
                name: os_string(disk.name()),
                file_system: os_string(disk.file_system()),
                mount_point: non_empty(disk.mount_point().to_string_lossy().into_owned()),
                kind: format!("{:?}", disk.kind()).to_lowercase(),
                removable: disk.is_removable(),
                total_bytes: total,
                available_bytes: available,
                used_bytes: used,
                usage_percent: optional_percent(used, total),
            }
        })
        .collect()
}

fn os_string(value: &std::ffi::OsStr) -> Option<String> {
    non_empty(value.to_string_lossy().into_owned())
}

fn non_empty(value: String) -> Option<String> {
    (!value.trim().is_empty()).then_some(value)
}

fn non_zero(value: u64) -> Option<u64> {
    (value > 0).then_some(value)
}

fn non_zero_or_zero(value: u64, total: Option<u64>) -> Option<u64> {
    total.map(|_| value)
}

fn to_u32(value: usize) -> Option<u32> {
    (value > 0).then(|| u32::try_from(value).ok()).flatten()
}

fn collected_at() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn platform_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "unknown"
    }
}
