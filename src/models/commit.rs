use serde::Deserialize;

/// Coverage totals returned by the Codecov API.
/// `coverage` can arrive as a float or a quoted string; the custom
/// deserializer below handles both forms transparently.
#[derive(Debug, Deserialize)]
pub struct Totals {
    #[serde(default, deserialize_with = "deserialize_coverage")]
    pub coverage: Option<f64>,
    pub lines: Option<i64>,
    pub hits: Option<i64>,
    pub misses: Option<i64>,
    pub partials: Option<i64>,
    pub branches: Option<i64>,
}

/// Top-level response from `GET /api/v2/{service}/{owner}/repos/{repo}/commits/{sha}`.
#[derive(Debug, Deserialize)]
pub struct CommitDetail {
    pub commitid: String,
    pub branch: Option<String>,
    pub state: Option<String>,
    pub totals: Option<Totals>,
}

/// Per-file totals inside the report endpoint response.
#[derive(Debug, Deserialize)]
pub struct FileTotals {
    #[serde(default, deserialize_with = "deserialize_coverage")]
    pub coverage: Option<f64>,
}

/// A single file entry from the report endpoint.
#[derive(Debug, Deserialize)]
pub struct ReportFile {
    pub name: String,
    pub totals: Option<FileTotals>,
}

/// Response from `GET /api/v2/{service}/{owner}/repos/{repo}/report/?sha={sha}`.
#[derive(Debug, Deserialize)]
pub struct CommitReport {
    pub files: Option<Vec<ReportFile>>,
}

/// Accepts `coverage` as either a JSON number or a quoted string.
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
