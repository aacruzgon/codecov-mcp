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
            .build()
            .map_err(AppError::Http)?;

        Ok(AuthenticatedClient(client))
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.0
    }
}
