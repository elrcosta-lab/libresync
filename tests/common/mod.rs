#[cfg(feature = "integration-test")]
pub mod drive_test_client;

use async_trait::async_trait;
use libresync_core::auth::error::AuthResult;
use libresync_core::auth::models::TokenSet;
use libresync_core::auth::provider::AuthProvider;
use libresync_core::auth::token_exchange::TokenResponse;
use libresync_core::drive::DriveApi;
use libresync_core::drive::client::DriveFile;
use libresync_core::drive::error::DriveResult;
use libresync_core::sync::config::SyncConfig;
use libresync_core::sync::engine::SyncEngine;
use std::sync::Arc;

#[allow(dead_code)]
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

pub struct MockDriveApi;

#[async_trait]
impl DriveApi for MockDriveApi {
    async fn list_files(&self, _parent_id: Option<&str>) -> DriveResult<Vec<DriveFile>> {
        Ok(vec![])
    }

    async fn get_metadata(&self, _file_id: &str) -> DriveResult<DriveFile> {
        Ok(DriveFile {
            id: "mock_file_id".into(),
            name: "mock_file.txt".into(),
            mime_type: "text/plain".into(),
            size: None,
            created_time: None,
            modified_time: None,
            md5_checksum: None,
            parents: None,
            trashed: false,
        })
    }

    async fn upload(
        &self,
        name: &str,
        _content: &[u8],
        _mime_type: &str,
        _parent_id: Option<&str>,
    ) -> DriveResult<DriveFile> {
        Ok(DriveFile {
            id: "mock_id".into(),
            name: name.into(),
            mime_type: "text/plain".into(),
            size: None,
            created_time: None,
            modified_time: None,
            md5_checksum: None,
            parents: None,
            trashed: false,
        })
    }

    async fn download(&self, _file_id: &str) -> DriveResult<Vec<u8>> {
        Ok(b"mock data".to_vec())
    }

    async fn delete(&self, _file_id: &str) -> DriveResult<()> {
        Ok(())
    }
}

pub fn create_test_engine() -> SyncEngine {
    let drive: Arc<dyn DriveApi> = Arc::new(MockDriveApi);
    let config = SyncConfig::default();
    SyncEngine::new(drive, config, "/tmp/libresync-test")
}
