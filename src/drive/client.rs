use std::sync::Arc;
use std::time::Instant;

use tokio::sync::Mutex;

use crate::auth::provider::AuthProvider;
use crate::drive::error::{DriveError, DriveResult};
use crate::transfer::token_bucket::TokenBucket;

const DRIVE_API_BASE: &str = "https://www.googleapis.com/drive/v3";
const UPLOAD_API_BASE: &str = "https://www.googleapis.com/upload/drive/v3";

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveFile {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub size: Option<String>,
    pub created_time: Option<String>,
    pub modified_time: Option<String>,
    pub md5_checksum: Option<String>,
    pub parents: Option<Vec<String>>,
    #[serde(default)]
    pub trashed: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct FileList {
    files: Vec<DriveFile>,
    next_page_token: Option<String>,
}

struct TokenCache {
    access_token: String,
    expires_at: Instant,
}

pub struct DriveApiClient {
    client: reqwest::Client,
    auth: Arc<dyn AuthProvider>,
    client_id: String,
    refresh_token: String,
    cache: Mutex<Option<TokenCache>>,
    drive_api_base: String,
    upload_api_base: String,
    bandwidth_limiter: std::sync::Mutex<Option<Arc<TokenBucket>>>,
}

impl DriveApiClient {
    pub fn new(
        auth: Arc<dyn AuthProvider>,
        client_id: &str,
        refresh_token: &str,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("reqwest Client");
        
        println!("[DriveApiClient] Conectado (client_id: {}...)", 
            if client_id.len() > 10 { &client_id[..10] } else { client_id }
        );
        
        Self {
            client,
            auth,
            client_id: client_id.to_string(),
            refresh_token: refresh_token.to_string(),
            cache: Mutex::new(None),
            drive_api_base: DRIVE_API_BASE.to_string(),
            upload_api_base: UPLOAD_API_BASE.to_string(),
            bandwidth_limiter: std::sync::Mutex::new(None),
        }
    }

    pub fn new_from_env(auth: Arc<dyn AuthProvider>) -> DriveResult<Self> {
        let client_id =
            std::env::var("GOOGLE_CLIENT_ID").map_err(|_| DriveError::Config("GOOGLE_CLIENT_ID not set".into()))?;
        let refresh_token =
            std::env::var("GOOGLE_REFRESH_TOKEN").map_err(|_| DriveError::Config("GOOGLE_REFRESH_TOKEN not set".into()))?;
        let mut client = Self::new(auth, &client_id, &refresh_token);
        if let Ok(kbps_str) = std::env::var("LIBRESYNC_BANDWIDTH_KBPS") {
            if let Ok(kbps) = kbps_str.parse::<u64>() {
                client = client.with_bandwidth_limit(kbps);
            }
        }
        Ok(client)
    }

    pub fn with_bandwidth_limit(mut self, kbps: u64) -> Self {
        if kbps > 0 {
            self.bandwidth_limiter = std::sync::Mutex::new(Some(Arc::new(TokenBucket::new(kbps))));
        }
        self
    }

    pub fn with_base_urls(mut self, drive_api_base: &str, upload_api_base: &str) -> Self {
        self.drive_api_base = drive_api_base.to_string();
        self.upload_api_base = upload_api_base.to_string();
        self
    }

    pub fn set_bandwidth(&self, kbps: u64) {
        let mut guard = self.bandwidth_limiter.lock().unwrap();
        if kbps > 0 {
            *guard = Some(Arc::new(TokenBucket::new(kbps)));
        } else {
            *guard = None;
        }
    }

    async fn apply_bandwidth_limit(&self, tokens: u64) {
        let bucket = self.bandwidth_limiter.lock().unwrap().clone();
        if let Some(bucket) = bucket {
            bucket.consume(tokens).await;
        }
    }

    async fn ensure_token(&self) -> DriveResult<String> {
        let mut cache = self.cache.lock().await;
        if let Some(ref cached) = *cache {
            if Instant::now() < cached.expires_at {
                return Ok(cached.access_token.clone());
            }
        }

        let resp = self
            .auth
            .refresh_token(&self.client_id, &self.refresh_token)
            .await
            .map_err(|e| DriveError::Auth(e.to_string()))?;

        let token = format!("Bearer {}", resp.access_token);
        *cache = Some(TokenCache {
            access_token: token.clone(),
            expires_at: Instant::now() + std::time::Duration::from_secs(resp.expires_in as u64 - 60),
        });
        Ok(token)
    }

    fn check_status(&self, status: reqwest::StatusCode, body: &str) -> DriveResult<()> {
        match status.as_u16() {
            200..=299 => Ok(()),
            401 | 403 => Err(DriveError::Auth(format!("HTTP {}: {}", status, body))),
            404 => Err(DriveError::NotFound(body.to_string())),
            429 => Err(DriveError::RateLimited { retry_after: None }),
            _ => Err(DriveError::Api {
                status: status.as_u16(),
                body: body.to_string(),
            }),
        }
    }

    async fn json_or_error<T: serde::de::DeserializeOwned>(
        &self,
        resp: reqwest::Response,
    ) -> DriveResult<T> {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        self.check_status(status, &body)?;
        serde_json::from_str(&body).map_err(|e| {
            let preview: String = body.chars().take(200).collect();
            DriveError::Serialization(format!("{} body={}", e, preview))
        })
    }

    pub async fn list_files(&self, parent_id: Option<&str>) -> DriveResult<Vec<DriveFile>> {
        println!("[list_files] Chamando Drive API list_files...");
        self.apply_bandwidth_limit(1024).await;
        let token = self.ensure_token().await?;
        let mut q = "trashed=false".to_string();
        if let Some(pid) = parent_id {
            q = format!("'{}' in parents and trashed=false", pid);
        }

        let resp = self
            .client
            .get(format!("{}/files", self.drive_api_base))
            .header("Authorization", &token)
            .query(&[
                ("q", q.as_str()),
                ("pageSize", "200"),
                ("fields", "files(id,name,mimeType,size,createdTime,modifiedTime,md5Checksum,parents,trashed)"),
            ])
            .send()
            .await
            .map_err(|e| DriveError::Network(e.to_string()))?;

        let list: FileList = self.json_or_error(resp).await?;
        println!("[list_files] {} arquivos retornados pela API", list.files.len());
        Ok(list.files)
    }

    pub async fn get_metadata(&self, file_id: &str) -> DriveResult<DriveFile> {
        self.apply_bandwidth_limit(1024).await;
        let token = self.ensure_token().await?;
        let resp = self
            .client
            .get(format!("{}/files/{}?fields=*", self.drive_api_base, file_id))
            .header("Authorization", &token)
            .send()
            .await
            .map_err(|e| DriveError::Network(e.to_string()))?;
        self.json_or_error(resp).await
    }

    pub async fn upload(
        &self,
        name: &str,
        content: &[u8],
        mime_type: &str,
        parent_id: Option<&str>,
    ) -> DriveResult<DriveFile> {
        self.apply_bandwidth_limit(content.len() as u64).await;
        let token = self.ensure_token().await?;
        let url = format!("{}/files?uploadType=multipart&fields=*", self.upload_api_base);

        let boundary = format!("boundary_{}", uuid::Uuid::new_v4());
        let metadata = serde_json::json!({
            "name": name,
            "mimeType": mime_type,
            "parents": parent_id.map(|p| vec![p]),
        });
        let metadata_str =
            serde_json::to_string(&metadata).map_err(|e| DriveError::Serialization(e.to_string()))?;

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

        let resp = self
            .client
            .post(&url)
            .header("Authorization", &token)
            .header("Content-Type", format!("multipart/related; boundary={}", boundary))
            .body(body)
            .send()
            .await
            .map_err(|e| DriveError::Network(e.to_string()))?;

        self.json_or_error(resp).await
    }

    pub async fn download(&self, file_id: &str) -> DriveResult<Vec<u8>> {
        let token = self.ensure_token().await?;
        let resp = self
            .client
            .get(format!("{}/files/{}?alt=media", self.drive_api_base, file_id))
            .header("Authorization", &token)
            .send()
            .await
            .map_err(|e| DriveError::Network(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                404 => DriveError::NotFound(file_id.to_string()),
                _ => DriveError::Api { status: status.as_u16(), body },
            });
        }

        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| DriveError::Network(e.to_string()))
    }

    pub async fn delete(&self, file_id: &str) -> DriveResult<()> {
        self.apply_bandwidth_limit(1024).await;
        let token = self.ensure_token().await?;
        let resp = self
            .client
            .delete(format!("{}/files/{}", self.drive_api_base, file_id))
            .header("Authorization", &token)
            .send()
            .await
            .map_err(|e| DriveError::Network(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DriveError::Api { status: status.as_u16(), body });
        }
        Ok(())
    }
}
