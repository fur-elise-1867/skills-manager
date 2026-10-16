use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SdsError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub operation_id: Option<String>,
}

impl SdsError {
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
