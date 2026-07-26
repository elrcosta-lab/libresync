use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AccountStatus {
    Active,
    Revoked,
    Expired,
    RequiresReauth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub scope: String,
    pub token_expires_at: i64,
    pub status: AccountStatus,
    pub is_active: bool,
    pub created_at: i64,
    pub last_sync_at: Option<i64>,
    pub quota_total: Option<i64>,
    pub quota_used: Option<i64>,
}

impl Account {
    pub fn new(id: String, email: String, display_name: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        Self {
            id,
            email,
            display_name,
            avatar_url: None,
            scope: "drive.file".into(),
            token_expires_at: 0,
            status: AccountStatus::Active,
            is_active: true,
            created_at: now,
            last_sync_at: None,
            quota_total: None,
            quota_used: None,
        }
    }

    pub fn revoke(&mut self) {
        self.status = AccountStatus::Revoked;
        self.is_active = false;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSet {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub expires_at: i64,
    pub token_type: String,
    pub scope: String,
}

impl TokenSet {
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        now >= self.expires_at
    }

    pub fn should_refresh(&self, margin_secs: i64) -> bool {
        if self.refresh_token.is_none() {
            return false;
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        now + margin_secs >= self.expires_at
    }
}
