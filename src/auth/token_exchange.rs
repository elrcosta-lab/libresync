use serde::Deserialize;

use crate::auth::error::{AuthError, AuthResult};

#[derive(Debug, Clone, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    pub expires_in: i64,
    #[serde(default)]
    pub scope: String,
    pub token_type: String,
    #[serde(default)]
    pub id_token: Option<String>,
}

pub async fn exchange_code(
    client: &reqwest::Client,
    token_url: &str,
    client_id: &str,
    code: &str,
    code_verifier: &str,
    redirect_uri: &str,
) -> AuthResult<TokenResponse> {
    let params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("code_verifier", code_verifier),
        ("redirect_uri", redirect_uri),
        ("client_id", client_id),
    ];

    let resp = client
        .post(token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| AuthError::NetworkError(e.to_string()))?;

    handle_response(resp).await
}

pub async fn refresh_access_token(
    client: &reqwest::Client,
    token_url: &str,
    client_id: &str,
    refresh_token: &str,
) -> AuthResult<TokenResponse> {
    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", client_id),
    ];

    let resp = client
        .post(token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| AuthError::NetworkError(e.to_string()))?;

    handle_response(resp).await
}

pub async fn revoke_token(
    client: &reqwest::Client,
    revoke_url: &str,
    token: &str,
) -> AuthResult<()> {
    let params = [("token", token)];

    let resp = client
        .post(revoke_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| AuthError::NetworkError(e.to_string()))?;

    let status = resp.status();
    if status.is_success() {
        Ok(())
    } else if status.as_u16() == 429 {
        Err(AuthError::RateLimited { retry_after: None })
    } else {
        Err(AuthError::NetworkError(format!(
            "revoke failed with status {}",
            status
        )))
    }
}

async fn handle_response<T: for<'de> Deserialize<'de>>(
    resp: reqwest::Response,
) -> AuthResult<T> {
    let status = resp.status();
    if status.is_success() {
        resp.json::<T>()
            .await
            .map_err(|e| AuthError::NetworkError(e.to_string()))
    } else if status.as_u16() == 429 {
        Err(AuthError::RateLimited { retry_after: None })
    } else if status.as_u16() == 400 {
        let body = resp.text().await.unwrap_or_default();
        Err(AuthError::StateError(format!("bad request: {}", body)))
    } else {
        Err(AuthError::NetworkError(format!("HTTP {}", status)))
    }
}
