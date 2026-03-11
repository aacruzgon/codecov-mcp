use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{codecov_client::CodecovClient, error::AppError};

#[derive(Deserialize, JsonSchema)]
pub struct GetCommitCoverageInput {
    /// The commit SHA to retrieve coverage for.
    pub sha: String,
    /// When true, includes per-file coverage breakdown in the response.
    pub include_files: Option<bool>,
}

#[derive(Serialize)]
pub struct CommitCoverageOutput {
    pub commit_sha: String,
    pub branch: Option<String>,
    pub state: String,
    pub coverage_pct: Option<f64>,
    pub lines: Option<i64>,
    pub hits: Option<i64>,
    pub misses: Option<i64>,
    pub partials: Option<i64>,
    pub branches: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<FileOutput>>,
    pub codecov_url: String,
}

#[derive(Serialize)]
pub struct FileOutput {
    pub name: String,
    pub coverage_pct: Option<f64>,
}

pub async fn get_commit_coverage(
    client: &CodecovClient,
    input: GetCommitCoverageInput,
) -> Result<CommitCoverageOutput, AppError> {
    let detail = client.get_commit_detail(&input.sha).await?;

    let state = detail.state.as_deref().unwrap_or("unknown").to_string();

    // Surface pending/error states as explicit errors so the agent can react.
    if state == "pending" || state == "error" {
        return Err(AppError::CoverageNotReady { state });
    }

    let totals = detail.totals.as_ref();
    let coverage_pct = totals.and_then(|t| t.coverage);
    let lines = totals.and_then(|t| t.lines);
    let hits = totals.and_then(|t| t.hits);
    let misses = totals.and_then(|t| t.misses);
    let partials = totals.and_then(|t| t.partials);
    let branches = totals.and_then(|t| t.branches);

    let files = if input.include_files.unwrap_or(false) {
        let report = client.get_commit_report(&input.sha).await?;
        Some(
            report
                .files
                .unwrap_or_default()
                .into_iter()
                .map(|f| FileOutput {
                    name: f.name,
                    coverage_pct: f.totals.and_then(|t| t.coverage),
                })
                .collect(),
        )
    } else {
        None
    };

    Ok(CommitCoverageOutput {
        commit_sha: detail.commitid,
        branch: detail.branch,
        state,
        coverage_pct,
        lines,
        hits,
        misses,
        partials,
        branches,
        files,
        codecov_url: client.app_commit_url(&input.sha),
    })
}
