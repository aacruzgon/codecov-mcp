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
}

impl CodecovClient {
    pub fn new(config: &Config) -> Result<Self, AppError> {
        Ok(CodecovClient {
            http: AuthenticatedClient::new(config)?,
            base_url: config.base_url.clone(),
            service: config.service.clone(),
            owner: config.owner.clone(),
            repo: config.repo.clone(),
        })
    }

    /// `https://app.codecov.io/{slug}/{owner}/{repo}/commit/{sha}`
    pub fn app_commit_url(&self, sha: &str) -> String {
        let slug = match self.service.as_str() {
            "github" => "gh",
            "bitbucket" => "bb",
            "gitlab" => "gl",
            other => other,
        };
        format!(
            "https://app.codecov.io/{}/{}/{}/commit/{}",
            slug, self.owner, self.repo, sha
        )
    }
}
