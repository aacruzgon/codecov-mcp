use serde::Deserialize;

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_totals(json: &str) -> ComparisonTotals {
        serde_json::from_str(json).expect("failed to parse ComparisonTotals")
    }

    #[test]
    fn test_coverage_as_float() {
        let t = parse_totals(r#"{"coverage": 60.0}"#);
        assert_eq!(t.coverage, Some(60.0));
    }

    #[test]
    fn test_coverage_as_string() {
        let t = parse_totals(r#"{"coverage": "72.50"}"#);
        assert!((t.coverage.unwrap() - 72.5).abs() < 0.001);
    }

    #[test]
    fn test_coverage_null_is_none() {
        let t = parse_totals(r#"{"coverage": null}"#);
        assert_eq!(t.coverage, None);
    }

    #[test]
    fn test_coverage_missing_is_none() {
        let t = parse_totals(r#"{}"#);
        assert_eq!(t.coverage, None);
    }

    #[test]
    fn test_coverage_non_parseable_string_is_none() {
        let t = parse_totals(r#"{"coverage": "n/a"}"#);
        assert_eq!(t.coverage, None);
    }

    #[test]
    fn test_coverage_object_is_none() {
        let t = parse_totals(r#"{"coverage": {"value": 50}}"#);
        assert_eq!(t.coverage, None);
    }
}

/// Handles both JSON number (75.0) and quoted string ("75.00") forms.
fn deserialize_coverage<'de, D>(d: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let val: Option<serde_json::Value> = Option::deserialize(d)?;
    Ok(match val {
        None => None,
        Some(serde_json::Value::Number(n)) => n.as_f64(),
        Some(serde_json::Value::String(s)) => s.parse().ok(),
        _ => None,
    })
}

/// Coverage totals for a single side (base, head, or patch) of a comparison.
#[derive(Debug, Deserialize)]
pub struct ComparisonTotals {
    #[serde(default, deserialize_with = "deserialize_coverage")]
    pub coverage: Option<f64>,
    pub lines: Option<i64>,
    pub hits: Option<i64>,
    pub misses: Option<i64>,
    #[allow(dead_code)]
    pub partials: Option<i64>,
    #[allow(dead_code)]
    pub branches: Option<i64>,
}

/// Response from `GET /api/v2/{service}/{owner}/repos/{repo}/compare/?pullid={id}`.
#[derive(Debug, Deserialize)]
pub struct ComparisonSummary {
    pub base_commit: Option<String>,
    pub head_commit: Option<String>,
    pub base: Option<ComparisonTotals>,
    pub head: Option<ComparisonTotals>,
    pub patch: Option<ComparisonTotals>,
}

/// Per-file coverage entry within the impacted files response.
#[derive(Debug, Deserialize)]
pub struct ImpactedFile {
    /// Current file name (after any rename/move). Use as canonical name.
    pub head_name: String,
    /// Previous file name. `None` for newly created files.
    #[allow(dead_code)]
    pub base_name: Option<String>,
    pub base_coverage: Option<ComparisonTotals>,
    pub head_coverage: Option<ComparisonTotals>,
    /// Patch-level totals: lines added and their coverage.
    pub patch_totals: Option<ComparisonTotals>,
    /// Raw diff coverage lines — not exposed in tool output at Phase 2.
    #[allow(dead_code)]
    pub added_diff_coverage: Option<serde_json::Value>,
    #[allow(dead_code)]
    pub removed_diff_coverage: Option<serde_json::Value>,
}

/// Response from `GET /api/v2/{service}/{owner}/repos/{repo}/compare/impacted_files?pullid={id}`.
/// `state` drives the polling logic: `"pending"` → retry, `"processed"` → parse `files`.
#[derive(Debug, Deserialize)]
pub struct ImpactedFilesResponse {
    pub state: String,
    pub files: Option<Vec<ImpactedFile>>,
}
