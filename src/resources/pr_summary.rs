use serde::Serialize;

use crate::{codecov_client::CodecovClient, error::AppError};

/// URI template advertised to MCP clients.
pub const URI_TEMPLATE: &str = "codecov://pr/{pull_id}/summary";

/// Parse `pull_id` from a URI of the form `codecov://pr/{pull_id}/summary`.
pub fn parse_uri(uri: &str) -> Option<u64> {
    // Expected: ["codecov:", "", "pr", "{pull_id}", "summary"]
    let parts: Vec<&str> = uri.splitn(6, '/').collect();
    if parts.len() == 5
        && parts[0] == "codecov:"
        && parts[1].is_empty()
        && parts[2] == "pr"
        && parts[4] == "summary"
    {
        parts[3].parse().ok()
    } else {
        None
    }
}

#[derive(Serialize)]
struct PrSummaryContent {
    pull_id: u64,
    service: String,
    owner: String,
    repo: String,
    base_commit: Option<String>,
    head_commit: Option<String>,
    base_coverage_pct: Option<f64>,
    head_coverage_pct: Option<f64>,
    patch_coverage_pct: Option<f64>,
    patch_lines: Option<i64>,
    patch_hits: Option<i64>,
    patch_misses: Option<i64>,
    codecov_url: String,
}

/// Fetch and serialize the PR summary resource for `pull_id`.
pub async fn fetch(
    client: &CodecovClient,
    pull_id: u64,
) -> Result<String, AppError> {
    let summary = client.get_comparison_summary(pull_id).await?;

    let content = PrSummaryContent {
        pull_id,
        service: client.service.clone(),
        owner: client.owner.clone(),
        repo: client.repo.clone(),
        base_commit: summary.base_commit,
        head_commit: summary.head_commit,
        base_coverage_pct: summary.base.as_ref().and_then(|t| t.coverage),
        head_coverage_pct: summary.head.as_ref().and_then(|t| t.coverage),
        patch_coverage_pct: summary.patch.as_ref().and_then(|t| t.coverage),
        patch_lines: summary.patch.as_ref().and_then(|t| t.lines),
        patch_hits: summary.patch.as_ref().and_then(|t| t.hits),
        patch_misses: summary.patch.as_ref().and_then(|t| t.misses),
        codecov_url: client.app_pull_url(pull_id),
    };

    serde_json::to_string_pretty(&content).map_err(AppError::Serialization)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_uri_valid() {
        assert_eq!(parse_uri("codecov://pr/42/summary"), Some(42));
        assert_eq!(parse_uri("codecov://pr/1234567/summary"), Some(1234567));
    }

    #[test]
    fn test_parse_uri_invalid() {
        assert_eq!(parse_uri("codecov://pr/abc/summary"), None);
        assert_eq!(parse_uri("codecov://pr/42/other"), None);
        assert_eq!(parse_uri("https://example.com/pr/42"), None);
        assert_eq!(parse_uri("codecov://pr/summary"), None);
    }
}
