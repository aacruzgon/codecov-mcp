use super::ranking::FileCandidate;

/// Remove files whose path doesn't end with one of the given extensions.
/// If `extensions` is empty, all files pass through.
///
/// Extensions should be provided with their dot, e.g. `".rs"`, `".py"`.
pub fn apply_extension_filter(
    candidates: Vec<FileCandidate>,
    extensions: &[String],
) -> Vec<FileCandidate> {
    if extensions.is_empty() {
        return candidates;
    }
    candidates
        .into_iter()
        .filter(|c| extensions.iter().any(|ext| c.file_path.ends_with(ext.as_str())))
        .collect()
}

/// Remove files with fewer uncovered added lines than `min`.
/// Default use: exclude files with 0 uncovered lines (`min = 1`).
pub fn apply_min_uncovered_lines_filter(
    candidates: Vec<FileCandidate>,
    min: i64,
) -> Vec<FileCandidate> {
    candidates.into_iter().filter(|c| c.patch_misses >= min).collect()
}

/// Remove files that have no changed lines at all (e.g. deleted-only files).
pub fn apply_zero_change_filter(candidates: Vec<FileCandidate>) -> Vec<FileCandidate> {
    candidates.into_iter().filter(|c| c.patch_lines > 0).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make(path: &str, patch_lines: i64, patch_misses: i64) -> FileCandidate {
        FileCandidate {
            file_path: path.into(),
            patch_coverage_pct: None,
            patch_lines,
            patch_misses,
            head_coverage_pct: Some(75.0),
            base_coverage_pct: Some(75.0),
        }
    }

    #[test]
    fn test_extension_filter_keeps_matching() {
        let files = vec![make("src/main.rs", 10, 5), make("src/lib.py", 10, 5)];
        let result = apply_extension_filter(files, &[".rs".to_string()]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].file_path, "src/main.rs");
    }

    #[test]
    fn test_extension_filter_empty_keeps_all() {
        let files = vec![make("src/main.rs", 10, 5), make("src/lib.py", 10, 5)];
        let result = apply_extension_filter(files, &[]);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_extension_filter_no_match_returns_empty() {
        let files = vec![make("src/main.rs", 10, 5)];
        let result = apply_extension_filter(files, &[".ts".to_string()]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_extension_filter_multiple_extensions() {
        let files = vec![
            make("src/main.rs", 10, 5),
            make("src/lib.py", 10, 5),
            make("src/app.ts", 10, 5),
        ];
        let result =
            apply_extension_filter(files, &[".rs".to_string(), ".py".to_string()]);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_min_uncovered_lines_filter() {
        let files = vec![
            make("src/a.rs", 10, 0),
            make("src/b.rs", 10, 3),
            make("src/c.rs", 10, 10),
        ];
        let result = apply_min_uncovered_lines_filter(files, 3);
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|f| f.patch_misses >= 3));
    }

    #[test]
    fn test_min_uncovered_lines_filter_excludes_zero() {
        let files = vec![make("src/a.rs", 10, 0), make("src/b.rs", 10, 1)];
        let result = apply_min_uncovered_lines_filter(files, 1);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].file_path, "src/b.rs");
    }

    #[test]
    fn test_zero_change_filter() {
        let files = vec![
            make("src/deleted.rs", 0, 0),
            make("src/changed.rs", 5, 2),
        ];
        let result = apply_zero_change_filter(files);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].file_path, "src/changed.rs");
    }
}
