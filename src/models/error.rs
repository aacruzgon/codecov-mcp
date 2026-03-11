use serde::Deserialize;

/// Body returned by the Codecov API on error responses.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ApiErrorResponse {
    pub detail: Option<String>,
}
