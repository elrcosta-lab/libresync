pub mod client;
pub mod error;

use async_trait::async_trait;
use client::DriveFile;
use error::DriveResult;

#[async_trait]
pub trait DriveApi: Send + Sync {
    async fn list_files(&self, parent_id: Option<&str>) -> DriveResult<Vec<DriveFile>>;
    async fn get_metadata(&self, file_id: &str) -> DriveResult<DriveFile>;
    async fn upload(
        &self,
        name: &str,
        content: &[u8],
        mime_type: &str,
        parent_id: Option<&str>,
    ) -> DriveResult<DriveFile>;
    async fn download(&self, file_id: &str) -> DriveResult<Vec<u8>>;
    async fn delete(&self, file_id: &str) -> DriveResult<()>;
}

#[async_trait]
impl DriveApi for client::DriveApiClient {
    async fn list_files(&self, parent_id: Option<&str>) -> DriveResult<Vec<DriveFile>> {
        self.list_files(parent_id).await
    }

    async fn get_metadata(&self, file_id: &str) -> DriveResult<DriveFile> {
        self.get_metadata(file_id).await
    }

    async fn upload(
        &self,
        name: &str,
        content: &[u8],
        mime_type: &str,
        parent_id: Option<&str>,
    ) -> DriveResult<DriveFile> {
        self.upload(name, content, mime_type, parent_id).await
    }

    async fn download(&self, file_id: &str) -> DriveResult<Vec<u8>> {
        self.download(file_id).await
    }

    async fn delete(&self, file_id: &str) -> DriveResult<()> {
        self.delete(file_id).await
    }
}

