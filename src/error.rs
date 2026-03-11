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

    #[error("Unauthorized: invalid or missing Codecov token")]
    Unauthorized,

    #[error("Forbidden: token does not have access to this repository")]
    Forbidden,

    #[error("Rate limited by Codecov API — please retry later")]
    RateLimited,

    #[error("Codecov API error ({status}): {message}")]
    Api { status: u16, message: String },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn to_error_data(err: AppError) -> ErrorData {
        ErrorData::from(err)
    }

    #[test]
    fn test_config_maps_to_invalid_params() {
        let d = to_error_data(AppError::Config("bad config".into()));
        assert_eq!(d.code, ErrorCode::INVALID_PARAMS);
        assert!(d.message.contains("bad config"));
    }

    #[test]
    fn test_not_found_maps_to_invalid_params() {
        let d = to_error_data(AppError::NotFound("commit abc not found".into()));
        assert_eq!(d.code, ErrorCode::INVALID_PARAMS);
        assert!(d.message.contains("commit abc not found"));
    }

    #[test]
    fn test_coverage_not_ready_maps_to_internal_error() {
        let d = to_error_data(AppError::CoverageNotReady { state: "pending".into() });
        assert_eq!(d.code, ErrorCode::INTERNAL_ERROR);
        assert!(d.message.contains("pending"));
    }

    #[test]
    fn test_no_coverage_data_maps_to_internal_error() {
        let d = to_error_data(AppError::NoCoverageData("no data for PR 1".into()));
        assert_eq!(d.code, ErrorCode::INTERNAL_ERROR);
        assert!(d.message.contains("no data for PR 1"));
    }

    #[test]
    fn test_unauthorized_maps_to_invalid_params() {
        let d = to_error_data(AppError::Unauthorized);
        assert_eq!(d.code, ErrorCode::INVALID_PARAMS);
        assert!(d.message.contains("Unauthorized"));
    }

    #[test]
    fn test_forbidden_maps_to_invalid_params() {
        let d = to_error_data(AppError::Forbidden);
        assert_eq!(d.code, ErrorCode::INVALID_PARAMS);
        assert!(d.message.contains("Forbidden"));
    }

    #[test]
    fn test_rate_limited_maps_to_internal_error() {
        let d = to_error_data(AppError::RateLimited);
        assert_eq!(d.code, ErrorCode::INTERNAL_ERROR);
        assert!(d.message.contains("Rate limited"));
    }

    #[test]
    fn test_api_error_maps_to_internal_error() {
        let d = to_error_data(AppError::Api { status: 503, message: "service unavailable".into() });
        assert_eq!(d.code, ErrorCode::INTERNAL_ERROR);
        assert!(d.message.contains("503"));
    }

    #[test]
    fn test_serialization_error_maps_to_internal_error() {
        let err: serde_json::Error = serde_json::from_str::<serde_json::Value>("{bad}").unwrap_err();
        let d = to_error_data(AppError::Serialization(err));
        assert_eq!(d.code, ErrorCode::INTERNAL_ERROR);
        assert!(d.message.contains("Serialization"));
    }
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
            AppError::Unauthorized => ErrorData {
                code: ErrorCode::INVALID_PARAMS,
                message: err.to_string().into(),
                data: None,
            },
            AppError::Forbidden => ErrorData {
                code: ErrorCode::INVALID_PARAMS,
                message: err.to_string().into(),
                data: None,
            },
            AppError::RateLimited => ErrorData {
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
