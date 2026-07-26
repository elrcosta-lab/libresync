use serde::{Deserialize, Serialize};

use crate::ui::config::UIConfig;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AppScreen {
    Login,
    Onboarding { step: u8 },
    Main,
    Preferences,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncStatus {
    Synced,
    Syncing,
    Error(String),
    Paused,
    Offline,
    Conflict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub quota_used: Option<i64>,
    pub quota_total: Option<i64>,
    pub is_active: bool,
}

impl AccountInfo {
    pub fn new(id: String, email: String, display_name: String) -> Self {
        Self {
            id,
            email,
            display_name,
            avatar_url: None,
            quota_used: None,
            quota_total: None,
            is_active: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderInfo {
    pub id: String,
    pub local_path: String,
    pub file_count: u64,
    pub status: SyncStatus,
    pub last_sync_at: Option<i64>,
}

impl FolderInfo {
    pub fn new(id: String, local_path: String) -> Self {
        Self {
            id,
            local_path,
            file_count: 0,
            status: SyncStatus::Synced,
            last_sync_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncActivity {
    pub event_type: String,
    pub file_path: String,
    pub file_size: Option<i64>,
    pub timestamp: i64,
    pub level: String,
    pub message: String,
}

impl SyncActivity {
    pub fn new(event_type: String, file_path: String, message: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        Self {
            event_type,
            file_path,
            file_size: None,
            timestamp: now,
            level: "info".into(),
            message,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppUiState {
    pub screen: AppScreen,
    pub active_account: Option<AccountInfo>,
    pub accounts: Vec<AccountInfo>,
    pub folders: Vec<FolderInfo>,
    pub sync_status: SyncStatus,
    pub activity: Vec<SyncActivity>,
    pub is_paused: bool,
    pub is_online: bool,
    pub config: UIConfig,
}

impl AppUiState {
    pub fn new() -> Self {
        Self {
            screen: AppScreen::Login,
            active_account: None,
            accounts: Vec::new(),
            folders: Vec::new(),
            sync_status: SyncStatus::Synced,
            activity: Vec::new(),
            is_paused: false,
            is_online: true,
            config: UIConfig::default(),
        }
    }

    pub fn add_account(&mut self, account: AccountInfo) {
        let was_login = self.accounts.is_empty() && self.screen == AppScreen::Login;
        self.accounts.push(account);
        if was_login {
            self.screen = AppScreen::Main;
        }
    }

    pub fn remove_account(&mut self, id: &str) {
        let was_active = self
            .active_account
            .as_ref()
            .map(|a| a.id.as_str() == id)
            .unwrap_or(false);
        if was_active {
            self.active_account = self.accounts.iter().find(|a| a.id != id).cloned();
        }
        self.accounts.retain(|a| a.id != id);
        if self.accounts.is_empty() {
            self.screen = AppScreen::Login;
            self.active_account = None;
        }
    }

    pub fn set_sync_status(&mut self, status: SyncStatus) {
        match &status {
            SyncStatus::Paused => self.is_paused = true,
            _ if self.sync_status == SyncStatus::Paused => self.is_paused = false,
            _ => {}
        }
        self.sync_status = status;
    }

    pub fn push_activity(&mut self, activity: SyncActivity) {
        self.activity.push(activity);
        if self.activity.len() > 100 {
            self.activity.remove(0);
        }
    }

    pub fn toggle_pause(&mut self) -> bool {
        self.is_paused = !self.is_paused;
        if self.is_paused {
            self.sync_status = SyncStatus::Paused;
        } else if self.sync_status == SyncStatus::Paused {
            self.sync_status = SyncStatus::Synced;
        }
        self.is_paused
    }

    pub fn set_screen(&mut self, screen: AppScreen) {
        self.screen = screen;
    }

    pub fn next_onboarding_step(&mut self) {
        if let AppScreen::Onboarding { step } = self.screen {
            if step < 3 {
                self.screen = AppScreen::Onboarding { step: step + 1 };
            } else {
                self.screen = AppScreen::Main;
            }
        }
    }
}

impl Default for AppUiState {
    fn default() -> Self {
        Self::new()
    }
}
