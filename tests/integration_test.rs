use std::sync::Arc;

use codecov_mcp::{
    codecov_client::CodecovClient,
    config::Config,
    error::AppError,
    resources::pr_summary,
    tools::{
        changed_files::{get_changed_files_coverage, GetChangedFilesCoverageInput},
        commit::{get_commit_coverage, GetCommitCoverageInput},
        suggest::{suggest_test_targets, SuggestTestTargetsInput},
    },
};

// ── helpers ──────────────────────────────────────────────────────────────────

fn make_client(base_url: &str) -> Arc<CodecovClient> {
    let config = Config {
        token: "test-token".into(),
        service: "github".into(),
        owner: "test-owner".into(),
        repo: "test-repo".into(),
        base_url: base_url.to_string(),
        max_retries: 3,
        poll_delay_ms: 0,
    };
    Arc::new(CodecovClient::new(&config).expect("failed to build test client"))
}

// ── get_commit_coverage ───────────────────────────────────────────────────────

#[tokio::test]
async fn test_integration_get_commit_coverage_success() {
    let mut server = mockito::Server::new_async().await;
    let sha = "abc123def456abc123def456abc123def456abc1";

    let _mock = server
        .mock(
            "GET",
            format!("/api/v2/github/test-owner/repos/test-repo/commits/{sha}").as_str(),
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(include_str!("fixtures/commit_detail_complete.json"))
        .create_async()
        .await;

    let client = make_client(&server.url());
    let result = get_commit_coverage(
        &client,
        GetCommitCoverageInput {
            sha: sha.to_string(),
            include_files: None,
        },
    )
    .await;

    assert!(result.is_ok(), "expected Ok, got {result:?}");
    let out = result.unwrap();
    assert_eq!(out.commit_sha, sha);
    assert_eq!(out.state, "complete");
    assert_eq!(out.coverage_pct, Some(75.0));
    assert_eq!(out.lines, Some(100));
    assert_eq!(out.hits, Some(75));
    assert!(out.files.is_none(), "files should be absent when include_files is false");
    assert!(out.codecov_url.contains(sha));
}

#[tokio::test]
async fn test_integration_get_commit_coverage_with_files() {
    let mut server = mockito::Server::new_async().await;
    let sha = "abc123def456abc123def456abc123def456abc1";

    let _mock_detail = server
        .mock(
            "GET",
            format!("/api/v2/github/test-owner/repos/test-repo/commits/{sha}").as_str(),
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(include_str!("fixtures/commit_detail_complete.json"))
        .create_async()
        .await;

    let _mock_report = server
        .mock("GET", "/api/v2/github/test-owner/repos/test-repo/report/")
        .match_query(mockito::Matcher::UrlEncoded("sha".into(), sha.into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(include_str!("fixtures/commit_report.json"))
        .create_async()
        .await;

    let client = make_client(&server.url());
    let result = get_commit_coverage(
        &client,
        GetCommitCoverageInput {
            sha: sha.to_string(),
            include_files: Some(true),
        },
    )
    .await;

    assert!(result.is_ok(), "expected Ok, got {result:?}");
    let out = result.unwrap();
    let files = out.files.expect("files should be populated");
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].name, "src/main.rs");
    assert_eq!(files[0].coverage_pct, Some(80.0));
}

#[tokio::test]
async fn test_integration_get_commit_coverage_pending_returns_error() {
    let mut server = mockito::Server::new_async().await;
    let sha = "abc123def456abc123def456abc123def456abc1";

    let _mock = server
        .mock(
            "GET",
            format!("/api/v2/github/test-owner/repos/test-repo/commits/{sha}").as_str(),
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(include_str!("fixtures/commit_detail_pending.json"))
        .create_async()
        .await;

    let client = make_client(&server.url());
    let result = get_commit_coverage(
        &client,
        GetCommitCoverageInput {
            sha: sha.to_string(),
            include_files: None,
        },
    )
    .await;

    assert!(
        matches!(result, Err(AppError::CoverageNotReady { .. })),
        "expected CoverageNotReady, got {result:?}"
    );
}

#[tokio::test]
async fn test_integration_get_commit_coverage_not_found() {
    let mut server = mockito::Server::new_async().await;
    let sha = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef";

    let _mock = server
        .mock(
            "GET",
            format!("/api/v2/github/test-owner/repos/test-repo/commits/{sha}").as_str(),
        )
        .with_status(404)
        .with_body(include_str!("fixtures/api_error_404.json"))
        .create_async()
        .await;

    let client = make_client(&server.url());
    let result = get_commit_coverage(
        &client,
        GetCommitCoverageInput {
            sha: sha.to_string(),
            include_files: None,
        },
    )
    .await;

    assert!(
        matches!(result, Err(AppError::NotFound(_))),
        "expected NotFound, got {result:?}"
    );
}

#[tokio::test]
async fn test_integration_get_commit_coverage_unauthorized() {
    let mut server = mockito::Server::new_async().await;
    let sha = "abc123def456abc123def456abc123def456abc1";

    let _mock = server
        .mock(
            "GET",
            format!("/api/v2/github/test-owner/repos/test-repo/commits/{sha}").as_str(),
        )
        .with_status(401)
        .create_async()
        .await;

    let client = make_client(&server.url());
    let result = get_commit_coverage(
        &client,
        GetCommitCoverageInput {
            sha: sha.to_string(),
            include_files: None,
        },
    )
    .await;

    assert!(
        matches!(result, Err(AppError::Unauthorized)),
        "expected Unauthorized, got {result:?}"
    );
}

// ── get_changed_files_coverage ────────────────────────────────────────────────

#[tokio::test]
async fn test_integration_get_changed_files_coverage_success() {
    let mut server = mockito::Server::new_async().await;
    let pull_id = 42u64;

    let _mock_summary = server
        .mock("GET", "/api/v2/github/test-owner/repos/test-repo/compare/")
        .match_query(mockito::Matcher::UrlEncoded(
            "pullid".into(),
            pull_id.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(include_str!("fixtures/comparison_summary.json"))
        .create_async()
        .await;

    let _mock_files = server
        .mock(
            "GET",
            "/api/v2/github/test-owner/repos/test-repo/compare/impacted_files",
        )
        .match_query(mockito::Matcher::UrlEncoded(
            "pullid".into(),
            pull_id.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(include_str!("fixtures/impacted_files_processed.json"))
        .create_async()
        .await;

    let client = make_client(&server.url());
    let result = get_changed_files_coverage(
        &client,
        GetChangedFilesCoverageInput {
            pull_id,
            include_patch_coverage: None,
        },
    )
    .await;

    assert!(result.is_ok(), "expected Ok, got {result:?}");
    let out = result.unwrap();
    assert_eq!(out.pull_id, pull_id);
    assert_eq!(out.totals.base_coverage_pct, Some(72.5));
    assert_eq!(out.totals.head_coverage_pct, Some(74.0));
    assert_eq!(out.totals.patch_coverage_pct, Some(60.0));
    assert_eq!(out.totals.patch_lines, Some(10));
    assert_eq!(out.files.len(), 2);
    assert_eq!(out.files[0].name, "src/main.rs");
    assert_eq!(out.files[0].patch_coverage_pct, Some(60.0));
    assert_eq!(out.files[0].added_lines, Some(10));
    assert_eq!(out.files[0].uncovered_added_lines, Some(4));
    // New file: no base coverage
    assert_eq!(out.files[1].name, "src/new_module.rs");
    assert!(out.files[1].base_coverage_pct.is_none());
    assert!(out.codecov_url.contains("pull/42"));
}

#[tokio::test]
async fn test_integration_get_changed_files_coverage_not_found() {
    let mut server = mockito::Server::new_async().await;
    let pull_id = 999u64;

    let _mock_summary = server
        .mock("GET", "/api/v2/github/test-owner/repos/test-repo/compare/")
        .match_query(mockito::Matcher::UrlEncoded(
            "pullid".into(),
            pull_id.to_string(),
        ))
        .with_status(404)
        .with_body(include_str!("fixtures/api_error_404.json"))
        .create_async()
        .await;

    let _mock_files = server
        .mock(
            "GET",
            "/api/v2/github/test-owner/repos/test-repo/compare/impacted_files",
        )
        .match_query(mockito::Matcher::UrlEncoded(
            "pullid".into(),
            pull_id.to_string(),
        ))
        .with_status(404)
        .with_body(include_str!("fixtures/api_error_404.json"))
        .create_async()
        .await;

    let client = make_client(&server.url());
    let result = get_changed_files_coverage(
        &client,
        GetChangedFilesCoverageInput {
            pull_id,
            include_patch_coverage: None,
        },
    )
    .await;

    assert!(
        matches!(result, Err(AppError::NotFound(_))),
        "expected NotFound, got {result:?}"
    );
}

#[tokio::test]
async fn test_integration_get_changed_files_coverage_pending_exhausted() {
    let mut server = mockito::Server::new_async().await;
    let pull_id = 42u64;

    let _mock_summary = server
        .mock("GET", "/api/v2/github/test-owner/repos/test-repo/compare/")
        .match_query(mockito::Matcher::UrlEncoded(
            "pullid".into(),
            pull_id.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(include_str!("fixtures/comparison_summary.json"))
        .create_async()
        .await;

    let _mock_files = server
        .mock(
            "GET",
            "/api/v2/github/test-owner/repos/test-repo/compare/impacted_files",
        )
        .match_query(mockito::Matcher::UrlEncoded(
            "pullid".into(),
            pull_id.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(include_str!("fixtures/impacted_files_pending.json"))
        .create_async()
        .await;

    let client = make_client(&server.url());
    let result = get_changed_files_coverage(
        &client,
        GetChangedFilesCoverageInput {
            pull_id,
            include_patch_coverage: None,
        },
    )
    .await;

    assert!(
        matches!(result, Err(AppError::CoverageNotReady { .. })),
        "expected CoverageNotReady, got {result:?}"
    );
}

// ── suggest_test_targets ──────────────────────────────────────────────────────

#[tokio::test]
async fn test_integration_suggest_test_targets_success() {
    let mut server = mockito::Server::new_async().await;
    let pull_id = 42u64;

    let _mock = server
        .mock(
            "GET",
            "/api/v2/github/test-owner/repos/test-repo/compare/impacted_files",
        )
        .match_query(mockito::Matcher::UrlEncoded(
            "pullid".into(),
            pull_id.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(include_str!("fixtures/impacted_files_processed.json"))
        .create_async()
        .await;

    let client = make_client(&server.url());
    let result = suggest_test_targets(
        &client,
        SuggestTestTargetsInput {
            pull_id,
            max_results: None,
            min_uncovered_lines: None,
            file_extensions: None,
        },
    )
    .await;

    assert!(result.is_ok(), "expected Ok, got {result:?}");
    let out = result.unwrap();
    assert_eq!(out.pull_id, pull_id);
    assert_eq!(out.files_analyzed, 2);
    assert_eq!(out.files_excluded, 0);
    assert_eq!(out.ranking_method, "weighted_patch_miss_rate");
    // Both files have uncovered lines; highest score should be rank 1
    assert_eq!(out.ranked_files[0].rank, 1);
    assert!(out.ranked_files[0].score >= out.ranked_files[1].score);
    // New file (src/new_module.rs) has is_new_file = true
    let new_file = out
        .ranked_files
        .iter()
        .find(|f| f.file_path == "src/new_module.rs")
        .expect("new_module.rs should be in results");
    assert!(new_file.is_new_file);
    assert!(!out.ranked_files[0].reason.is_empty());
}

#[tokio::test]
async fn test_integration_suggest_test_targets_max_results() {
    let mut server = mockito::Server::new_async().await;
    let pull_id = 42u64;

    let _mock = server
        .mock(
            "GET",
            "/api/v2/github/test-owner/repos/test-repo/compare/impacted_files",
        )
        .match_query(mockito::Matcher::UrlEncoded(
            "pullid".into(),
            pull_id.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(include_str!("fixtures/impacted_files_processed.json"))
        .create_async()
        .await;

    let client = make_client(&server.url());
    let result = suggest_test_targets(
        &client,
        SuggestTestTargetsInput {
            pull_id,
            max_results: Some(1),
            min_uncovered_lines: None,
            file_extensions: None,
        },
    )
    .await;

    assert!(result.is_ok(), "expected Ok, got {result:?}");
    assert_eq!(result.unwrap().ranked_files.len(), 1);
}

#[tokio::test]
async fn test_integration_suggest_test_targets_extension_filter() {
    let mut server = mockito::Server::new_async().await;
    let pull_id = 42u64;

    let _mock = server
        .mock(
            "GET",
            "/api/v2/github/test-owner/repos/test-repo/compare/impacted_files",
        )
        .match_query(mockito::Matcher::UrlEncoded(
            "pullid".into(),
            pull_id.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(include_str!("fixtures/impacted_files_processed.json"))
        .create_async()
        .await;

    let client = make_client(&server.url());
    // Both fixture files are .rs — filter should keep both
    let result = suggest_test_targets(
        &client,
        SuggestTestTargetsInput {
            pull_id,
            max_results: None,
            min_uncovered_lines: None,
            file_extensions: Some(vec![".rs".to_string()]),
        },
    )
    .await;

    assert!(result.is_ok(), "expected Ok, got {result:?}");
    let out = result.unwrap();
    assert_eq!(out.files_analyzed, 2);
    assert_eq!(out.files_excluded, 0);

    // Filter for a non-existent extension → NoCoverageData
    let _mock2 = server
        .mock(
            "GET",
            "/api/v2/github/test-owner/repos/test-repo/compare/impacted_files",
        )
        .match_query(mockito::Matcher::UrlEncoded(
            "pullid".into(),
            pull_id.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(include_str!("fixtures/impacted_files_processed.json"))
        .create_async()
        .await;

    let result2 = suggest_test_targets(
        &client,
        SuggestTestTargetsInput {
            pull_id,
            max_results: None,
            min_uncovered_lines: None,
            file_extensions: Some(vec![".py".to_string()]),
        },
    )
    .await;

    assert!(
        matches!(result2, Err(AppError::NoCoverageData(_))),
        "expected NoCoverageData when no files match extension, got {result2:?}"
    );
}

#[tokio::test]
async fn test_integration_suggest_test_targets_rate_limited() {
    let mut server = mockito::Server::new_async().await;
    let pull_id = 42u64;

    let _mock = server
        .mock(
            "GET",
            "/api/v2/github/test-owner/repos/test-repo/compare/impacted_files",
        )
        .match_query(mockito::Matcher::UrlEncoded(
            "pullid".into(),
            pull_id.to_string(),
        ))
        .with_status(429)
        .create_async()
        .await;

    let client = make_client(&server.url());
    let result = suggest_test_targets(
        &client,
        SuggestTestTargetsInput {
            pull_id,
            max_results: None,
            min_uncovered_lines: None,
            file_extensions: None,
        },
    )
    .await;

    assert!(
        matches!(result, Err(AppError::RateLimited)),
        "expected RateLimited, got {result:?}"
    );
}

// ── get_changed_files_coverage (include_patch_coverage=false) ─────────────────

#[tokio::test]
async fn test_integration_get_changed_files_coverage_exclude_patch() {
    let mut server = mockito::Server::new_async().await;
    let pull_id = 42u64;

    let _mock_summary = server
        .mock("GET", "/api/v2/github/test-owner/repos/test-repo/compare/")
        .match_query(mockito::Matcher::UrlEncoded(
            "pullid".into(),
            pull_id.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(include_str!("fixtures/comparison_summary.json"))
        .create_async()
        .await;

    let _mock_files = server
        .mock(
            "GET",
            "/api/v2/github/test-owner/repos/test-repo/compare/impacted_files",
        )
        .match_query(mockito::Matcher::UrlEncoded(
            "pullid".into(),
            pull_id.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(include_str!("fixtures/impacted_files_processed.json"))
        .create_async()
        .await;

    let client = make_client(&server.url());
    let result = get_changed_files_coverage(
        &client,
        GetChangedFilesCoverageInput {
            pull_id,
            include_patch_coverage: Some(false),
        },
    )
    .await;

    assert!(result.is_ok(), "expected Ok, got {result:?}");
    let out = result.unwrap();
    // With include_patch_coverage=false, patch_coverage_pct on every file should be None
    for file in &out.files {
        assert!(
            file.patch_coverage_pct.is_none(),
            "expected patch_coverage_pct=None when include=false, got {:?}",
            file.patch_coverage_pct
        );
    }
    // added_lines / covered / uncovered are still present (come from patch totals regardless)
}

// ── pr_summary::fetch ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_integration_pr_summary_fetch_success() {
    let mut server = mockito::Server::new_async().await;
    let pull_id = 42u64;

    let _mock = server
        .mock("GET", "/api/v2/github/test-owner/repos/test-repo/compare/")
        .match_query(mockito::Matcher::UrlEncoded(
            "pullid".into(),
            pull_id.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(include_str!("fixtures/comparison_summary.json"))
        .create_async()
        .await;

    let client = make_client(&server.url());
    let result = pr_summary::fetch(&client, pull_id).await;
    assert!(result.is_ok(), "expected Ok, got {result:?}");

    let json_str = result.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("valid JSON");
    assert_eq!(parsed["pull_id"], pull_id);
    assert_eq!(parsed["owner"], "test-owner");
    assert_eq!(parsed["repo"], "test-repo");
    assert_eq!(parsed["base_coverage_pct"], 72.5);
    assert_eq!(parsed["head_coverage_pct"], 74.0);
    assert_eq!(parsed["patch_coverage_pct"], 60.0);
}

#[tokio::test]
async fn test_integration_pr_summary_fetch_not_found() {
    let mut server = mockito::Server::new_async().await;
    let pull_id = 9999u64;

    let _mock = server
        .mock("GET", "/api/v2/github/test-owner/repos/test-repo/compare/")
        .match_query(mockito::Matcher::UrlEncoded(
            "pullid".into(),
            pull_id.to_string(),
        ))
        .with_status(404)
        .create_async()
        .await;

    let client = make_client(&server.url());
    let result = pr_summary::fetch(&client, pull_id).await;
    assert!(
        matches!(result, Err(AppError::NotFound(_))),
        "expected NotFound, got {result:?}"
    );
}
