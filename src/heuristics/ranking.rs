/// Intermediate representation of one changed file, ready for scoring.
/// Constructed from `ImpactedFile` by the tool handler before entering the
/// heuristics pipeline.
pub struct FileCandidate {
    pub file_path: String,
    pub patch_coverage_pct: Option<f64>,
    /// Total lines added in the patch (0 if the file was deleted-only).
    pub patch_lines: i64,
    /// Uncovered lines in the patch.
    pub patch_misses: i64,
    pub head_coverage_pct: Option<f64>,
    /// `None` when the file is newly created (no prior baseline).
    pub base_coverage_pct: Option<f64>,
}

/// Scored and ranked output for a single file.
pub struct RankedFile {
    pub rank: usize,
    pub file_path: String,
    /// Weighted score in [0, 1].
    pub score: f64,
    pub patch_coverage_pct: Option<f64>,
    pub uncovered_added_lines: i64,
    pub total_added_lines: i64,
    pub is_new_file: bool,
    pub reason: String,
}

/// Weighted score for a single file candidate.
///
/// Formula:
/// ```text
/// score = 0.50 * patch_miss_rate
///       + 0.30 * min(patch_misses, 50) / 50
///       + 0.15 * is_new_file
///       + 0.05 * max(0, 0.5 - head_coverage_pct / 100)
/// ```
pub fn score_file(c: &FileCandidate) -> f64 {
    let patch_miss_rate = c.patch_misses as f64 / c.patch_lines.max(1) as f64;
    let normalized_uncovered = (c.patch_misses as f64).min(50.0) / 50.0;
    let is_new_file = c.base_coverage_pct.is_none() && c.head_coverage_pct.is_some();
    let is_new_file_bonus = if is_new_file { 1.0 } else { 0.0 };
    let head_frac = c.head_coverage_pct.unwrap_or(0.0) / 100.0;
    let low_coverage_penalty = (0.5_f64 - head_frac).max(0.0);

    0.50 * patch_miss_rate
        + 0.30 * normalized_uncovered
        + 0.15 * is_new_file_bonus
        + 0.05 * low_coverage_penalty
}

/// Human-readable explanation of the dominant scoring factor.
pub fn reason_for(c: &FileCandidate) -> String {
    let patch_miss_rate = c.patch_misses as f64 / c.patch_lines.max(1) as f64;
    let is_new_file = c.base_coverage_pct.is_none() && c.head_coverage_pct.is_some();

    // Evaluate each component's raw contribution (before weight).
    let miss_rate_contrib = 0.50 * patch_miss_rate;
    let uncovered_contrib = 0.30 * (c.patch_misses as f64).min(50.0) / 50.0;
    let new_file_contrib = if is_new_file { 0.15 } else { 0.0 };
    let head_frac = c.head_coverage_pct.unwrap_or(0.0) / 100.0;
    let low_cov_contrib = 0.05 * (0.5_f64 - head_frac).max(0.0);

    // Pick the dominant factor.
    let max = miss_rate_contrib
        .max(uncovered_contrib)
        .max(new_file_contrib)
        .max(low_cov_contrib);

    if (max - miss_rate_contrib).abs() < f64::EPSILON || max == miss_rate_contrib {
        format!(
            "High patch miss rate: {} of {} changed lines uncovered",
            c.patch_misses, c.patch_lines
        )
    } else if max == uncovered_contrib {
        format!("{} uncovered added lines", c.patch_misses)
    } else if max == new_file_contrib {
        "New file with no prior coverage baseline".to_string()
    } else {
        format!(
            "Low overall coverage: {:.1}%",
            c.head_coverage_pct.unwrap_or(0.0)
        )
    }
}

/// Score, sort descending, and assign ranks to a list of candidates.
pub fn rank_files(candidates: Vec<FileCandidate>) -> Vec<RankedFile> {
    let mut scored: Vec<(FileCandidate, f64)> = candidates
        .into_iter()
        .map(|c| {
            let s = score_file(&c);
            (c, s)
        })
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    scored
        .into_iter()
        .enumerate()
        .map(|(i, (c, score))| {
            let is_new_file = c.base_coverage_pct.is_none() && c.head_coverage_pct.is_some();
            let reason = reason_for(&c);
            RankedFile {
                rank: i + 1,
                file_path: c.file_path,
                score,
                patch_coverage_pct: c.patch_coverage_pct,
                uncovered_added_lines: c.patch_misses,
                total_added_lines: c.patch_lines,
                is_new_file,
                reason,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(
        file_path: &str,
        patch_lines: i64,
        patch_misses: i64,
        head_pct: Option<f64>,
        base_pct: Option<f64>,
    ) -> FileCandidate {
        let patch_hits = patch_lines - patch_misses;
        let patch_coverage_pct = if patch_lines > 0 {
            Some(patch_hits as f64 / patch_lines as f64 * 100.0)
        } else {
            None
        };
        FileCandidate {
            file_path: file_path.into(),
            patch_coverage_pct,
            patch_lines,
            patch_misses,
            head_coverage_pct: head_pct,
            base_coverage_pct: base_pct,
        }
    }

    #[test]
    fn test_score_all_misses() {
        let c = candidate("src/a.rs", 10, 10, Some(50.0), Some(50.0));
        let s = score_file(&c);
        // patch_miss_rate=1.0, normalized_uncovered=10/50=0.2, no new_file, no low_cov
        let expected = 0.50 * 1.0 + 0.30 * 0.2 + 0.15 * 0.0 + 0.05 * 0.0;
        assert!((s - expected).abs() < 1e-9);
    }

    #[test]
    fn test_score_all_hits() {
        let c = candidate("src/a.rs", 10, 0, Some(80.0), Some(75.0));
        let s = score_file(&c);
        // patch_miss_rate=0, normalized_uncovered=0, not new, head>=50%
        assert!((s - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_score_new_file_bonus() {
        // base_pct = None → new file
        let c = candidate("src/new.rs", 20, 10, Some(50.0), None);
        let s = score_file(&c);
        // miss_rate=0.5, normalized=10/50=0.2, new_file=1, head>=50%
        let expected = 0.50 * 0.5 + 0.30 * 0.2 + 0.15 * 1.0 + 0.05 * 0.0;
        assert!((s - expected).abs() < 1e-9);
    }

    #[test]
    fn test_score_low_coverage_penalty() {
        // head_coverage_pct=30% → penalty = max(0, 0.5 - 0.30) = 0.20
        let c = candidate("src/a.rs", 10, 0, Some(30.0), Some(30.0));
        let s = score_file(&c);
        let expected = 0.0 + 0.0 + 0.0 + 0.05 * 0.20;
        assert!((s - expected).abs() < 1e-9);
    }

    #[test]
    fn test_score_normalized_uncovered_capped_at_50() {
        // 100 misses but cap is 50
        let c = candidate("src/a.rs", 100, 100, Some(0.0), Some(50.0));
        let s = score_file(&c);
        // normalized_uncovered = min(100,50)/50 = 1.0
        // low_cov = max(0, 0.5 - 0.0) = 0.5
        let expected = 0.50 * 1.0 + 0.30 * 1.0 + 0.15 * 0.0 + 0.05 * 0.5;
        assert!((s - expected).abs() < 1e-9);
    }

    #[test]
    fn test_rank_files_sorted_descending() {
        let high = candidate("src/high.rs", 10, 10, Some(50.0), Some(50.0));
        let low = candidate("src/low.rs", 10, 0, Some(80.0), Some(80.0));
        let ranked = rank_files(vec![low, high]);
        assert_eq!(ranked[0].file_path, "src/high.rs");
        assert_eq!(ranked[0].rank, 1);
        assert_eq!(ranked[1].file_path, "src/low.rs");
        assert_eq!(ranked[1].rank, 2);
    }

    #[test]
    fn test_rank_files_is_new_file_flag() {
        let new_file = candidate("src/new.rs", 20, 5, Some(75.0), None);
        let old_file = candidate("src/old.rs", 20, 5, Some(75.0), Some(75.0));
        let ranked = rank_files(vec![new_file, old_file]);
        let new = ranked.iter().find(|r| r.file_path == "src/new.rs").unwrap();
        let old = ranked.iter().find(|r| r.file_path == "src/old.rs").unwrap();
        assert!(new.is_new_file);
        assert!(!old.is_new_file);
    }

    #[test]
    fn test_reason_high_miss_rate() {
        let c = candidate("src/a.rs", 10, 8, Some(50.0), Some(50.0));
        let r = reason_for(&c);
        assert!(
            r.contains("8 of 10"),
            "expected miss rate reason, got: {r}"
        );
    }

    #[test]
    fn test_reason_new_file() {
        // Make new_file the dominant factor: 0 misses → no miss rate or uncovered contrib
        let c = candidate("src/new.rs", 10, 0, Some(90.0), None);
        let r = reason_for(&c);
        assert!(r.contains("New file"), "expected new file reason, got: {r}");
    }

    #[test]
    fn test_reason_low_coverage() {
        // 0 misses, existing file, low head coverage → low_cov is dominant
        let c = candidate("src/a.rs", 10, 0, Some(20.0), Some(20.0));
        let r = reason_for(&c);
        assert!(
            r.contains("Low overall coverage"),
            "expected low coverage reason, got: {r}"
        );
    }
}
