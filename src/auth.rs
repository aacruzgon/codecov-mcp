use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};

use crate::{config::Config, error::AppError};

/// Newtype around `reqwest::Client` that pre-configures the bearer token
/// as a sensitive default header so it never appears in logs.
pub struct AuthenticatedClient(reqwest::Client);

impl AuthenticatedClient {
    pub fn new(config: &Config) -> Result<Self, AppError> {
        let mut auth_value =
            HeaderValue::from_str(&format!("bearer {}", config.token)).map_err(|_| {
                AppError::Config(
                    "CODECOV_TOKEN contains characters invalid for HTTP headers".into(),
                )
            })?;
        auth_value.set_sensitive(true);

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, auth_value);

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .user_agent(concat!("codecov-mcp/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(AppError::Http)?;

        Ok(AuthenticatedClient(client))
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[tokio::test]
    async fn test_user_agent_is_set() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/")
            .match_header(
                "user-agent",
                mockito::Matcher::Regex(r"^codecov-mcp/\d+\.\d+\.\d+".to_string()),
            )
            .with_status(200)
            .create_async()
            .await;

        let config = Config {
            token: "test-token".into(),
            service: "github".into(),
            owner: "owner".into(),
            repo: "repo".into(),
            base_url: server.url(),
            max_retries: 1,
            poll_delay_ms: 0,
        };
        let auth = AuthenticatedClient::new(&config).expect("build client");
        let _ = auth.client().get(server.url()).send().await;

        mock.assert_async().await;
    }
}
