use serde::Serialize;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxError {
    pub code: &'static str,
    pub message: String,
}

impl NginxError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn io(context: &'static str, error: std::io::Error) -> Self {
        Self::new("NGINX_IO_FAILED", format!("{context}: {error}"))
    }
}

impl Display for NginxError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for NginxError {}

pub type NginxResult<T> = Result<T, NginxError>;
