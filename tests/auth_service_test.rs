use libresync_core::auth::error::{AuthError, AuthResult};
use libresync_core::auth::session::PkceSession;
use libresync_core::auth::pkce;

#[test]
fn test_auth_error_display() {
    let err = AuthError::NetworkError("connection refused".into());
    let msg = format!("{}", err);
    assert!(msg.contains("connection refused"), "deve conter a mensagem");
}

#[test]
fn test_auth_error_source() {
    let err = AuthError::TokenExpired;
    assert_eq!(format!("{}", err), "token expired");
}

#[test]
fn test_pkce_session_creation() {
    let session = PkceSession::new("my-client-id");
    assert_eq!(session.client_id, "my-client-id");
    assert_eq!(session.code_verifier.len(), 171);
    assert_eq!(session.state.len(), 43); // 32 bytes base64url
}

#[test]
fn test_pkce_session_challenge_matches_verifier() {
    let session = PkceSession::new("client-id");
    let expected = pkce::compute_code_challenge(&session.code_verifier);
    assert_eq!(session.code_challenge, expected);
}

#[test]
fn test_pkce_session_validate_state_ok() {
    let session = PkceSession::new("client-id");
    assert!(session.validate_state(&session.state).is_ok());
}

#[test]
fn test_pkce_session_validate_state_fail() {
    let session = PkceSession::new("client-id");
    let result = session.validate_state("wrong-state");
    assert!(result.is_err());
    match result {
        Err(AuthError::CsrfMismatch) => {},
        _ => panic!("esperado CsrfMismatch"),
    }
}

#[test]
fn test_pkce_session_authorization_url() {
    let session = PkceSession::new("my-client-id");
    let url = session.authorization_url("http://localhost:65432/callback");
    assert!(url.starts_with("https://accounts.google.com/o/oauth2/v2/auth?"));
    assert!(url.contains("client_id=my-client-id"));
    assert!(url.contains(&session.code_challenge));
    assert!(url.contains(&session.state));
}

#[test]
fn test_auth_error_into_result() {
    fn might_fail(should_fail: bool) -> AuthResult<()> {
        if should_fail {
            Err(AuthError::TokenExpired)
        } else {
            Ok(())
        }
    }
    assert!(might_fail(false).is_ok());
    assert!(might_fail(true).is_err());
}
