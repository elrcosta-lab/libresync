#![cfg(feature = "integration-test")]

#[path = "common/drive_test_client.rs"]
mod drive_test_client;

use std::sync::Arc;

use drive_test_client::GoogleDriveTestClient;
use libresync_core::auth::provider::GoogleAuthProvider;
use libresync_core::drive::client::DriveApiClient;
use libresync_core::drive::DriveApi;
use libresync_core::sync::config::SyncConfig;
use libresync_core::sync::engine::SyncEngine;

#[tokio::test]
async fn test_end_to_end_sync_cycle() {
    let tmp_dir = tempfile::tempdir().expect("temp dir");
    let sync_dir = tmp_dir.path().to_string_lossy().to_string();

    let auth = Arc::new(GoogleAuthProvider::new());
    let client_id =
        std::env::var("GOOGLE_CLIENT_ID").expect("GOOGLE_CLIENT_ID required");
    let refresh_token =
        std::env::var("GOOGLE_REFRESH_TOKEN").expect("GOOGLE_REFRESH_TOKEN required");
    let mut drive_test = GoogleDriveTestClient::new().await.expect("drive test client");
    let test_folder_id = drive_test.find_or_create_test_folder().await.unwrap();

    // Create a test file
    let file_name = format!("e2e_sync_test_{}.txt", uuid::Uuid::new_v4());
    let file_content = b"Hello from LibreSync E2E test!";
    let file_path = format!("{}/{}", sync_dir, file_name);
    std::fs::write(&file_path, file_content).expect("write test file");

    // Set up sync engine with the drive API
    let drive_api: Arc<dyn DriveApi> =
        Arc::new(DriveApiClient::new(auth.clone(), &client_id, &refresh_token));
    let sync_config = SyncConfig::default();
    let mut engine = SyncEngine::new(drive_api, sync_config, &sync_dir);

    // Upload the file via engine
    engine
        .on_file_changed(&file_path)
        .await
        .expect("upload should succeed");

    // Process queue
    engine.process_queue().await.expect("process queue");

    // Verify the file exists on Drive
    let remote_files = drive_test.list_files(None).await.unwrap();
    let found = remote_files.iter().any(|f| f.name == file_name);
    assert!(found, "uploaded file '{}' should appear on Drive", file_name);

    // Find the file on Drive and download it
    let remote_file = remote_files.iter().find(|f| f.name == file_name).unwrap();
    let downloaded = drive_test.download_file(&remote_file.id).await.unwrap();
    assert_eq!(
        downloaded, file_content,
        "downloaded content should match uploaded"
    );

    // Test remote change -> local download
    let remote_name = format!("e2e_remote_{}.txt", uuid::Uuid::new_v4());
    let remote_content = b"Created directly on Drive!";
    let uploaded = drive_test
        .upload_file(&remote_name, remote_content, "text/plain", Some(&test_folder_id))
        .await
        .expect("upload to drive");

    engine
        .on_remote_change(&uploaded.id)
        .await
        .expect("remote change should succeed");

    engine.process_queue().await.expect("process queue");

    let local_remote_path = format!("{}/{}", sync_dir, remote_name);
    let local_remote_content = std::fs::read(&local_remote_path).unwrap_or_default();
    assert_eq!(
        local_remote_content, remote_content,
        "remote-created file should be downloaded locally"
    );

    // Clean up: delete files from Drive
    drive_test.delete_file(&remote_file.id).await.unwrap();
    drive_test.delete_file(&uploaded.id).await.unwrap();
    std::fs::remove_file(&file_path).ok();
    std::fs::remove_file(&local_remote_path).ok();
}
