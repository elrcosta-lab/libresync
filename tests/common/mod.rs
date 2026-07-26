use async_trait::async_trait;
use libresync_core::auth::error::AuthResult;
use libresync_core::auth::models::TokenSet;
use libresync_core::auth::provider::AuthProvider;
use libresync_core::auth::token_exchange::TokenResponse;
use libresync_core::sync::config::SyncConfig;
use libresync_core::sync::engine::SyncEngine;
use std::sync::Arc;

pub struct MockAuthProvider;

#[async_trait]
impl AuthProvider for MockAuthProvider {
    async fn exchange_code(
        &self,
        _client_id: &str,
        _code: &str,
        _code_verifier: &str,
        _redirect_uri: &str,
    ) -> AuthResult<TokenResponse> {
        Ok(TokenResponse {
            access_token: "mock_token".into(),
            refresh_token: Some("mock_refresh".into()),
            expires_in: 3600,
            scope: "drive.file".into(),
            token_type: "Bearer".into(),
            id_token: None,
        })
    }

    async fn refresh_token(
        &self,
        _client_id: &str,
        _refresh_token: &str,
    ) -> AuthResult<TokenResponse> {
        Ok(TokenResponse {
            access_token: "mock_refreshed_token".into(),
            refresh_token: Some("mock_refresh".into()),
            expires_in: 3600,
            scope: "drive.file".into(),
            token_type: "Bearer".into(),
            id_token: None,
        })
    }

    async fn revoke_token(&self, _token: &str) -> AuthResult<()> {
        Ok(())
    }

    async fn ensure_valid_token<'a>(
        &self,
        tokens: &'a mut TokenSet,
        _client_id: &str,
    ) -> AuthResult<&'a TokenSet> {
        Ok(tokens)
    }
}

pub fn create_test_engine() -> SyncEngine {
    let provider: Arc<dyn AuthProvider> = Arc::new(MockAuthProvider);
    let config = SyncConfig::default();
    SyncEngine::new(provider, config)
}
