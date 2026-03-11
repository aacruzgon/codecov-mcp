use rmcp::model::{ErrorCode, ErrorData};

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Coverage not ready (state: {state})")]
    CoverageNotReady { state: String },

    #[error("No coverage data: {0}")]
    NoCoverageData(String),

    #[error("Codecov API error ({status}): {message}")]
    Api { status: u16, message: String },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

impl From<AppError> for ErrorData {
    fn from(err: AppError) -> Self {
        match &err {
            AppError::Config(_) => ErrorData {
                code: ErrorCode::INVALID_PARAMS,
                message: err.to_string().into(),
                data: None,
            },
            AppError::NotFound(_) => ErrorData {
                code: ErrorCode::INVALID_PARAMS,
                message: err.to_string().into(),
                data: None,
            },
            AppError::CoverageNotReady { .. } => ErrorData {
                code: ErrorCode::INTERNAL_ERROR,
                message: err.to_string().into(),
                data: None,
            },
            AppError::NoCoverageData(_) => ErrorData {
                code: ErrorCode::INTERNAL_ERROR,
                message: err.to_string().into(),
                data: None,
            },
            AppError::Api { .. } => ErrorData {
                code: ErrorCode::INTERNAL_ERROR,
                message: err.to_string().into(),
                data: None,
            },
            AppError::Serialization(_) => ErrorData {
                code: ErrorCode::INTERNAL_ERROR,
                message: err.to_string().into(),
                data: None,
            },
            AppError::Http(_) => ErrorData {
                code: ErrorCode::INTERNAL_ERROR,
                message: err.to_string().into(),
                data: None,
            },
        }
    }
}
