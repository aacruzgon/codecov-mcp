use super::{handle_api_response, CodecovClient};
use crate::{
    error::AppError,
    models::commit::{CommitDetail, CommitReport},
};

impl CodecovClient {
    /// `GET /api/v2/{service}/{owner}/repos/{repo}/commits/{sha}`
    pub async fn get_commit_detail(&self, sha: &str) -> Result<CommitDetail, AppError> {
        let url = format!(
            "{}/api/v2/{}/{}/repos/{}/commits/{}",
            self.base_url, self.service, self.owner, self.repo, sha
        );

        let response = self
            .http
            .client()
            .get(&url)
            .send()
            .await
            .map_err(AppError::Http)?;
        let response = handle_api_response(response, format!("commit {sha} not found")).await?;
        response.json::<CommitDetail>().await.map_err(AppError::Http)
    }

    /// `GET /api/v2/{service}/{owner}/repos/{repo}/report/?sha={sha}`
    pub async fn get_commit_report(&self, sha: &str) -> Result<CommitReport, AppError> {
        let url = format!(
            "{}/api/v2/{}/{}/repos/{}/report/",
            self.base_url, self.service, self.owner, self.repo
        );

        let response = self
            .http
            .client()
            .get(&url)
            .query(&[("sha", sha)])
            .send()
            .await
            .map_err(AppError::Http)?;

        let response = handle_api_response(response, format!("report for commit {sha} not found")).await?;
        response.json::<CommitReport>().await.map_err(AppError::Http)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn make_client(base_url: &str) -> CodecovClient {
        let config = Config {
            token: "test-token".into(),
            service: "github".into(),
            owner: "test-owner".into(),
            repo: "test-repo".into(),
            base_url: base_url.to_string(),
            max_retries: 3,
            poll_delay_ms: 100,
        };
        CodecovClient::new(&config).expect("failed to build CodecovClient")
    }

    #[tokio::test]
    async fn test_get_commit_detail_success() {
        let mut server = mockito::Server::new_async().await;
        let sha = "abc123def456abc123def456abc123def456abc1";
        let mock = server
            .mock(
                "GET",
                format!("/api/v2/github/test-owner/repos/test-repo/commits/{sha}").as_str(),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(include_str!("../../tests/fixtures/commit_detail_complete.json"))
            .create_async()
            .await;

        let client = make_client(&server.url());
        let result = client.get_commit_detail(sha).await;

        assert!(result.is_ok(), "expected Ok, got {result:?}");
        let detail = result.unwrap();
        assert_eq!(detail.commitid, sha);
        assert_eq!(detail.state.as_deref(), Some("complete"));
        assert_eq!(
            detail.totals.as_ref().and_then(|t| t.coverage),
            Some(75.0)
        );

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_commit_detail_not_found() {
        let mut server = mockito::Server::new_async().await;
        let sha = "deadbeef00000000000000000000000000000000";
        let mock = server
            .mock(
                "GET",
                format!("/api/v2/github/test-owner/repos/test-repo/commits/{sha}").as_str(),
            )
            .with_status(404)
            .with_header("content-type", "application/json")
            .with_body(include_str!("../../tests/fixtures/api_error_404.json"))
            .create_async()
            .await;

        let client = make_client(&server.url());
        let result = client.get_commit_detail(sha).await;

        assert!(
            matches!(result, Err(AppError::NotFound(_))),
            "expected NotFound, got {result:?}"
        );
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_commit_detail_pending_state() {
        let mut server = mockito::Server::new_async().await;
        let sha = "abc123def456abc123def456abc123def456abc1";
        let mock = server
            .mock(
                "GET",
                format!("/api/v2/github/test-owner/repos/test-repo/commits/{sha}").as_str(),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(include_str!("../../tests/fixtures/commit_detail_pending.json"))
            .create_async()
            .await;

        let client = make_client(&server.url());
        let result = client.get_commit_detail(sha).await;

        assert!(result.is_ok(), "expected Ok, got {result:?}");
        let detail = result.unwrap();
        assert_eq!(detail.state.as_deref(), Some("pending"));
        assert!(detail.totals.is_none());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_commit_detail_server_error() {
        let mut server = mockito::Server::new_async().await;
        let sha = "abc123def456abc123def456abc123def456abc1";
        let mock = server
            .mock(
                "GET",
                format!("/api/v2/github/test-owner/repos/test-repo/commits/{sha}").as_str(),
            )
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let client = make_client(&server.url());
        let result = client.get_commit_detail(sha).await;

        assert!(
            matches!(result, Err(AppError::Api { status: 500, .. })),
            "expected Api(500), got {result:?}"
        );
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_commit_detail_unauthorized() {
        let mut server = mockito::Server::new_async().await;
        let sha = "abc123def456abc123def456abc123def456abc1";
        let mock = server
            .mock(
                "GET",
                format!("/api/v2/github/test-owner/repos/test-repo/commits/{sha}").as_str(),
            )
            .with_status(401)
            .create_async()
            .await;

        let client = make_client(&server.url());
        let result = client.get_commit_detail(sha).await;

        assert!(
            matches!(result, Err(AppError::Unauthorized)),
            "expected Unauthorized, got {result:?}"
        );
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_commit_detail_forbidden() {
        let mut server = mockito::Server::new_async().await;
        let sha = "abc123def456abc123def456abc123def456abc1";
        let mock = server
            .mock(
                "GET",
                format!("/api/v2/github/test-owner/repos/test-repo/commits/{sha}").as_str(),
            )
            .with_status(403)
            .create_async()
            .await;

        let client = make_client(&server.url());
        let result = client.get_commit_detail(sha).await;

        assert!(
            matches!(result, Err(AppError::Forbidden)),
            "expected Forbidden, got {result:?}"
        );
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_commit_detail_rate_limited() {
        let mut server = mockito::Server::new_async().await;
        let sha = "abc123def456abc123def456abc123def456abc1";
        let mock = server
            .mock(
                "GET",
                format!("/api/v2/github/test-owner/repos/test-repo/commits/{sha}").as_str(),
            )
            .with_status(429)
            .create_async()
            .await;

        let client = make_client(&server.url());
        let result = client.get_commit_detail(sha).await;

        assert!(
            matches!(result, Err(AppError::RateLimited)),
            "expected RateLimited, got {result:?}"
        );
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_commit_report_success() {
        let mut server = mockito::Server::new_async().await;
        let sha = "abc123def456abc123def456abc123def456abc1";
        let mock = server
            .mock(
                "GET",
                "/api/v2/github/test-owner/repos/test-repo/report/",
            )
            .match_query(mockito::Matcher::UrlEncoded("sha".into(), sha.into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(include_str!("../../tests/fixtures/commit_report.json"))
            .create_async()
            .await;

        let client = make_client(&server.url());
        let result = client.get_commit_report(sha).await;

        assert!(result.is_ok(), "expected Ok, got {result:?}");
        let report = result.unwrap();
        let files = report.files.unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].name, "src/main.rs");

        mock.assert_async().await;
    }
}
