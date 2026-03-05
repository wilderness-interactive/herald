use crate::config::GoogleConfig;

const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

#[derive(Debug, serde::Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Debug)]
pub enum AuthError {
    RequestFailed(String),
    InvalidResponse(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::RequestFailed(msg) => write!(f, "Token request failed: {msg}"),
            AuthError::InvalidResponse(msg) => write!(f, "Invalid token response: {msg}"),
        }
    }
}

pub async fn fetch_access_token(
    client: &reqwest::Client,
    config: &GoogleConfig,
) -> Result<String, AuthError> {
    let response = client
        .post(TOKEN_URL)
        .form(&[
            ("client_id", config.client_id.as_str()),
            ("client_secret", config.client_secret.as_str()),
            ("refresh_token", config.refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .map_err(|e| AuthError::RequestFailed(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AuthError::RequestFailed(format!("{status}: {body}")));
    }

    let token_data: TokenResponse = response
        .json()
        .await
        .map_err(|e| AuthError::InvalidResponse(e.to_string()))?;

    Ok(token_data.access_token)
}
