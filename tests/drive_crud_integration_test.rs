#![cfg(feature = "integration-test")]

#[path = "common/drive_test_client.rs"]
mod drive_test_client;

use drive_test_client::GoogleDriveTestClient;

#[tokio::test]
async fn test_list_root_files() {
    let client = GoogleDriveTestClient::new().await.unwrap();
    let files = client.list_files(None).await.unwrap();
    assert!(!files.is_empty(), "should list at least one file/folder");
}

#[tokio::test]
async fn test_create_and_delete_folder() {
    let client = GoogleDriveTestClient::new().await.unwrap();
    let folder_name = format!("_test_libresync_folder_{}", uuid::Uuid::new_v4());

    let folder = client.create_folder(&folder_name, None).await.unwrap();
    assert_eq!(folder.name, folder_name);
    assert_eq!(folder.mime_type, "application/vnd.google-apps.folder");

    let files = client.list_files(None).await.unwrap();
    assert!(files.iter().any(|f| f.id == folder.id));

    client.delete_file(&folder.id).await.unwrap();
}

#[tokio::test]
async fn test_upload_and_download_text_file() {
    let client = GoogleDriveTestClient::new().await.unwrap();
    let content = b"Hello from LibreSync integration test!";
    let file_name = format!("_test_libresync_{}.txt", uuid::Uuid::new_v4());

    let uploaded = client
        .upload_file(&file_name, content, "text/plain", None)
        .await
        .unwrap();
    assert_eq!(uploaded.name, file_name);
    assert_eq!(uploaded.mime_type, "text/plain");

    let downloaded = client.download_file(&uploaded.id).await.unwrap();
    assert_eq!(downloaded, content);

    client.delete_file(&uploaded.id).await.unwrap();
}

#[tokio::test]
async fn test_upload_binary_file() {
    let client = GoogleDriveTestClient::new().await.unwrap();
    use rand::RngCore;
    let mut content = vec![0u8; 100];
    rand::rngs::OsRng.fill_bytes(&mut content);
    let file_name = format!("_test_libresync_bin_{}.bin", uuid::Uuid::new_v4());

    let uploaded = client
        .upload_file(&file_name, &content, "application/octet-stream", None)
        .await
        .unwrap();

    let downloaded = client.download_file(&uploaded.id).await.unwrap();
    assert_eq!(downloaded.len(), 100);
    assert_eq!(downloaded, content);

    client.delete_file(&uploaded.id).await.unwrap();
}

#[tokio::test]
async fn test_file_update() {
    let client = GoogleDriveTestClient::new().await.unwrap();
    let file_name = format!("_test_libresync_update_{}.txt", uuid::Uuid::new_v4());

    let uploaded = client
        .upload_file(&file_name, b"version 1", "text/plain", None)
        .await
        .unwrap();

    let updated = client
        .update_file(&uploaded.id, b"version 2 updated content", "text/plain")
        .await
        .unwrap();
    assert_eq!(updated.id, uploaded.id);

    let downloaded = client.download_file(&uploaded.id).await.unwrap();
    assert_eq!(String::from_utf8_lossy(&downloaded), "version 2 updated content");

    client.delete_file(&uploaded.id).await.unwrap();
}

#[tokio::test]
async fn test_get_metadata() {
    let client = GoogleDriveTestClient::new().await.unwrap();
    let file_name = format!("_test_libresync_meta_{}.txt", uuid::Uuid::new_v4());

    let uploaded = client
        .upload_file(&file_name, b"metadata test", "text/plain", None)
        .await
        .unwrap();

    let meta = client.get_metadata(&uploaded.id).await.unwrap();
    assert_eq!(meta.id, uploaded.id);
    assert_eq!(meta.name, file_name);
    assert_eq!(meta.mime_type, "text/plain");
    assert!(meta.created_time.is_some());
    assert!(meta.modified_time.is_some());

    client.delete_file(&uploaded.id).await.unwrap();
}

#[tokio::test]
async fn test_nested_folder_operations() {
    let client = GoogleDriveTestClient::new().await.unwrap();
    let parent_name = format!("_test_libresync_parent_{}", uuid::Uuid::new_v4());
    let child_name = "_test_libresync_child";

    let parent = client.create_folder(&parent_name, None).await.unwrap();
    let child = client
        .create_folder(child_name, Some(&parent.id))
        .await
        .unwrap();

    let file_name = "test_file.txt";
    let content = b"nested file content";
    let uploaded = client
        .upload_file(file_name, content, "text/plain", Some(&child.id))
        .await
        .unwrap();

    let child_files = client.list_files(Some(&child.id)).await.unwrap();
    assert!(child_files.iter().any(|f| f.id == uploaded.id));

    client.delete_file(&uploaded.id).await.unwrap();
    client.delete_file(&child.id).await.unwrap();
    client.delete_file(&parent.id).await.unwrap();
}
