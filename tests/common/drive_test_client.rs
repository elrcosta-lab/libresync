#![allow(dead_code)]

use std::env;


const DRIVE_API_BASE: &str = "https://www.googleapis.com/drive/v3";
const UPLOAD_API_BASE: &str = "https://www.googleapis.com/upload/drive/v3";
const OAUTH_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

#[derive(Debug, serde::Deserialize)]
pub struct DriveFile {
    pub id: String,
    pub name: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub size: Option<String>,
    #[serde(rename = "createdTime")]
    pub created_time: Option<String>,
    #[serde(rename = "modifiedTime")]
    pub modified_time: Option<String>,
    #[serde(rename = "md5Checksum")]
    pub md5_checksum: Option<String>,
    pub parents: Option<Vec<String>>,
    #[serde(default)]
    pub trashed: bool,
}

#[derive(serde::Deserialize)]
struct FileListResponse {
    files: Vec<DriveFile>,
    next_page_token: Option<String>,
}

#[derive(serde::Serialize)]
struct CreateFileRequest {
    name: String,
    #[serde(rename = "mimeType")]
    mime_type: String,
    parents: Option<Vec<String>>,
}

#[derive(serde::Serialize)]
struct UpdateFileRequest {
    name: Option<String>,
    #[serde(rename = "mimeType")]
    mime_type: Option<String>,
}

#[derive(serde::Serialize)]
struct QueryParams {
    q: Option<String>,
    page_size: Option<u32>,
    fields: Option<String>,
    spaces: Option<String>,
}

#[derive(Debug)]
pub enum DriveTestError {
    MissingEnvVar(String),
    TokenRefreshFailed(String),
    ApiError { status: u16, body: String },
    NetworkError(String),
    CleanupError(String),
}

impl std::fmt::Display for DriveTestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingEnvVar(v) => write!(f, "missing env var: {}", v),
            Self::TokenRefreshFailed(msg) => write!(f, "token refresh failed: {}", msg),
            Self::ApiError { status, body } => write!(f, "API error {}: {}", status, body),
            Self::NetworkError(msg) => write!(f, "network error: {}", msg),
            Self::CleanupError(msg) => write!(f, "cleanup error: {}", msg),
        }
    }
}

#[derive(serde::Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: i64,
}

pub struct GoogleDriveTestClient {
    client: reqwest::Client,
    access_token: String,
    pub client_id: String,
    pub refresh_token: String,
    test_folder_id: Option<String>,
}

impl GoogleDriveTestClient {
    async fn json_or_error<T: serde::de::DeserializeOwned>(
        &self,
        resp: reqwest::Response,
    ) -> Result<T, DriveTestError> {
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DriveTestError::ApiError {
                status: status.as_u16(),
                body,
            });
        }
        let body = resp.text().await.map_err(|e| DriveTestError::NetworkError(e.to_string()))?;
        serde_json::from_str(&body).map_err(|e| {
            DriveTestError::NetworkError(format!(
                "json decode: {} body={}",
                e, body
            ))
        })
    }
    pub async fn new() -> Result<Self, DriveTestError> {
        let client_id = env::var("GOOGLE_CLIENT_ID").map_err(|_| {
            DriveTestError::MissingEnvVar("GOOGLE_CLIENT_ID".into())
        })?;
        let refresh_token = env::var("GOOGLE_REFRESH_TOKEN").map_err(|_| {
            DriveTestError::MissingEnvVar("GOOGLE_REFRESH_TOKEN".into())
        })?;
        let client_secret = env::var("GOOGLE_CLIENT_SECRET").ok();

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| DriveTestError::NetworkError(e.to_string()))?;

        let access_token = Self::do_refresh(&client, &client_id, &refresh_token, client_secret.as_deref()).await?;

        Ok(Self {
            client,
            access_token,
            client_id,
            refresh_token,
            test_folder_id: None,
        })
    }

    async fn do_refresh(
        client: &reqwest::Client,
        client_id: &str,
        refresh_token: &str,
        client_secret: Option<&str>,
    ) -> Result<String, DriveTestError> {
        let mut params = vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", client_id),
        ];
        if let Some(secret) = client_secret {
            params.push(("client_secret", secret));
        }

        let resp = client
            .post(OAUTH_TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| DriveTestError::TokenRefreshFailed(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DriveTestError::TokenRefreshFailed(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        let token: TokenResponse = resp
            .json()
            .await
            .map_err(|e| DriveTestError::TokenRefreshFailed(e.to_string()))?;

        Ok(token.access_token)
    }

    pub async fn ensure_token(&mut self) -> Result<(), DriveTestError> {
        let client_secret = env::var("GOOGLE_CLIENT_SECRET").ok();
        self.access_token = Self::do_refresh(
            &self.client,
            &self.client_id,
            &self.refresh_token,
            client_secret.as_deref(),
        )
        .await?;
        Ok(())
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.access_token)
    }

    pub async fn list_files(
        &self,
        parent_id: Option<&str>,
    ) -> Result<Vec<DriveFile>, DriveTestError> {
        let mut q = String::from("trashed=false");
        if let Some(pid) = parent_id {
            q = format!("'{}' in parents and trashed=false", pid);
        }

        let resp = self
            .client
            .get(format!("{}/files", DRIVE_API_BASE))
            .header("Authorization", self.auth_header())
            .query(&[
                ("q", q.as_str()),
                ("pageSize", "100"),
                ("fields", "files(id,name,mimeType,size,createdTime,modifiedTime,md5Checksum,parents,trashed)"),
            ])
            .send()
            .await
            .map_err(|e| DriveTestError::NetworkError(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DriveTestError::ApiError {
                status: status.as_u16(),
                body,
            });
        }

        let list: FileListResponse = resp
            .json()
            .await
            .map_err(|e| DriveTestError::NetworkError(e.to_string()))?;
        Ok(list.files)
    }

    pub async fn create_folder(
        &self,
        name: &str,
        parent_id: Option<&str>,
    ) -> Result<DriveFile, DriveTestError> {
        let body = CreateFileRequest {
            name: name.into(),
            mime_type: "application/vnd.google-apps.folder".into(),
            parents: parent_id.map(|p| vec![p.into()]),
        };

        let resp = self
            .client
            .post(format!("{}/files?fields=*", DRIVE_API_BASE))
            .header("Authorization", self.auth_header())
            .json(&body)
            .send()
            .await
            .map_err(|e| DriveTestError::NetworkError(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DriveTestError::ApiError {
                status: status.as_u16(),
                body,
            });
        }

        self.json_or_error(resp).await
    }

    pub async fn upload_file(
        &self,
        name: &str,
        content: &[u8],
        mime_type: &str,
        parent_id: Option<&str>,
    ) -> Result<DriveFile, DriveTestError> {
        let mut url = format!("{}/files?uploadType=multipart&fields=*", UPLOAD_API_BASE);
        if let Some(pid) = parent_id {
            url = format!("{}/files?uploadType=multipart&parents={}&fields=*", UPLOAD_API_BASE, pid);
        }

        let boundary = format!("boundary_{}", uuid::Uuid::new_v4());
        let body = build_multipart_body(name, content, mime_type, parent_id, &boundary);

        let resp = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header(
                "Content-Type",
                format!("multipart/related; boundary={}", boundary),
            )
            .body(body)
            .send()
            .await
            .map_err(|e| DriveTestError::NetworkError(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DriveTestError::ApiError {
                status: status.as_u16(),
                body,
            });
        }

        self.json_or_error(resp).await
    }

    pub async fn update_file(
        &self,
        file_id: &str,
        content: &[u8],
        mime_type: &str,
    ) -> Result<DriveFile, DriveTestError> {
        let url = format!(
            "{}/files/{}?uploadType=multipart&fields=*",
            UPLOAD_API_BASE, file_id
        );

        let boundary = format!("boundary_{}", uuid::Uuid::new_v4());
        let metadata_json = serde_json::to_string(&UpdateFileRequest {
            name: None,
            mime_type: Some(mime_type.into()),
        })
        .unwrap();

        let mut body = Vec::new();
        body.extend_from_slice(b"--");
        body.extend_from_slice(boundary.as_bytes());
        body.extend_from_slice(b"\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n");
        body.extend_from_slice(metadata_json.as_bytes());
        body.extend_from_slice(b"\r\n--");
        body.extend_from_slice(boundary.as_bytes());
        body.extend_from_slice(b"\r\nContent-Type: ");
        body.extend_from_slice(mime_type.as_bytes());
        body.extend_from_slice(b"\r\n\r\n");
        body.extend_from_slice(content);
        body.extend_from_slice(b"\r\n--");
        body.extend_from_slice(boundary.as_bytes());
        body.extend_from_slice(b"--\r\n");

        let resp = self
            .client
            .patch(&url)
            .header("Authorization", self.auth_header())
            .header(
                "Content-Type",
                format!("multipart/related; boundary={}", boundary),
            )
            .body(body)
            .send()
            .await
            .map_err(|e| DriveTestError::NetworkError(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DriveTestError::ApiError {
                status: status.as_u16(),
                body,
            });
        }

        self.json_or_error(resp).await
    }

    pub async fn download_file(&self, file_id: &str) -> Result<Vec<u8>, DriveTestError> {
        let resp = self
            .client
            .get(format!("{}/files/{}?alt=media", DRIVE_API_BASE, file_id))
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| DriveTestError::NetworkError(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DriveTestError::ApiError {
                status: status.as_u16(),
                body,
            });
        }

        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| DriveTestError::NetworkError(e.to_string()))
    }

    pub async fn get_metadata(&self, file_id: &str) -> Result<DriveFile, DriveTestError> {
        let resp = self
            .client
            .get(format!("{}/files/{}?fields=*", DRIVE_API_BASE, file_id))
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| DriveTestError::NetworkError(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DriveTestError::ApiError {
                status: status.as_u16(),
                body,
            });
        }

        self.json_or_error(resp).await
    }

    pub async fn delete_file(&self, file_id: &str) -> Result<(), DriveTestError> {
        let resp = self
            .client
            .delete(format!("{}/files/{}", DRIVE_API_BASE, file_id))
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| DriveTestError::NetworkError(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DriveTestError::ApiError {
                status: status.as_u16(),
                body,
            });
        }
        Ok(())
    }

    pub async fn find_or_create_test_folder(&mut self) -> Result<String, DriveTestError> {
        if let Some(ref id) = self.test_folder_id {
            return Ok(id.clone());
        }

        let test_folder_name = ".libresync-test";
        let files = self.list_files(None).await?;

        if let Some(existing) = files
            .iter()
            .find(|f| f.name == test_folder_name && f.mime_type == "application/vnd.google-apps.folder")
        {
            self.test_folder_id = Some(existing.id.clone());
            return Ok(existing.id.clone());
        }

        let folder = self.create_folder(test_folder_name, None).await?;
        self.test_folder_id = Some(folder.id.clone());
        Ok(folder.id)
    }

    pub async fn cleanup(&mut self) -> Result<(), DriveTestError> {
        if let Some(ref id) = self.test_folder_id.take() {
            let files = self.list_files(Some(id)).await.unwrap_or_default();
            for file in files {
                self.delete_file(&file.id).await.ok();
            }
            self.delete_file(id).await.ok();
        }
        Ok(())
    }
}

fn build_multipart_body(
    name: &str,
    content: &[u8],
    mime_type: &str,
    parent_id: Option<&str>,
    boundary: &str,
) -> Vec<u8> {
    let metadata = serde_json::json!({
        "name": name,
        "mimeType": mime_type,
        "parents": parent_id.map(|p| vec![p]),
    });
    let metadata_str = serde_json::to_string(&metadata).unwrap();

    let mut body = Vec::new();
    body.extend_from_slice(b"--");
    body.extend_from_slice(boundary.as_bytes());
    body.extend_from_slice(b"\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n");
    body.extend_from_slice(metadata_str.as_bytes());
    body.extend_from_slice(b"\r\n--");
    body.extend_from_slice(boundary.as_bytes());
    body.extend_from_slice(b"\r\nContent-Type: ");
    body.extend_from_slice(mime_type.as_bytes());
    body.extend_from_slice(b"\r\n\r\n");
    body.extend_from_slice(content);
    body.extend_from_slice(b"\r\n--");
    body.extend_from_slice(boundary.as_bytes());
    body.extend_from_slice(b"--\r\n");
    body
}
