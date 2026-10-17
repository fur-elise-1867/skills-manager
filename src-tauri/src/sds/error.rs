use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdsError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub operation_id: Option<String>,
}

impl SdsError {
    pub fn new(code: &str, message: &str, retryable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable,
            operation_id: None,
        }
    }
    pub fn invalid(message: &str) -> Self {
        Self {
            code: "INVALID_CONFIGURATION".into(),
            message: message.into(),
            retryable: false,
            operation_id: None,
        }
    }
}

impl std::fmt::Display for SdsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for SdsError {}
