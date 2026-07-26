use std::process::Command;
use std::time::Duration;

use crate::auth::device_flow::DeviceFlow;
use crate::auth::error::{AuthError, AuthResult};
use crate::auth::provider::AuthProvider;
use crate::auth::server::CallbackServer;
use crate::auth::session::PkceSession;
use crate::auth::token_exchange::TokenResponse;
use crate::keyring::storage::TokenStorage;

const REDIRECT_URI: &str = "http://localhost:65432/callback";

pub struct LoginFlow {
    client_id: String,
}

impl LoginFlow {
    pub fn new(client_id: &str) -> Self {
        Self {
            client_id: client_id.to_string(),
        }
    }

    pub async fn login(&self, provider: &dyn AuthProvider) -> AuthResult<TokenResponse> {
        let session = PkceSession::new(&self.client_id);
        let auth_url = session.authorization_url(REDIRECT_URI);

        let server = CallbackServer::new().with_timeout(Duration::from_secs(300));

        let auth_fut = server.wait_for_callback(&session.state);
        let open_fut = async {
            if cfg!(not(test)) {
                let has_display = std::env::var("DISPLAY").is_ok()
                    || std::env::var("WAYLAND_DISPLAY").is_ok();
                if has_display {
                    let _ = Command::new("xdg-open").arg(&auth_url).spawn();
                } else {
                    eprintln!("Open this URL manually:\n  {}", auth_url);
                }
            }
        };

        let _ = tokio::join!(open_fut);

        let auth_code = auth_fut.await?;

        let token_response = provider
            .exchange_code(
                &self.client_id,
                &auth_code.code,
                &session.code_verifier,
                REDIRECT_URI,
            )
            .await?;

        let storage = TokenStorage::new().await;
        let token_json = serde_json::to_string(&token_response)
            .map_err(|e| AuthError::StateError(e.to_string()))?;
        let _ = storage.store("default", &token_json).await;

        Ok(token_response)
    }

    pub async fn login_device(&self, _provider: &dyn AuthProvider) -> AuthResult<TokenResponse> {
        let device_flow = DeviceFlow::new(&self.client_id);
        let device_code = device_flow.request_device_code().await?;

        println!();
        println!("== Device Authorization ==");
        println!();
        println!("Visit this URL in any browser:");
        println!("  {}", device_code.verification_url);
        println!();
        println!("Enter the code:");
        println!("  {}", device_code.user_code);
        println!();
        println!("This code expires in {} seconds.", device_code.expires_in);

        let token_response = device_flow.poll_for_token(&device_code).await?;

        let storage = TokenStorage::new().await;
        let token_json = serde_json::to_string(&token_response)
            .map_err(|e| AuthError::StateError(e.to_string()))?;
        let _ = storage.store("default", &token_json).await;

        Ok(token_response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::error::AuthError;
    use crate::auth::token_exchange::TokenResponse;
    use async_trait::async_trait;

    struct MockProvider;

    #[async_trait]
    impl AuthProvider for MockProvider {
        async fn exchange_code(
            &self,
            _client_id: &str,
            _code: &str,
            _code_verifier: &str,
            _redirect_uri: &str,
        ) -> AuthResult<TokenResponse> {
            Ok(TokenResponse {
                access_token: "mock_access".into(),
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
                access_token: "refreshed".into(),
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
            tokens: &'a mut crate::auth::models::TokenSet,
            _client_id: &str,
        ) -> AuthResult<&'a crate::auth::models::TokenSet> {
            Ok(tokens)
        }
    }

    #[test]
    fn test_login_flow_creation() {
        let flow = LoginFlow::new("test-client-id");
        assert_eq!(flow.client_id, "test-client-id");
    }

    #[tokio::test]
    async fn test_login_flow_integration() {
        let flow = LoginFlow::new("test-client-id");
        let session = crate::auth::session::PkceSession::new("test-client-id");
        let _url = session.authorization_url("http://localhost:0/callback");
        assert_eq!(flow.client_id, "test-client-id");
    }
}
