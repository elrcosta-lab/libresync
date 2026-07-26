use base64::Engine;
use rand::rngs::OsRng;
use rand::RngCore;

use crate::auth::error::{AuthError, AuthResult};
use crate::auth::pkce;
use crate::auth::url;

pub enum PkceSessionState {
    WaitingCallback,
    CodeReceived(String),
    Completed,
    Failed,
}

pub struct PkceSession {
    pub client_id: String,
    pub code_verifier: String,
    pub code_challenge: String,
    pub state: String,
    pub state_enum: PkceSessionState,
    pub created_at: std::time::Instant,
}

fn generate_state() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

impl PkceSession {
    pub fn new(client_id: &str) -> Self {
        let code_verifier = pkce::generate_code_verifier();
        let code_challenge = pkce::compute_code_challenge(&code_verifier);
        let state = generate_state();

        Self {
            client_id: client_id.to_string(),
            code_verifier,
            code_challenge,
            state,
            state_enum: PkceSessionState::WaitingCallback,
            created_at: std::time::Instant::now(),
        }
    }

    pub fn validate_state(&self, received_state: &str) -> AuthResult<()> {
        if self.state == received_state {
            Ok(())
        } else {
            Err(AuthError::CsrfMismatch)
        }
    }

    pub fn authorization_url(&self, redirect_uri: &str) -> String {
        url::build_authorization_url(
            &self.client_id,
            redirect_uri,
            &self.code_challenge,
            &self.state,
        )
    }
}
