use libresync_core::ui::state::{AccountInfo, AppUiState, SyncStatus};
use libresync_core::ui::tray::{TrayState, TrayMenu, tray_icon_for_state, tray_status_text};

#[test]
fn test_tray_state_from_synced() {
    assert_eq!(TrayState::from(&SyncStatus::Synced), TrayState::Synced);
}

#[test]
fn test_tray_state_from_syncing() {
    assert_eq!(TrayState::from(&SyncStatus::Syncing), TrayState::Syncing);
}

#[test]
fn test_tray_state_from_error() {
    assert_eq!(TrayState::from(&SyncStatus::Error("fail".into())), TrayState::Error);
}

#[test]
fn test_tray_state_from_paused() {
    assert_eq!(TrayState::from(&SyncStatus::Paused), TrayState::Paused);
}

#[test]
fn test_tray_state_from_offline() {
    assert_eq!(TrayState::from(&SyncStatus::Offline), TrayState::Offline);
}

#[test]
fn test_tray_state_from_conflict() {
    assert_eq!(TrayState::from(&SyncStatus::Conflict), TrayState::Synced);
}

#[test]
fn test_menu_reflects_paused_state() {
    let mut state = AppUiState::new();
    let account = AccountInfo::new("1".into(), "u@t.com".into(), "U".into());
    state.add_account(account);
    state.set_sync_status(SyncStatus::Paused);

    let menu = TrayMenu::from_state(&state);
    assert!(menu.is_paused);
    assert!(menu.can_pause);
    assert_eq!(menu.status_text, "Sync paused");
}

#[test]
fn test_menu_reflects_synced_state() {
    let mut state = AppUiState::new();
    let account = AccountInfo::new("1".into(), "u@t.com".into(), "U".into());
    state.add_account(account);
    state.set_sync_status(SyncStatus::Synced);

    let menu = TrayMenu::from_state(&state);
    assert!(!menu.is_paused);
    assert!(menu.can_pause);
    assert_eq!(menu.status_text, "All files synced");
}

#[test]
fn test_menu_can_pause_false_when_offline() {
    let mut state = AppUiState::new();
    state.set_sync_status(SyncStatus::Offline);
    let menu = TrayMenu::from_state(&state);
    assert!(!menu.can_pause);
}

#[test]
fn test_menu_can_pause_false_when_error() {
    let mut state = AppUiState::new();
    state.set_sync_status(SyncStatus::Error("critical".into()));
    let menu = TrayMenu::from_state(&state);
    assert!(!menu.can_pause);
}

#[test]
fn test_tray_icon_for_state() {
    let synced = AppUiState::new();
    assert_eq!(tray_icon_for_state(&synced), "synced");

    let mut syncing = AppUiState::new();
    syncing.set_sync_status(SyncStatus::Syncing);
    assert_eq!(tray_icon_for_state(&syncing), "syncing");

    let mut error = AppUiState::new();
    error.set_sync_status(SyncStatus::Error("x".into()));
    assert_eq!(tray_icon_for_state(&error), "error");

    let mut paused = AppUiState::new();
    paused.set_sync_status(SyncStatus::Paused);
    assert_eq!(tray_icon_for_state(&paused), "paused");

    let mut offline = AppUiState::new();
    offline.set_sync_status(SyncStatus::Offline);
    assert_eq!(tray_icon_for_state(&offline), "offline");
}

#[test]
fn test_tray_status_text() {
    let mut state = AppUiState::new();
    assert_eq!(tray_status_text(&state), "All files synced");

    state.set_sync_status(SyncStatus::Syncing);
    assert_eq!(tray_status_text(&state), "Syncing...");

    state.set_sync_status(SyncStatus::Error("disk full".into()));
    assert_eq!(tray_status_text(&state), "Error: disk full");

    state.set_sync_status(SyncStatus::Paused);
    assert_eq!(tray_status_text(&state), "Sync paused");

    state.set_sync_status(SyncStatus::Offline);
    assert_eq!(tray_status_text(&state), "Offline");

    state.set_sync_status(SyncStatus::Conflict);
    assert_eq!(tray_status_text(&state), "Conflict detected");
}
