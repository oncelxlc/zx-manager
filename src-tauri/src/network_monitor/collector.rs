use super::dto::RawApplicationSample;
use super::error::NetworkMonitorResult;
#[cfg(not(windows))]
use super::error::{NetworkMonitorError, NetworkMonitorErrorCode};

pub trait PlatformNetworkCollector: Send {
    fn prepare(&mut self) -> NetworkMonitorResult<()>;
    fn collect(&mut self) -> NetworkMonitorResult<RawApplicationSample>;
    fn pause(&mut self) -> NetworkMonitorResult<()>;
    fn shutdown(&mut self);
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
    fn prepare(&mut self) -> NetworkMonitorResult<()> {
        if let Some(helper) = self.helper.as_mut() {
            if helper.prepare().is_ok() {
                return Ok(());
            }
            self.helper.take();
        }
        let mut helper = super::helper::HelperClient::start()?;
        helper.prepare()?;
        self.helper = Some(helper);
        Ok(())
    }

    fn collect(&mut self) -> NetworkMonitorResult<RawApplicationSample> {
        let result = self
            .helper
            .as_mut()
            .ok_or_else(|| {
                super::error::NetworkMonitorError::new(
                    super::error::NetworkMonitorErrorCode::HelperDisconnected,
                    "network helper is not prepared",
                )
            })?
            .sample();
        if result.is_err() {
            self.helper.take();
        }
        result
    }

    fn pause(&mut self) -> NetworkMonitorResult<()> {
        let Some(helper) = self.helper.as_mut() else {
            return Ok(());
        };
        let result = helper.pause();
        if result.is_err() {
            self.helper.take();
        }
        result
    }

    fn shutdown(&mut self) {
        self.helper.take();
    }
}

#[cfg(not(windows))]
#[derive(Default)]
struct HostPlatformCollector;

#[cfg(not(windows))]
impl PlatformNetworkCollector for HostPlatformCollector {
    fn prepare(&mut self) -> NetworkMonitorResult<()> {
        Err(NetworkMonitorError::new(
            NetworkMonitorErrorCode::UnsupportedPlatform,
            "application network monitoring is supported on Windows only",
        ))
    }

    fn collect(&mut self) -> NetworkMonitorResult<RawApplicationSample> {
        Err(NetworkMonitorError::new(
            NetworkMonitorErrorCode::UnsupportedPlatform,
            "application network monitoring is supported on Windows only",
        ))
    }

    fn pause(&mut self) -> NetworkMonitorResult<()> {
        Ok(())
    }

    fn shutdown(&mut self) {}
}
