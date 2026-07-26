use async_trait::async_trait;

use crate::auth::error::{AuthError, AuthResult};
use crate::auth::models::TokenSet;
use crate::auth::token_exchange::{self, TokenResponse};

const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_REVOKE_URL: &str = "https://oauth2.googleapis.com/revoke";

#[async_trait]
pub trait AuthProvider {
    async fn exchange_code(
        &self,
        client_id: &str,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> AuthResult<TokenResponse>;

    async fn refresh_token(
        &self,
        client_id: &str,
        refresh_token: &str,
    ) -> AuthResult<TokenResponse>;

    async fn revoke_token(&self, token: &str) -> AuthResult<()>;

    async fn ensure_valid_token<'a>(
        &self,
        tokens: &'a mut TokenSet,
        client_id: &str,
    ) -> AuthResult<&'a TokenSet>;
}

pub struct GoogleAuthProvider {
    client: reqwest::Client,
    client_secret: Option<String>,
}

impl GoogleAuthProvider {
    pub fn new() -> Self {
        let client_secret = std::env::var("GOOGLE_CLIENT_SECRET").ok();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build reqwest Client");
        Self {
            client,
            client_secret,
        }
    }

    pub fn with_client(client: reqwest::Client) -> Self {
        let client_secret = std::env::var("GOOGLE_CLIENT_SECRET").ok();
        Self {
            client,
            client_secret,
        }
    }

    pub fn with_client_secret(client: reqwest::Client, client_secret: &str) -> Self {
        Self {
            client,
            client_secret: Some(client_secret.to_string()),
        }
    }
}

impl Default for GoogleAuthProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AuthProvider for GoogleAuthProvider {
    async fn exchange_code(
        &self,
        client_id: &str,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> AuthResult<TokenResponse> {
        token_exchange::exchange_code(
            &self.client,
            GOOGLE_TOKEN_URL,
            client_id,
            code,
            code_verifier,
            redirect_uri,
            self.client_secret.as_deref(),
        )
        .await
    }

    async fn refresh_token(
        &self,
        client_id: &str,
        refresh_token: &str,
    ) -> AuthResult<TokenResponse> {
        token_exchange::refresh_access_token(
            &self.client,
            GOOGLE_TOKEN_URL,
            client_id,
            refresh_token,
            self.client_secret.as_deref(),
        )
        .await
    }

    async fn revoke_token(&self, token: &str) -> AuthResult<()> {
        token_exchange::revoke_token(&self.client, GOOGLE_REVOKE_URL, token).await
    }

    async fn ensure_valid_token<'a>(
        &self,
        tokens: &'a mut TokenSet,
        client_id: &str,
    ) -> AuthResult<&'a TokenSet> {
        if tokens.should_refresh(300) {
            let refresh_token = tokens
                .refresh_token
                .as_deref()
                .ok_or(AuthError::TokenExpired)?;
            let response = self.refresh_token(client_id, refresh_token).await?;
            tokens.access_token = response.access_token;
            tokens.scope = response.scope;
            if let Some(id_token) = response.id_token {
                tokens.id_token = Some(id_token);
            }
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            tokens.expires_at = now + response.expires_in;
        }
        Ok(tokens)
    }
}
