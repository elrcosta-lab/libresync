use crate::ui::state::{AppUiState, SyncStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayState {
    Synced,
    Syncing,
    Error,
    Paused,
    Offline,
}

impl From<&SyncStatus> for TrayState {
    fn from(status: &SyncStatus) -> Self {
        match status {
            SyncStatus::Synced => TrayState::Synced,
            SyncStatus::Syncing => TrayState::Syncing,
            SyncStatus::Error(_) => TrayState::Error,
            SyncStatus::Paused => TrayState::Paused,
            SyncStatus::Offline => TrayState::Offline,
            SyncStatus::Conflict => TrayState::Synced,
        }
    }
}

pub struct TrayMenu {
    pub status_text: String,
    pub can_pause: bool,
    pub is_paused: bool,
}

impl TrayMenu {
    pub fn from_state(state: &AppUiState) -> Self {
        Self {
            status_text: tray_status_text(state),
            can_pause: !matches!(
                state.sync_status,
                SyncStatus::Offline | SyncStatus::Error(_)
            ),
            is_paused: state.is_paused,
        }
    }
}

pub fn tray_icon_for_state(state: &AppUiState) -> &str {
    match &state.sync_status {
        SyncStatus::Synced => "synced",
        SyncStatus::Syncing => "syncing",
        SyncStatus::Error(_) => "error",
        SyncStatus::Paused => "paused",
        SyncStatus::Offline => "offline",
        SyncStatus::Conflict => "synced",
    }
}

pub fn tray_status_text(state: &AppUiState) -> String {
    match &state.sync_status {
        SyncStatus::Synced => "All files synced".into(),
        SyncStatus::Syncing => "Syncing...".into(),
        SyncStatus::Error(msg) => format!("Error: {}", msg),
        SyncStatus::Paused => "Sync paused".into(),
        SyncStatus::Offline => "Offline".into(),
        SyncStatus::Conflict => "Conflict detected".into(),
    }
}
