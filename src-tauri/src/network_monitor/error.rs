use serde::Serialize;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkMonitorError {
    pub code: NetworkMonitorErrorCode,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub enum NetworkMonitorErrorCode {
    InvalidRequest,
    UnsupportedPlatform,
    ElevationCancelled,
    HelperDisconnected,
    ProtocolMismatch,
    CollectorUnavailable,
    StorageUnavailable,
    Internal,
}

impl NetworkMonitorError {
    pub fn new(code: NetworkMonitorErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn storage(error: impl Display) -> Self {
        Self::new(
            NetworkMonitorErrorCode::StorageUnavailable,
            error.to_string(),
        )
    }
}

impl Display for NetworkMonitorError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for NetworkMonitorError {}

pub type NetworkMonitorResult<T> = Result<T, NetworkMonitorError>;
