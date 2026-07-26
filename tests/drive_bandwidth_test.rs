use std::sync::Arc;
use std::time::Duration;

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use libresync_core::drive::client::DriveApiClient;

mod common;

#[tokio::test]
async fn test_bandwidth_limiter_created() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/files"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "files": []
        })))
        .mount(&mock_server)
        .await;

    let auth = Arc::new(common::MockAuthProvider);
    let client = DriveApiClient::new(auth, "test_id", "test_refresh")
        .with_bandwidth_limit(100_000)
        .with_base_urls(&mock_server.uri(), &mock_server.uri());

    let start = tokio::time::Instant::now();
    client.list_files(None).await.unwrap();
    let elapsed = start.elapsed();

    assert!(
        elapsed >= Duration::from_millis(5),
        "bandwidth limiter should delay request, elapsed: {:?}",
        elapsed
    );
}

#[tokio::test]
async fn test_bandwidth_limiter_zero() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/files"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "files": []
        })))
        .mount(&mock_server)
        .await;

    let auth = Arc::new(common::MockAuthProvider);
    let client = DriveApiClient::new(auth, "test_id", "test_refresh")
        .with_bandwidth_limit(0)
        .with_base_urls(&mock_server.uri(), &mock_server.uri());

    let start = tokio::time::Instant::now();
    client.list_files(None).await.unwrap();
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_millis(500),
        "zero bandwidth limit should not delay, elapsed: {:?}",
        elapsed
    );
}

#[tokio::test]
async fn test_bandwidth_delays_request() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/files"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "mock_id",
            "name": "test.txt",
            "mimeType": "text/plain"
        })))
        .mount(&mock_server)
        .await;

    let auth = Arc::new(common::MockAuthProvider);

    let client_no_limit = DriveApiClient::new(auth.clone(), "test_id", "test_refresh")
        .with_base_urls(&mock_server.uri(), &mock_server.uri());

    let client_limited = DriveApiClient::new(auth, "test_id", "test_refresh")
        .with_bandwidth_limit(10_000)
        .with_base_urls(&mock_server.uri(), &mock_server.uri());

    let content = vec![0u8; 5_000];

    let start = tokio::time::Instant::now();
    client_no_limit
        .upload("test.txt", &content, "text/plain", None)
        .await
        .unwrap();
    let no_limit_elapsed = start.elapsed();

    let start = tokio::time::Instant::now();
    client_limited
        .upload("test.txt", &content, "text/plain", None)
        .await
        .unwrap();
    let limited_elapsed = start.elapsed();

    assert!(
        limited_elapsed > no_limit_elapsed + Duration::from_millis(100),
        "bandwidth limited request should be slower, no_limit: {:?}, limited: {:?}",
        no_limit_elapsed,
        limited_elapsed
    );
}
