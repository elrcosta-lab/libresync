use libresync_core::auth::error::AuthResult;
use libresync_core::auth::token_exchange::{exchange_code, refresh_access_token, revoke_token, TokenResponse};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn test_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap()
}

#[tokio::test]
async fn test_exchange_code_success() {
    let server = MockServer::start().await;
    let client = test_client();

    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "ya29.new_token",
            "expires_in": 3600,
            "scope": "drive.file",
            "token_type": "Bearer",
            "refresh_token": "1//refresh_new",
            "id_token": null
        })))
        .mount(&server)
        .await;

    let token_url = format!("{}/token", server.uri());
    let result: AuthResult<TokenResponse> = exchange_code(
        &client,
        &token_url,
        "client_id_123",
        "auth_code_xyz",
        "verifier_abc",
        "http://localhost:9999/callback",
    )
    .await;

    assert!(result.is_ok());
    let token = result.unwrap();
    assert_eq!(token.access_token, "ya29.new_token");
    assert_eq!(token.expires_in, 3600);
    assert_eq!(token.scope, "drive.file");
    assert_eq!(token.token_type, "Bearer");
    assert_eq!(token.refresh_token, Some("1//refresh_new".to_string()));
    assert_eq!(token.id_token, None);
}

#[tokio::test]
async fn test_exchange_code_missing_refresh_token() {
    let server = MockServer::start().await;
    let client = test_client();

    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "ya29.offline",
            "expires_in": 3600,
            "scope": "drive.file",
            "token_type": "Bearer"
        })))
        .mount(&server)
        .await;

    let token_url = format!("{}/token", server.uri());
    let result: AuthResult<TokenResponse> = exchange_code(
        &client,
        &token_url,
        "client_id_123",
        "auth_code_xyz",
        "verifier_abc",
        "http://localhost:9999/callback",
    )
    .await;

    assert!(result.is_ok());
    let token = result.unwrap();
    assert_eq!(token.access_token, "ya29.offline");
    assert!(token.refresh_token.is_none());
    assert!(token.id_token.is_none());
}

#[tokio::test]
async fn test_exchange_code_with_id_token() {
    let server = MockServer::start().await;
    let client = test_client();

    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "ya29.id",
            "expires_in": 3600,
            "scope": "openid profile",
            "token_type": "Bearer",
            "refresh_token": "1//refresh",
            "id_token": "eyJ.eyJ9.sig"
        })))
        .mount(&server)
        .await;

    let token_url = format!("{}/token", server.uri());
    let result: AuthResult<TokenResponse> = exchange_code(
        &client,
        &token_url,
        "client_id",
        "code",
        "verifier",
        "http://localhost/callback",
    )
    .await;

    assert!(result.is_ok());
    let token = result.unwrap();
    assert_eq!(token.id_token, Some("eyJ.eyJ9.sig".to_string()));
}

#[tokio::test]
async fn test_refresh_access_token_success() {
    let server = MockServer::start().await;
    let client = test_client();

    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "ya29.refreshed",
            "expires_in": 3600,
            "scope": "drive.file",
            "token_type": "Bearer"
        })))
        .mount(&server)
        .await;

    let token_url = format!("{}/token", server.uri());
    let result: AuthResult<TokenResponse> =
        refresh_access_token(&client, &token_url, "client_id_123", "1//old_refresh").await;

    assert!(result.is_ok());
    let token = result.unwrap();
    assert_eq!(token.access_token, "ya29.refreshed");
}

#[tokio::test]
async fn test_revoke_token_success() {
    let server = MockServer::start().await;
    let client = test_client();

    Mock::given(method("POST"))
        .and(path("/revoke"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let revoke_url = format!("{}/revoke", server.uri());
    let result = revoke_token(&client, &revoke_url, "ya29.token_to_revoke").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_revoke_token_invalid() {
    let server = MockServer::start().await;
    let client = test_client();

    Mock::given(method("POST"))
        .and(path("/revoke"))
        .respond_with(ResponseTemplate::new(400))
        .mount(&server)
        .await;

    let revoke_url = format!("{}/revoke", server.uri());
    let result = revoke_token(&client, &revoke_url, "invalid_token").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_rate_limit_returns_error() {
    let server = MockServer::start().await;
    let client = test_client();

    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(429))
        .mount(&server)
        .await;

    let token_url = format!("{}/token", server.uri());
    let result: AuthResult<TokenResponse> = exchange_code(
        &client,
        &token_url,
        "client_id",
        "code",
        "verifier",
        "http://localhost/callback",
    )
    .await;

    assert!(result.is_err());
    match result {
        Err(libresync_core::auth::error::AuthError::RateLimited { .. }) => {}
        _ => panic!("Expected RateLimited error"),
    }
}
