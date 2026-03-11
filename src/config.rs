use crate::error::AppError;

#[derive(Debug)]
pub struct Config {
    pub token: String,
    pub service: String,
    pub owner: String,
    pub repo: String,
    pub base_url: String,
    pub max_retries: u32,
    pub poll_delay_ms: u64,
}

impl Config {
    pub fn from_env() -> Result<Self, AppError> {
        let token = std::env::var("CODECOV_TOKEN")
            .map_err(|_| AppError::Config("CODECOV_TOKEN is required".into()))?;
        if token.trim().is_empty() {
            return Err(AppError::Config("CODECOV_TOKEN cannot be empty".into()));
        }
        let owner = std::env::var("CODECOV_OWNER")
            .map_err(|_| AppError::Config("CODECOV_OWNER is required".into()))?;
        let repo = std::env::var("CODECOV_REPO")
            .map_err(|_| AppError::Config("CODECOV_REPO is required".into()))?;

        let service = std::env::var("CODECOV_SERVICE").unwrap_or_else(|_| "github".into());
        let base_url =
            std::env::var("CODECOV_BASE_URL").unwrap_or_else(|_| "https://api.codecov.io".into());
        let max_retries = std::env::var("CODECOV_MAX_RETRIES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);
        let poll_delay_ms = std::env::var("CODECOV_POLL_DELAY_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2000);

        Ok(Config {
            token,
            service,
            owner,
            repo,
            base_url,
            max_retries,
            poll_delay_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_missing_token_fails() {
        std::env::remove_var("CODECOV_TOKEN");
        let result = Config::from_env();
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_token_fails() {
        std::env::set_var("CODECOV_TOKEN", "");
        std::env::set_var("CODECOV_OWNER", "test-owner");
        std::env::set_var("CODECOV_REPO", "test-repo");
        let result = Config::from_env();
        assert!(
            matches!(result, Err(AppError::Config(ref msg)) if msg.contains("cannot be empty")),
            "expected Config error about empty token, got {result:?}"
        );
    }

    #[test]
    fn test_valid_config_loads() {
        std::env::set_var("CODECOV_TOKEN", "my-token");
        std::env::set_var("CODECOV_OWNER", "my-owner");
        std::env::set_var("CODECOV_REPO", "my-repo");
        std::env::set_var("CODECOV_SERVICE", "gitlab");
        let result = Config::from_env();
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        let config = result.unwrap();
        assert_eq!(config.token, "my-token");
        assert_eq!(config.owner, "my-owner");
        assert_eq!(config.repo, "my-repo");
        assert_eq!(config.service, "gitlab");
    }
}
