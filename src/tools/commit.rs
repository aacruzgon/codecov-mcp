use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
pub struct GetCommitCoverageInput {
    /// The commit SHA to look up coverage for
    pub sha: String,
    /// Whether to include per-file coverage breakdown
    pub include_files: Option<bool>,
}

pub async fn get_commit_coverage(
    input: GetCommitCoverageInput,
) -> Result<serde_json::Value, crate::error::AppError> {
    // Phase 0 stub — hardcoded response until real API client is implemented
    let response = serde_json::json!({
        "commit_sha": input.sha,
        "branch": "main",
        "state": "complete",
        "coverage_pct": 75.0,
        "lines": 100,
        "hits": 75,
        "misses": 20,
        "partials": 5,
        "branches": 0,
        "files": if input.include_files.unwrap_or(false) {
            serde_json::json!([
                {
                    "name": "src/main.rs",
                    "coverage_pct": 80.0
                }
            ])
        } else {
            serde_json::Value::Null
        },
        "codecov_url": "https://app.codecov.io/stub"
    });

    Ok(response)
}
