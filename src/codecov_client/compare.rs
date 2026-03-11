use super::{handle_api_response, CodecovClient};
use crate::{
    error::AppError,
    models::comparison::{ComparisonSummary, ImpactedFilesResponse},
};

impl CodecovClient {
    /// `GET /api/v2/{service}/{owner}/repos/{repo}/compare/?pullid={pull_id}`
    pub async fn get_comparison_summary(
        &self,
        pull_id: u64,
    ) -> Result<ComparisonSummary, AppError> {
        let url = format!(
            "{}/api/v2/{}/{}/repos/{}/compare/",
            self.base_url, self.service, self.owner, self.repo
        );
        let response = self
            .http
            .client()
            .get(&url)
            .query(&[("pullid", pull_id.to_string())])
            .send()
            .await
            .map_err(AppError::Http)?;
        let response =
            handle_api_response(response, format!("pull request {pull_id} not found")).await?;
        response.json::<ComparisonSummary>().await.map_err(AppError::Http)
    }

    /// `GET /api/v2/{service}/{owner}/repos/{repo}/compare/impacted_files?pullid={pull_id}`
    ///
    /// Single fetch — does NOT poll. Use [`poll_until_processed`] for polling.
    pub async fn get_impacted_files(
        &self,
        pull_id: u64,
    ) -> Result<ImpactedFilesResponse, AppError> {
        let url = format!(
            "{}/api/v2/{}/{}/repos/{}/compare/impacted_files",
            self.base_url, self.service, self.owner, self.repo
        );
        let response = self
            .http
            .client()
            .get(&url)
            .query(&[("pullid", pull_id.to_string())])
            .send()
            .await
            .map_err(AppError::Http)?;
        let response =
            handle_api_response(response, format!("pull request {pull_id} not found")).await?;
        response.json::<ImpactedFilesResponse>().await.map_err(AppError::Http)
    }

    /// Polls [`get_impacted_files`] until `state == "processed"` or retries are exhausted.
    ///
    /// Uses `self.max_retries` and `self.poll_delay_ms` from config.
    /// Returns `Err(AppError::CoverageNotReady)` if all retries are exhausted.
    pub async fn poll_until_processed(
        &self,
        pull_id: u64,
    ) -> Result<ImpactedFilesResponse, AppError> {
        let mut attempts = 0u32;
        loop {
            let result = self.get_impacted_files(pull_id).await?;
            if result.state == "processed" {
                return Ok(result);
            }
            attempts += 1;
            if attempts >= self.max_retries {
                return Err(AppError::CoverageNotReady { state: result.state });
            }
            tokio::time::sleep(std::time::Duration::from_millis(self.poll_delay_ms)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::sync::{Arc, Mutex};

    fn make_client(base_url: &str) -> CodecovClient {
        let config = Config {
            token: "test-token".into(),
            service: "github".into(),
            owner: "test-owner".into(),
            repo: "test-repo".into(),
            base_url: base_url.to_string(),
            max_retries: 3,
            poll_delay_ms: 0,
        };
        CodecovClient::new(&config).expect("failed to build CodecovClient")
    }

    #[tokio::test]
    async fn test_get_comparison_summary_success() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/api/v2/github/test-owner/repos/test-repo/compare/")
            .match_query(mockito::Matcher::UrlEncoded("pullid".into(), "42".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(include_str!("../../tests/fixtures/comparison_summary.json"))
            .create_async()
            .await;

        let client = make_client(&server.url());
        let result = client.get_comparison_summary(42).await;

        assert!(result.is_ok(), "expected Ok, got {result:?}");
        let summary = result.unwrap();
        assert_eq!(
            summary.base_commit.as_deref(),
            Some("aaaaaaaabbbbbbbbccccccccddddddddeeeeeeee")
        );
        assert_eq!(summary.base.as_ref().and_then(|t| t.coverage), Some(72.5));
        assert_eq!(summary.head.as_ref().and_then(|t| t.coverage), Some(74.0));
        assert_eq!(summary.patch.as_ref().and_then(|t| t.lines), Some(10));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_comparison_summary_not_found() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/api/v2/github/test-owner/repos/test-repo/compare/")
            .match_query(mockito::Matcher::UrlEncoded("pullid".into(), "99".into()))
            .with_status(404)
            .with_body(include_str!("../../tests/fixtures/api_error_404.json"))
            .create_async()
            .await;

        let client = make_client(&server.url());
        let result = client.get_comparison_summary(99).await;

        assert!(
            matches!(result, Err(AppError::NotFound(_))),
            "expected NotFound, got {result:?}"
        );
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_impacted_files_processed() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock(
                "GET",
                "/api/v2/github/test-owner/repos/test-repo/compare/impacted_files",
            )
            .match_query(mockito::Matcher::UrlEncoded("pullid".into(), "42".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(include_str!("../../tests/fixtures/impacted_files_processed.json"))
            .create_async()
            .await;

        let client = make_client(&server.url());
        let result = client.get_impacted_files(42).await;

        assert!(result.is_ok(), "expected Ok, got {result:?}");
        let files = result.unwrap();
        assert_eq!(files.state, "processed");
        let entries = files.files.unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].head_name, "src/main.rs");
        assert!(entries[1].base_name.is_none(), "new file should have no base_name");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_impacted_files_not_found() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock(
                "GET",
                "/api/v2/github/test-owner/repos/test-repo/compare/impacted_files",
            )
            .match_query(mockito::Matcher::UrlEncoded("pullid".into(), "99".into()))
            .with_status(404)
            .with_body(include_str!("../../tests/fixtures/api_error_404.json"))
            .create_async()
            .await;

        let client = make_client(&server.url());
        let result = client.get_impacted_files(99).await;

        assert!(
            matches!(result, Err(AppError::NotFound(_))),
            "expected NotFound, got {result:?}"
        );
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_poll_until_processed_retries_then_succeeds() {
        let mut server = mockito::Server::new_async().await;
        let call_count = Arc::new(Mutex::new(0usize));
        let call_count_clone = Arc::clone(&call_count);

        let mock = server
            .mock(
                "GET",
                "/api/v2/github/test-owner/repos/test-repo/compare/impacted_files",
            )
            .match_query(mockito::Matcher::UrlEncoded("pullid".into(), "42".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_chunked_body(move |w| {
                let mut count = call_count_clone.lock().unwrap();
                *count += 1;
                if *count < 3 {
                    std::io::Write::write_all(
                        w,
                        include_bytes!("../../tests/fixtures/impacted_files_pending.json"),
                    )
                } else {
                    std::io::Write::write_all(
                        w,
                        include_bytes!("../../tests/fixtures/impacted_files_processed.json"),
                    )
                }
            })
            .expect(3)
            .create_async()
            .await;

        let client = make_client(&server.url());
        let result = client.poll_until_processed(42).await;

        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(result.unwrap().state, "processed");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_poll_until_processed_exhaustion() {
        let mut server = mockito::Server::new_async().await;
        // Always returns pending; max_retries=3, so 3 calls before giving up.
        let mock = server
            .mock(
                "GET",
                "/api/v2/github/test-owner/repos/test-repo/compare/impacted_files",
            )
            .match_query(mockito::Matcher::UrlEncoded("pullid".into(), "42".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(include_str!("../../tests/fixtures/impacted_files_pending.json"))
            .expect(3)
            .create_async()
            .await;

        let client = make_client(&server.url());
        let result = client.poll_until_processed(42).await;

        assert!(
            matches!(result, Err(AppError::CoverageNotReady { .. })),
            "expected CoverageNotReady, got {result:?}"
        );
        mock.assert_async().await;
    }
}
