use std::time::Duration;

use serde::Deserialize;

use crate::auth::error::{AuthError, AuthResult};
use crate::auth::token_exchange::TokenResponse;

const DEVICE_CODE_URL: &str = "https://oauth2.googleapis.com/device/code";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const SCOPE: &str = "https://www.googleapis.com/auth/drive.file";

#[derive(Debug, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_url: String,
    pub expires_in: u64,
    pub interval: u64,
}

pub struct DeviceFlow {
    client_id: String,
    client: reqwest::Client,
}

impl DeviceFlow {
    pub fn new(client_id: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build reqwest Client");
        Self {
            client_id: client_id.to_string(),
            client,
        }
    }

    pub async fn request_device_code(&self) -> AuthResult<DeviceCodeResponse> {
        let params = [
            ("client_id", self.client_id.as_str()),
            ("scope", SCOPE),
        ];

        let resp = self
            .client
            .post(DEVICE_CODE_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| AuthError::NetworkError(e.to_string()))?;

        let status = resp.status();
        if status.is_success() {
            resp.json::<DeviceCodeResponse>()
                .await
                .map_err(|e| AuthError::StateError(format!("device code json: {}", e)))
        } else {
            let body = resp.text().await.unwrap_or_default();
            Err(AuthError::StateError(format!(
                "device code HTTP {}: {}",
                status, body
            )))
        }
    }

    pub async fn poll_for_token(&self, device_code: &DeviceCodeResponse) -> AuthResult<TokenResponse> {
        let interval = Duration::from_secs(device_code.interval.max(5));
        let timeout = Duration::from_secs(device_code.expires_in);

        let start = std::time::Instant::now();

        loop {
            if start.elapsed() >= timeout {
                return Err(AuthError::LoginTimeout);
            }

            tokio::time::sleep(interval).await;

            let params = [
                ("client_id", self.client_id.as_str()),
                ("device_code", &device_code.device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ];

            let resp = self
                .client
                .post(TOKEN_URL)
                .form(&params)
                .send()
                .await
                .map_err(|e| AuthError::NetworkError(e.to_string()))?;

            let status = resp.status();
            if status.is_success() {
                return resp
                    .json::<TokenResponse>()
                    .await
                    .map_err(|e| AuthError::StateError(format!("token json: {}", e)));
            }

            if status.as_u16() == 400 {
                let body = resp.text().await.unwrap_or_default();
                if body.contains("authorization_pending") || body.contains("slow_down") {
                    continue;
                }
                if body.contains("access_denied") {
                    return Err(AuthError::LoginDenied);
                }
                if body.contains("expired_token") {
                    return Err(AuthError::LoginTimeout);
                }
                return Err(AuthError::StateError(format!("poll error: {}", body)));
            }

            if status.as_u16() == 429 {
                return Err(AuthError::RateLimited { retry_after: None });
            }

            return Err(AuthError::NetworkError(format!("HTTP {}", status)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_flow_creation() {
        let flow = DeviceFlow::new("test-client-id");
        assert_eq!(flow.client_id, "test-client-id");
    }
}
