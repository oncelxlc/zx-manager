use std::collections::HashMap;

use super::dto::GpuInformation;

#[derive(Debug, Clone, PartialEq, Eq)]
struct AdapterCandidate {
    name: String,
    vendor_id: Option<u32>,
    device_id: Option<u32>,
    device_type: String,
    backend: String,
    driver: Option<String>,
    driver_info: Option<String>,
}

impl AdapterCandidate {
    fn physical_key(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.name.trim().to_lowercase(),
            self.vendor_id.unwrap_or_default(),
            self.device_id.unwrap_or_default(),
            self.device_type
        )
    }

    fn into_dto(self) -> GpuInformation {
        GpuInformation {
            name: self.name,
            vendor_id: self.vendor_id,
            device_id: self.device_id,
            device_type: self.device_type,
            backend: self.backend,
            driver: self.driver,
            driver_info: self.driver_info,
            dedicated_memory_bytes: None,
            shared_memory_bytes: None,
        }
    }
}

pub fn collect_gpus() -> Vec<GpuInformation> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapters =
        tauri::async_runtime::block_on(instance.enumerate_adapters(wgpu::Backends::all()));

    let candidates = adapters
        .into_iter()
        .filter_map(|adapter| candidate_from_info(adapter.get_info()))
        .collect();

    deduplicate(candidates)
        .into_iter()
        .map(AdapterCandidate::into_dto)
        .collect()
}

fn candidate_from_info(info: wgpu::AdapterInfo) -> Option<AdapterCandidate> {
    if info.backend == wgpu::Backend::Noop || info.name.trim().is_empty() {
        return None;
    }

    Some(AdapterCandidate {
        name: info.name,
        vendor_id: (info.vendor != 0).then_some(info.vendor),
        device_id: (info.device != 0).then_some(info.device),
        device_type: map_device_type(info.device_type).to_string(),
        backend: map_backend(info.backend).to_string(),
        driver: non_empty(info.driver),
        driver_info: non_empty(info.driver_info),
    })
}

fn non_empty(value: String) -> Option<String> {
    (!value.trim().is_empty()).then_some(value)
}

fn map_device_type(device_type: wgpu::DeviceType) -> &'static str {
    match device_type {
        wgpu::DeviceType::IntegratedGpu => "integrated",
        wgpu::DeviceType::DiscreteGpu => "discrete",
        wgpu::DeviceType::VirtualGpu => "virtual",
        wgpu::DeviceType::Cpu => "cpu",
        wgpu::DeviceType::Other => "other",
    }
}

fn map_backend(backend: wgpu::Backend) -> &'static str {
    match backend {
        wgpu::Backend::Vulkan => "vulkan",
        wgpu::Backend::Metal => "metal",
        wgpu::Backend::Dx12 => "dx12",
        wgpu::Backend::Gl => "gl",
        wgpu::Backend::BrowserWebGpu => "browser-webgpu",
        wgpu::Backend::Noop => "noop",
    }
}

fn backend_priority(backend: &str) -> u8 {
    #[cfg(target_os = "windows")]
    {
        return match backend {
            "dx12" => 0,
            "vulkan" => 1,
            "gl" => 2,
            _ => 3,
        };
    }
    #[cfg(target_os = "macos")]
    {
        return match backend {
            "metal" => 0,
            "vulkan" => 1,
            "gl" => 2,
            _ => 3,
        };
    }
    #[cfg(target_os = "linux")]
    {
        return match backend {
            "vulkan" => 0,
            "gl" => 1,
            _ => 2,
        };
    }
    #[allow(unreachable_code)]
    0
}

fn deduplicate(candidates: Vec<AdapterCandidate>) -> Vec<AdapterCandidate> {
    let mut by_physical_device = HashMap::<String, AdapterCandidate>::new();

    for candidate in candidates {
        let key = candidate.physical_key();
        match by_physical_device.get(&key) {
            Some(existing)
                if backend_priority(&existing.backend) <= backend_priority(&candidate.backend) => {}
            _ => {
                by_physical_device.insert(key, candidate);
            }
        }
    }

    let mut deduplicated: Vec<_> = by_physical_device.into_values().collect();
    deduplicated.sort_by(|left, right| left.name.cmp(&right.name));
    deduplicated
}

#[cfg(test)]
mod tests {
    use super::{backend_priority, deduplicate, map_backend, map_device_type, AdapterCandidate};

    fn candidate(backend: &str) -> AdapterCandidate {
        AdapterCandidate {
            name: "Example GPU".to_string(),
            vendor_id: Some(1),
            device_id: Some(2),
            device_type: "discrete".to_string(),
            backend: backend.to_string(),
            driver: None,
            driver_info: None,
        }
    }

    #[test]
    fn gpu_type_and_backend_mappings_are_stable() {
        assert_eq!(
            map_device_type(wgpu::DeviceType::IntegratedGpu),
            "integrated"
        );
        assert_eq!(map_device_type(wgpu::DeviceType::Cpu), "cpu");
        assert_eq!(map_backend(wgpu::Backend::Dx12), "dx12");
        assert_eq!(map_backend(wgpu::Backend::BrowserWebGpu), "browser-webgpu");
    }

    #[test]
    fn duplicate_backends_keep_the_platform_preferred_adapter() {
        let values = deduplicate(vec![
            candidate("gl"),
            candidate("vulkan"),
            candidate("dx12"),
        ]);

        assert_eq!(values.len(), 1);
        let expected = ["gl", "vulkan", "dx12"]
            .into_iter()
            .min_by_key(|backend| backend_priority(backend))
            .expect("a backend exists");
        assert_eq!(values[0].backend, expected);
    }

    #[test]
    fn different_physical_devices_are_not_deduplicated() {
        let mut second = candidate("vulkan");
        second.device_id = Some(3);

        assert_eq!(deduplicate(vec![candidate("vulkan"), second]).len(), 2);
    }
}
