#![cfg(feature = "integration-test")]

use libresync_core::auth::provider::{AuthProvider, GoogleAuthProvider};

#[tokio::test]
async fn test_token_refresh_works() {
    let provider = GoogleAuthProvider::new();
    let client_id =
        std::env::var("GOOGLE_CLIENT_ID").expect("GOOGLE_CLIENT_ID required");
    let refresh_token =
        std::env::var("GOOGLE_REFRESH_TOKEN").expect("GOOGLE_REFRESH_TOKEN required");

    let result = provider.refresh_token(&client_id, &refresh_token).await;
    assert!(result.is_ok(), "token refresh should succeed: {:?}", result.err());
    let tokens = result.unwrap();
    assert!(!tokens.access_token.is_empty());
    assert!(tokens.expires_in >= 3000);
    assert_eq!(tokens.token_type, "Bearer");
}

#[tokio::test]
async fn test_token_can_create_and_delete_file() {
    let provider = GoogleAuthProvider::new();
    let client_id =
        std::env::var("GOOGLE_CLIENT_ID").expect("GOOGLE_CLIENT_ID required");
    let refresh_token =
        std::env::var("GOOGLE_REFRESH_TOKEN").expect("GOOGLE_REFRESH_TOKEN required");

    let tokens = provider.refresh_token(&client_id, &refresh_token).await.unwrap();
    let client = reqwest::Client::new();
    let bearer = format!("Bearer {}", tokens.access_token);

    // Try listing files with scope info
    let list_resp = client
        .get("https://www.googleapis.com/drive/v3/files?pageSize=1&fields=*")
        .header("Authorization", &bearer)
        .send()
        .await
        .unwrap();
    let list_status = list_resp.status();
    let list_body = list_resp.text().await.unwrap_or_default();
    eprintln!(
        "LIST status={} body={}",
        list_status, list_body
    );

    // Try creating a simple file
    let metadata = serde_json::json!({
        "name": "_test_auth_token.txt",
        "mimeType": "text/plain"
    });

    let create_resp = client
        .post("https://www.googleapis.com/drive/v3/files")
        .header("Authorization", &bearer)
        .json(&metadata)
        .send()
        .await
        .unwrap();
    let create_status = create_resp.status();
    let create_body = create_resp.text().await.unwrap_or_default();
    eprintln!(
        "CREATE status={} body={}",
        create_status, create_body
    );

    assert!(create_status.is_success(), "create returned {}: {}", create_status, create_body);
}

#[tokio::test]
async fn test_invalid_token_returns_401() {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://www.googleapis.com/drive/v3/files?pageSize=1")
        .header("Authorization", "Bearer invalid_token_xyz")
        .send()
        .await
        .expect("API call should return");

    assert_eq!(resp.status().as_u16(), 401);
}

#[tokio::test]
async fn test_invalid_refresh_token_returns_error() {
    let provider = GoogleAuthProvider::new();
    let client_id =
        std::env::var("GOOGLE_CLIENT_ID").expect("GOOGLE_CLIENT_ID required");

    let result = provider
        .refresh_token(&client_id, "invalid_refresh_token_xyz")
        .await;

    assert!(result.is_err());
}
