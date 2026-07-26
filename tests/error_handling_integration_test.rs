#![cfg(feature = "integration-test")]

#[path = "common/drive_test_client.rs"]
mod drive_test_client;

use drive_test_client::GoogleDriveTestClient;

#[tokio::test]
async fn test_not_found_error() {
    let client = GoogleDriveTestClient::new().await.unwrap();
    let result = client.download_file("invalid_file_id_xyz_12345").await;
    assert!(result.is_err(), "non-existent file should return error");
}

#[tokio::test]
async fn test_delete_nonexistent_file() {
    let client = GoogleDriveTestClient::new().await.unwrap();
    let result = client.delete_file("nonexistent_file_id").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_empty_folder_operations() {
    let client = GoogleDriveTestClient::new().await.unwrap();
    let folder_name = format!("_test_libresync_empty_{}", uuid::Uuid::new_v4());

    let folder = client.create_folder(&folder_name, None).await.unwrap();
    let files = client.list_files(Some(&folder.id)).await.unwrap();
    assert!(files.is_empty(), "new folder should be empty");

    client.delete_file(&folder.id).await.unwrap();
}

#[tokio::test]
async fn test_upload_empty_file() {
    let client = GoogleDriveTestClient::new().await.unwrap();
    let file_name = format!("_test_libresync_empty_{}.txt", uuid::Uuid::new_v4());

    let uploaded = client
        .upload_file(&file_name, b"", "text/plain", None)
        .await
        .unwrap();

    let downloaded = client.download_file(&uploaded.id).await.unwrap();
    assert!(downloaded.is_empty(), "empty file download should be empty");

    client.delete_file(&uploaded.id).await.unwrap();
}

#[tokio::test]
async fn test_cleanup_after_self() {
    let mut client = GoogleDriveTestClient::new().await.unwrap();
    let folder_id = client.find_or_create_test_folder().await.unwrap();

    client
        .upload_file("cleanup_test.txt", b"data", "text/plain", Some(&folder_id))
        .await
        .unwrap();

    let files = client.list_files(Some(&folder_id)).await.unwrap();
    assert!(!files.is_empty());

    client.cleanup().await.unwrap();
    let files = client.list_files(None).await.unwrap();
    assert!(
        !files.iter().any(|f| f.name == ".libresync-test"),
        "test folder should be removed after cleanup"
    );
}
