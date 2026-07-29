use super::dto::RawApplicationSample;
use super::error::NetworkMonitorResult;
#[cfg(not(windows))]
use super::error::{NetworkMonitorError, NetworkMonitorErrorCode};

pub trait PlatformNetworkCollector: Send {
    fn collect(&mut self) -> NetworkMonitorResult<RawApplicationSample>;
}

pub fn create_platform_collector() -> Box<dyn PlatformNetworkCollector> {
    Box::new(HostPlatformCollector::default())
}

#[cfg(windows)]
#[derive(Default)]
struct HostPlatformCollector {
    helper: Option<super::helper::HelperClient>,
}

#[cfg(windows)]
impl PlatformNetworkCollector for HostPlatformCollector {
    fn collect(&mut self) -> NetworkMonitorResult<RawApplicationSample> {
        if self.helper.is_none() {
            self.helper = Some(super::helper::HelperClient::start()?);
        }
        self.helper
            .as_mut()
            .expect("network helper initialized")
            .sample()
    }
}

#[cfg(not(windows))]
#[derive(Default)]
struct HostPlatformCollector;

#[cfg(not(windows))]
impl PlatformNetworkCollector for HostPlatformCollector {
    fn collect(&mut self) -> NetworkMonitorResult<RawApplicationSample> {
        Err(NetworkMonitorError::new(
            NetworkMonitorErrorCode::UnsupportedPlatform,
            "application network monitoring is supported on Windows only",
        ))
    }
}
