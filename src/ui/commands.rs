use serde::{Deserialize, Serialize};

use crate::ui::config::UIConfig;
use crate::ui::state::{AppUiState, SyncActivity, SyncStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncProgressInfo {
    pub total_jobs: u64,
    pub completed_jobs: u64,
    pub failed_jobs: u64,
    pub progress_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppErrorInfo {
    pub code: String,
    pub message: String,
    pub detail: Option<String>,
    pub action: Option<String>,
}

pub fn handle_get_sync_state(state: &AppUiState) -> SyncStatus {
    state.sync_status.clone()
}

pub fn handle_toggle_pause(state: &mut AppUiState) -> bool {
    state.toggle_pause()
}

pub fn handle_get_recent_events(state: &AppUiState, limit: usize) -> Vec<SyncActivity> {
    let len = state.activity.len();
    if len == 0 {
        return Vec::new();
    }
    let start = len.saturating_sub(limit);
    state.activity[start..].to_vec()
}

pub fn handle_update_config(state: &mut AppUiState, config: UIConfig) -> UIConfig {
    state.config = config.clone();
    config
}
