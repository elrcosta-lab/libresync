use libresync_core::ui::notifications::{NotificationManager, NotificationType};

#[test]
fn test_rate_limit_blocks_second_notification() {
    let mut manager = NotificationManager::new(60, false);
    let first = manager.send(&NotificationType::Info { message: "first".into() });
    assert!(first, "first notification should be sent");

    let second = manager.send(&NotificationType::Info { message: "second".into() });
    assert!(!second, "second notification should be rate-limited");
}

#[test]
fn test_different_type_not_affected_by_rate_limit() {
    let mut manager = NotificationManager::new(60, false);

    let first = manager.send(&NotificationType::Info { message: "info msg".into() });
    assert!(first);

    let second = manager.send(&NotificationType::Info { message: "info again".into() });
    assert!(!second, "same type should be rate-limited");

    let warning = manager.send(&NotificationType::Warning { message: "warning msg".into() });
    assert!(warning, "different type should not be affected");
}

#[test]
fn test_suppress_when_focused_blocks_notifications() {
    let mut manager = NotificationManager::new(60, true);
    manager.set_focused(true);

    let result = manager.send(&NotificationType::Info { message: "test".into() });
    assert!(!result, "notification should be suppressed when focused");
}

#[test]
fn test_suppress_when_focused_allows_when_not_focused() {
    let mut manager = NotificationManager::new(60, true);
    manager.set_focused(false);

    let result = manager.send(&NotificationType::Info { message: "test".into() });
    assert!(result, "notification should be allowed when not focused");
}

#[test]
fn test_multiple_notification_types_have_independent_rate_limiters() {
    let mut manager = NotificationManager::new(60, false);

    assert!(manager.send(&NotificationType::SyncCompleted { file_count: 5 }));
    assert!(manager.send(&NotificationType::Conflict { file_name: "f.txt".into() }));
    assert!(manager.send(&NotificationType::AuthError { message: "auth failed".into() }));
    assert!(manager.send(&NotificationType::ConnectionLost));
    assert!(manager.send(&NotificationType::ConnectionRestored));
    assert!(manager.send(&NotificationType::Error { message: "err".into() }));
    assert!(manager.send(&NotificationType::Warning { message: "warn".into() }));
    assert!(manager.send(&NotificationType::Info { message: "info".into() }));

    assert!(!manager.send(&NotificationType::Info { message: "info2".into() }));
    assert!(!manager.send(&NotificationType::Warning { message: "warn2".into() }));
    assert!(!manager.send(&NotificationType::ConnectionLost));
}

#[test]
fn test_set_suppress_when_focused_toggle() {
    let mut manager = NotificationManager::new(60, true);
    manager.set_focused(true);
    assert!(!manager.send(&NotificationType::Info { message: "x".into() }));

    manager.set_suppress_when_focused(false);
    assert!(manager.send(&NotificationType::Info { message: "y".into() }));
}

#[test]
fn test_manager_new_defaults_to_not_focused() {
    let manager = NotificationManager::new(60, false);
    let mut m = manager;
    let result = m.send(&NotificationType::Info { message: "test".into() });
    assert!(result);
}
