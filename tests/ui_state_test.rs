use libresync_core::ui::state::*;
use libresync_core::ui::config::UIConfig;

#[test]
fn test_initial_state_is_login() {
    let state = AppUiState::new();
    assert_eq!(state.screen, AppScreen::Login);
    assert!(state.active_account.is_none());
    assert!(state.accounts.is_empty());
    assert!(state.folders.is_empty());
    assert!(!state.is_paused);
    assert!(state.is_online);
}

#[test]
fn test_add_account_changes_screen_to_main() {
    let mut state = AppUiState::new();
    let account = AccountInfo::new("1".into(), "a@b.com".into(), "User".into());
    state.add_account(account);
    assert_eq!(state.screen, AppScreen::Main);
    assert_eq!(state.accounts.len(), 1);
}

#[test]
fn test_add_account_does_not_change_screen_if_already_main() {
    let mut state = AppUiState::new();
    state.set_screen(AppScreen::Main);
    let account = AccountInfo::new("1".into(), "a@b.com".into(), "User".into());
    state.add_account(account);
    assert_eq!(state.screen, AppScreen::Main);
}

#[test]
fn test_set_sync_status_transitions() {
    let mut state = AppUiState::new();
    assert_eq!(state.sync_status, SyncStatus::Synced);

    state.set_sync_status(SyncStatus::Syncing);
    assert_eq!(state.sync_status, SyncStatus::Syncing);

    state.set_sync_status(SyncStatus::Error("disk full".into()));
    assert_eq!(state.sync_status, SyncStatus::Error("disk full".into()));

    state.set_sync_status(SyncStatus::Paused);
    assert!(state.is_paused);

    state.set_sync_status(SyncStatus::Conflict);
    assert_eq!(state.sync_status, SyncStatus::Conflict);
}

#[test]
fn test_recent_activity_limited_to_100() {
    let mut state = AppUiState::new();
    for i in 0..101 {
        state.push_activity(SyncActivity::new(
            "modify".into(),
            format!("/path/{}", i),
            format!("event {}", i),
        ));
    }
    assert_eq!(state.activity.len(), 100);
    assert_eq!(state.activity[0].file_path, "/path/1");
    assert_eq!(state.activity[99].file_path, "/path/100");
}

#[test]
fn test_toggle_pause_resume() {
    let mut state = AppUiState::new();
    assert!(!state.is_paused);

    let paused = state.toggle_pause();
    assert!(paused);
    assert!(state.is_paused);
    assert_eq!(state.sync_status, SyncStatus::Paused);

    let resumed = state.toggle_pause();
    assert!(!resumed);
    assert!(!state.is_paused);
    assert_eq!(state.sync_status, SyncStatus::Synced);
}

#[test]
fn test_onboarding_steps_increment() {
    let mut state = AppUiState::new();
    state.set_screen(AppScreen::Onboarding { step: 1 });
    assert_eq!(state.screen, AppScreen::Onboarding { step: 1 });

    state.next_onboarding_step();
    assert_eq!(state.screen, AppScreen::Onboarding { step: 2 });

    state.next_onboarding_step();
    assert_eq!(state.screen, AppScreen::Onboarding { step: 3 });

    state.next_onboarding_step();
    assert_eq!(state.screen, AppScreen::Main);
}

#[test]
fn test_remove_active_account_clears_and_goes_to_login() {
    let mut state = AppUiState::new();
    let account = AccountInfo::new("1".into(), "a@b.com".into(), "User".into());
    state.add_account(account);
    state.active_account = state.accounts.first().cloned();
    assert!(state.active_account.is_some());

    state.remove_account("1");
    assert!(state.active_account.is_none());
    assert!(state.accounts.is_empty());
    assert_eq!(state.screen, AppScreen::Login);
}

#[test]
fn test_remove_account_keeps_other_active() {
    let mut state = AppUiState::new();
    let a1 = AccountInfo::new("1".into(), "a@b.com".into(), "A".into());
    let a2 = AccountInfo::new("2".into(), "b@c.com".into(), "B".into());
    state.add_account(a1);
    state.add_account(a2);
    state.active_account = state.accounts.first().cloned();

    state.remove_account("1");
    assert_eq!(state.accounts.len(), 1);
    assert_eq!(state.screen, AppScreen::Main);
}

#[test]
fn test_set_sync_status_paused_sets_is_paused() {
    let mut state = AppUiState::new();
    state.set_sync_status(SyncStatus::Paused);
    assert!(state.is_paused);
    assert_eq!(state.sync_status, SyncStatus::Paused);
}

#[test]
fn test_default_config_values() {
    let config = UIConfig::default();
    assert!(config.auto_sync_on_login);
    assert!(!config.notify_only_errors);
    assert!(config.minimize_to_tray);
    assert_eq!(config.locale, "pt-BR");
}
