use libresync_core::sync::error::SyncError;
use libresync_core::sync::state::{SyncState, SyncStateMachine};

fn assert_valid_transition(from: SyncState, to: SyncState) {
    let mut sm = SyncStateMachine::from(from);
    assert!(sm.can_transition_to(&to));
    assert!(sm.transition(to).is_ok());
    assert_eq!(sm.current(), to);
}

fn assert_invalid_transition(from: SyncState, to: SyncState) {
    let mut sm = SyncStateMachine::from(from);
    assert!(!sm.can_transition_to(&to));
    let result = sm.transition(to);
    assert!(result.is_err());
    match result {
        Err(SyncError::InvalidTransition { from: f, to: t }) => {
            assert_eq!(f, from);
            assert_eq!(t, to);
        }
        _ => panic!("expected InvalidTransition error"),
    }
    assert_eq!(sm.current(), from);
}

#[test]
fn test_initial_state_is_idle() {
    let sm = SyncStateMachine::new();
    assert_eq!(sm.current(), SyncState::Idle);
}

#[test]
fn test_idle_to_scanning_valid() {
    assert_valid_transition(SyncState::Idle, SyncState::Scanning);
}

#[test]
fn test_idle_to_paused_valid() {
    assert_valid_transition(SyncState::Idle, SyncState::Paused);
}

#[test]
fn test_idle_to_offline_valid() {
    assert_valid_transition(SyncState::Idle, SyncState::Offline);
}

#[test]
fn test_scanning_to_queuing_valid() {
    assert_valid_transition(SyncState::Scanning, SyncState::Queuing);
}

#[test]
fn test_scanning_to_error_valid() {
    assert_valid_transition(SyncState::Scanning, SyncState::Error);
}

#[test]
fn test_uploading_to_verifying_valid() {
    assert_valid_transition(SyncState::Uploading, SyncState::Verifying);
}

#[test]
fn test_uploading_to_conflict_valid() {
    assert_valid_transition(SyncState::Uploading, SyncState::Conflict);
}

#[test]
fn test_conflict_to_resolving_valid() {
    assert_valid_transition(SyncState::Conflict, SyncState::Resolving);
}

#[test]
fn test_verifying_to_idle_valid() {
    assert_valid_transition(SyncState::Verifying, SyncState::Idle);
}

#[test]
fn test_error_to_idle_valid() {
    assert_valid_transition(SyncState::Error, SyncState::Idle);
}

#[test]
fn test_paused_to_idle_valid() {
    assert_valid_transition(SyncState::Paused, SyncState::Idle);
}

#[test]
fn test_offline_to_idle_valid() {
    assert_valid_transition(SyncState::Offline, SyncState::Idle);
}

#[test]
fn test_retrying_to_queuing_valid() {
    assert_valid_transition(SyncState::Retrying, SyncState::Queuing);
}

#[test]
fn test_scanning_to_idle_invalid() {
    assert_invalid_transition(SyncState::Scanning, SyncState::Idle);
}

#[test]
fn test_idle_to_uploading_invalid() {
    assert_invalid_transition(SyncState::Idle, SyncState::Uploading);
}

#[test]
fn test_idle_to_downloading_invalid() {
    assert_invalid_transition(SyncState::Idle, SyncState::Downloading);
}

#[test]
fn test_queuing_to_idle_invalid() {
    assert_invalid_transition(SyncState::Queuing, SyncState::Idle);
}

#[test]
fn test_paused_to_scanning_invalid() {
    assert_invalid_transition(SyncState::Paused, SyncState::Scanning);
}

#[test]
fn test_offline_to_scanning_invalid() {
    assert_invalid_transition(SyncState::Offline, SyncState::Scanning);
}

#[test]
fn test_resolving_transitions() {
    assert_valid_transition(SyncState::Resolving, SyncState::Idle);
    assert_valid_transition(SyncState::Resolving, SyncState::Queuing);
    assert_valid_transition(SyncState::Resolving, SyncState::Error);
    assert_invalid_transition(SyncState::Resolving, SyncState::Uploading);
}
