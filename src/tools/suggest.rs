use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    codecov_client::CodecovClient,
    error::AppError,
    heuristics::{
        filters::{apply_extension_filter, apply_min_uncovered_lines_filter, apply_zero_change_filter},
        ranking::{FileCandidate, rank_files},
    },
};

#[derive(Deserialize, JsonSchema)]
pub struct SuggestTestTargetsInput {
    /// The pull request number to analyze.
    pub pull_id: u64,
    /// Maximum number of files to return. Defaults to 10.
    pub max_results: Option<usize>,
    /// Exclude files with fewer uncovered added lines than this threshold.
    /// Defaults to 1 (exclude files with no uncovered lines).
    pub min_uncovered_lines: Option<i64>,
    /// Restrict output to files with these extensions, e.g. `[".rs", ".py"]`.
    /// If omitted, all file types are considered.
    pub file_extensions: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct SuggestTestTargetsOutput {
    pub pull_id: u64,
    pub ranked_files: Vec<RankedFileOutput>,
    pub ranking_method: String,
    /// Number of files that passed all filters and were scored.
    pub files_analyzed: usize,
    /// Number of files excluded by filters.
    pub files_excluded: usize,
}

#[derive(Serialize)]
pub struct RankedFileOutput {
    pub rank: usize,
    pub file_path: String,
    /// Weighted score in [0, 1] — higher means more urgent to test.
    pub score: f64,
    pub patch_coverage_pct: Option<f64>,
    pub uncovered_added_lines: i64,
    pub total_added_lines: i64,
    pub is_new_file: bool,
    pub reason: String,
}

pub async fn suggest_test_targets(
    client: &CodecovClient,
    input: SuggestTestTargetsInput,
) -> Result<SuggestTestTargetsOutput, AppError> {
    let pull_id = input.pull_id;
    let max_results = input.max_results.unwrap_or(10);
    let min_uncovered = input.min_uncovered_lines.unwrap_or(1);
    let extensions = input.file_extensions.unwrap_or_default();

    let impacted = client.poll_until_processed(pull_id).await?;
    let raw_files = impacted.files.unwrap_or_default();
    let total = raw_files.len();

    // Convert API model → scoring candidates.
    let candidates: Vec<FileCandidate> = raw_files
        .into_iter()
        .map(|f| {
            let patch_lines = f.patch_totals.as_ref().and_then(|t| t.lines).unwrap_or(0);
            let patch_misses = f.patch_totals.as_ref().and_then(|t| t.misses).unwrap_or(0);
            FileCandidate {
                file_path: f.head_name,
                patch_coverage_pct: f.patch_totals.as_ref().and_then(|t| t.coverage),
                patch_lines,
                patch_misses,
                head_coverage_pct: f.head_coverage.as_ref().and_then(|t| t.coverage),
                base_coverage_pct: f.base_coverage.as_ref().and_then(|t| t.coverage),
            }
        })
        .collect();

    // Filtering pipeline.
    let candidates = apply_zero_change_filter(candidates);
    let candidates = apply_extension_filter(candidates, &extensions);
    let candidates = apply_min_uncovered_lines_filter(candidates, min_uncovered);

    let files_analyzed = candidates.len();
    let files_excluded = total - files_analyzed;

    if files_analyzed == 0 {
        return Err(AppError::NoCoverageData(format!(
            "no files matched the filters for pull request {pull_id}"
        )));
    }

    // Score, rank, truncate.
    let ranked = rank_files(candidates);
    let ranked_files = ranked
        .into_iter()
        .take(max_results)
        .map(|r| RankedFileOutput {
            rank: r.rank,
            file_path: r.file_path,
            score: r.score,
            patch_coverage_pct: r.patch_coverage_pct,
            uncovered_added_lines: r.uncovered_added_lines,
            total_added_lines: r.total_added_lines,
            is_new_file: r.is_new_file,
            reason: r.reason,
        })
        .collect();

    Ok(SuggestTestTargetsOutput {
        pull_id,
        ranked_files,
        ranking_method: "weighted_patch_miss_rate".into(),
        files_analyzed,
        files_excluded,
    })
}
