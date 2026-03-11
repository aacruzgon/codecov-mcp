use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{codecov_client::CodecovClient, error::AppError};

#[derive(Deserialize, JsonSchema)]
pub struct GetChangedFilesCoverageInput {
    /// The pull request number to retrieve coverage for.
    pub pull_id: u64,
    /// When true, includes per-file patch coverage breakdown in the response.
    /// Defaults to true.
    pub include_patch_coverage: Option<bool>,
}

#[derive(Serialize)]
pub struct ChangedFilesCoverageOutput {
    pub pull_id: u64,
    pub base_commit: Option<String>,
    pub head_commit: Option<String>,
    pub totals: PatchTotalsOutput,
    pub files: Vec<FilePatchOutput>,
    pub codecov_url: String,
}

#[derive(Serialize)]
pub struct PatchTotalsOutput {
    pub base_coverage_pct: Option<f64>,
    pub head_coverage_pct: Option<f64>,
    pub patch_coverage_pct: Option<f64>,
    pub patch_lines: Option<i64>,
    pub patch_hits: Option<i64>,
    pub patch_misses: Option<i64>,
}

#[derive(Serialize)]
pub struct FilePatchOutput {
    pub name: String,
    /// `None` for newly created files.
    pub base_coverage_pct: Option<f64>,
    pub head_coverage_pct: Option<f64>,
    pub patch_coverage_pct: Option<f64>,
    pub added_lines: Option<i64>,
    pub covered_added_lines: Option<i64>,
    pub uncovered_added_lines: Option<i64>,
}

pub async fn get_changed_files_coverage(
    client: &CodecovClient,
    input: GetChangedFilesCoverageInput,
) -> Result<ChangedFilesCoverageOutput, AppError> {
    let pull_id = input.pull_id;

    // Fire both requests concurrently — summary is fast, impacted_files may need polling.
    let (summary, impacted) = tokio::try_join!(
        client.get_comparison_summary(pull_id),
        client.poll_until_processed(pull_id),
    )?;

    let raw_files = impacted.files.unwrap_or_default();
    if raw_files.is_empty() && summary.patch.is_none() {
        return Err(AppError::NoCoverageData(format!(
            "no coverage data available for pull request {pull_id}"
        )));
    }

    let totals = PatchTotalsOutput {
        base_coverage_pct: summary.base.as_ref().and_then(|t| t.coverage),
        head_coverage_pct: summary.head.as_ref().and_then(|t| t.coverage),
        patch_coverage_pct: summary.patch.as_ref().and_then(|t| t.coverage),
        patch_lines: summary.patch.as_ref().and_then(|t| t.lines),
        patch_hits: summary.patch.as_ref().and_then(|t| t.hits),
        patch_misses: summary.patch.as_ref().and_then(|t| t.misses),
    };

    let include = input.include_patch_coverage.unwrap_or(true);
    let files = raw_files
        .into_iter()
        .map(|f| {
            let patch = f.patch_totals.as_ref();
            FilePatchOutput {
                name: f.head_name,
                base_coverage_pct: f.base_coverage.as_ref().and_then(|t| t.coverage),
                head_coverage_pct: f.head_coverage.as_ref().and_then(|t| t.coverage),
                patch_coverage_pct: if include { patch.and_then(|t| t.coverage) } else { None },
                added_lines: patch.and_then(|t| t.lines),
                covered_added_lines: patch.and_then(|t| t.hits),
                uncovered_added_lines: patch.and_then(|t| t.misses),
            }
        })
        .collect();

    Ok(ChangedFilesCoverageOutput {
        pull_id,
        base_commit: summary.base_commit,
        head_commit: summary.head_commit,
        totals,
        files,
        codecov_url: client.app_pull_url(pull_id),
    })
}
