use libresync_core::ui::state::{AccountInfo, AppUiState, SyncStatus, SyncActivity};
use libresync_core::ui::config::UIConfig;
use libresync_core::ui::commands::*;

fn make_state() -> AppUiState {
    let mut state = AppUiState::new();
    let account = AccountInfo::new("a1".into(), "u@t.com".into(), "U".into());
    state.add_account(account);
    state.set_sync_status(SyncStatus::Syncing);
    state
}

#[test]
fn test_get_sync_state_returns_correct_status() {
    let state = make_state();
    assert_eq!(handle_get_sync_state(&state), SyncStatus::Syncing);
}

#[test]
fn test_get_sync_state_returns_synced_when_idle() {
    let state = AppUiState::new();
    assert_eq!(handle_get_sync_state(&state), SyncStatus::Synced);
}

#[test]
fn test_get_recent_events_returns_last_n() {
    let mut state = make_state();
    for i in 0..10 {
        state.push_activity(SyncActivity::new(
            "modify".into(),
            format!("/path/{}", i),
            format!("event {}", i),
        ));
    }
    let events = handle_get_recent_events(&state, 3);
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].file_path, "/path/7");
    assert_eq!(events[2].file_path, "/path/9");
}

#[test]
fn test_get_recent_events_limit_greater_than_available() {
    let mut state = make_state();
    state.push_activity(SyncActivity::new("modify".into(), "/path/0".into(), "e0".into()));
    state.push_activity(SyncActivity::new("modify".into(), "/path/1".into(), "e1".into()));

    let events = handle_get_recent_events(&state, 10);
    assert_eq!(events.len(), 2);
}

#[test]
fn test_get_recent_events_empty() {
    let state = make_state();
    let events = handle_get_recent_events(&state, 5);
    assert!(events.is_empty());
}

#[test]
fn test_update_config_alters_state() {
    let mut state = make_state();
    let new_config = UIConfig {
        client_id: String::new(),
        sync_folder: String::new(),
        bandwidth_limit: 0,
        auto_start: false,
        polling_interval: 30,
        auto_sync_on_login: false,
        notify_only_errors: true,
        minimize_to_tray: false,
        locale: "en-US".into(),
    };
    let returned = handle_update_config(&mut state, new_config);
    assert!(!state.config.auto_sync_on_login);
    assert!(state.config.notify_only_errors);
    assert!(!state.config.minimize_to_tray);
    assert_eq!(state.config.locale, "en-US");
    assert!(!returned.auto_sync_on_login);
}

#[test]
fn test_toggle_pause_toggles_state() {
    let mut state = make_state();
    assert!(!state.is_paused);

    let paused = handle_toggle_pause(&mut state);
    assert!(paused);
    assert!(state.is_paused);

    let resumed = handle_toggle_pause(&mut state);
    assert!(!resumed);
    assert!(!state.is_paused);
}

#[test]
fn test_toggle_pause_sets_sync_status() {
    let mut state = make_state();
    handle_toggle_pause(&mut state);
    assert_eq!(state.sync_status, SyncStatus::Paused);

    handle_toggle_pause(&mut state);
    assert_eq!(state.sync_status, SyncStatus::Synced);
}
