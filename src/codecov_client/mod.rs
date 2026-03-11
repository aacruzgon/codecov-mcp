pub mod commits;
pub mod compare;
pub mod file_report;
pub mod pulls;

use crate::{auth::AuthenticatedClient, config::Config, error::AppError};

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
