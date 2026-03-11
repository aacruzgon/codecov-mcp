pub mod commits;
pub mod compare;
pub mod file_report;
pub mod pulls;

use reqwest::StatusCode;

use crate::{auth::AuthenticatedClient, config::Config, error::AppError};

pub(crate) async fn handle_api_response(
    response: reqwest::Response,
    not_found_msg: impl Into<String>,
) -> Result<reqwest::Response, AppError> {
    match response.status() {
        StatusCode::OK => Ok(response),
        StatusCode::UNAUTHORIZED => Err(AppError::Unauthorized),
        StatusCode::FORBIDDEN => Err(AppError::Forbidden),
        StatusCode::NOT_FOUND => Err(AppError::NotFound(not_found_msg.into())),
        StatusCode::TOO_MANY_REQUESTS => Err(AppError::RateLimited),
        status => {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".into());
            Err(AppError::Api {
                status: status.as_u16(),
                message,
            })
        }
    }
}

/// High-level Codecov API client.  Holds a single authenticated `reqwest::Client`
/// together with the repo coordinates from `Config`.
pub struct CodecovClient {
    pub(crate) http: AuthenticatedClient,
    pub(crate) base_url: String,
    pub(crate) service: String,
    pub(crate) owner: String,
    pub(crate) repo: String,
    pub(crate) max_retries: u32,
    pub(crate) poll_delay_ms: u64,
}

impl CodecovClient {
    pub fn new(config: &Config) -> Result<Self, AppError> {
        Ok(CodecovClient {
            http: AuthenticatedClient::new(config)?,
            base_url: config.base_url.clone(),
            service: config.service.clone(),
            owner: config.owner.clone(),
            repo: config.repo.clone(),
            max_retries: config.max_retries,
            poll_delay_ms: config.poll_delay_ms,
        })
    }

    /// `https://app.codecov.io/{slug}/{owner}/{repo}/pull/{pull_id}`
    pub fn app_pull_url(&self, pull_id: u64) -> String {
        let slug = self.service_slug();
        format!("https://app.codecov.io/{}/{}/{}/pull/{}", slug, self.owner, self.repo, pull_id)
    }

    /// `https://app.codecov.io/{slug}/{owner}/{repo}/commit/{sha}`
    pub fn app_commit_url(&self, sha: &str) -> String {
        let slug = self.service_slug();
        format!("https://app.codecov.io/{}/{}/{}/commit/{}", slug, self.owner, self.repo, sha)
    }

    fn service_slug(&self) -> &str {
        match self.service.as_str() {
            "github" => "gh",
            "bitbucket" => "bb",
            "gitlab" => "gl",
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn test_client(service: &str) -> CodecovClient {
        let config = Config {
            token: "tok".into(),
            service: service.into(),
            owner: "org".into(),
            repo: "myrepo".into(),
            base_url: "https://api.codecov.io".into(),
            max_retries: 1,
            poll_delay_ms: 0,
        };
        CodecovClient::new(&config).unwrap()
    }

    #[test]
    fn test_service_slug_github() {
        let c = test_client("github");
        assert_eq!(c.app_pull_url(1), "https://app.codecov.io/gh/org/myrepo/pull/1");
    }

    #[test]
    fn test_service_slug_bitbucket() {
        let c = test_client("bitbucket");
        assert_eq!(c.app_pull_url(5), "https://app.codecov.io/bb/org/myrepo/pull/5");
    }

    #[test]
    fn test_service_slug_gitlab() {
        let c = test_client("gitlab");
        assert_eq!(c.app_pull_url(7), "https://app.codecov.io/gl/org/myrepo/pull/7");
    }

    #[test]
    fn test_service_slug_custom() {
        let c = test_client("custom-vcs");
        assert_eq!(c.app_pull_url(2), "https://app.codecov.io/custom-vcs/org/myrepo/pull/2");
    }

    #[test]
    fn test_app_commit_url() {
        let c = test_client("github");
        assert_eq!(
            c.app_commit_url("abc123"),
            "https://app.codecov.io/gh/org/myrepo/commit/abc123"
        );
    }

    #[tokio::test]
    async fn test_handle_api_response_5xx_returns_api_error() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", "/test")
            .with_status(500)
            .with_body("internal server error")
            .create_async()
            .await;

        let resp = reqwest::get(format!("{}/test", server.url()))
            .await
            .unwrap();
        let err = handle_api_response(resp, "not found").await.unwrap_err();
        assert!(matches!(err, AppError::Api { status: 500, .. }));
    }
}
