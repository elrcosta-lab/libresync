use libresync_core::notification::{DesktopNotifier, NotificationSender, NotificationType};

#[test]
fn test_notification_sender_creates_notification() {
    let sender = NotificationSender::new();

    assert_eq!(
        sender.send(&NotificationType::SyncCompleted { file_count: 5 }),
        "Sincronização concluída: 5 arquivos"
    );
    assert_eq!(
        sender.send(&NotificationType::Conflict { file_name: "f.txt".into() }),
        "Conflito detectado em f.txt"
    );
    assert_eq!(
        sender.send(&NotificationType::AuthError { message: "invalid token".into() }),
        "Erro de autenticação: invalid token"
    );
    assert_eq!(
        sender.send(&NotificationType::ConnectionLost),
        "Conexão perdida. Tentando reconectar..."
    );
    assert_eq!(
        sender.send(&NotificationType::ConnectionRestored),
        "Conexão restaurada"
    );
    assert_eq!(
        sender.send(&NotificationType::Error { message: "disk full".into() }),
        "Erro: disk full"
    );
    assert_eq!(
        sender.send(&NotificationType::Warning { message: "low space".into() }),
        "Aviso: low space"
    );
    assert_eq!(
        sender.send(&NotificationType::Info { message: "hello".into() }),
        "hello"
    );
}

#[test]
fn test_desktop_notifier_rate_limits() {
    let mut notifier = DesktopNotifier::new(60, false);

    let first = notifier.notify(&NotificationType::Info { message: "first".into() });
    assert!(first.is_some(), "first notification should be sent");

    let second = notifier.notify(&NotificationType::Info { message: "second".into() });
    assert!(second.is_none(), "second notification should be rate-limited");
}

#[test]
fn test_desktop_notifier_suppress() {
    let mut notifier = DesktopNotifier::new(60, true);
    notifier.set_focused(true);

    let result = notifier.notify(&NotificationType::Info { message: "test".into() });
    assert!(result.is_none(), "notification should be suppressed when focused");

    notifier.set_focused(false);
    let result = notifier.notify(&NotificationType::Info { message: "test2".into() });
    assert!(result.is_some(), "notification should be allowed when not focused");
}

#[test]
fn test_different_types_independent() {
    let mut notifier = DesktopNotifier::new(60, false);

    assert!(notifier
        .notify(&NotificationType::SyncCompleted { file_count: 5 })
        .is_some());
    assert!(notifier
        .notify(&NotificationType::Conflict { file_name: "f.txt".into() })
        .is_some());
    assert!(notifier
        .notify(&NotificationType::AuthError { message: "fail".into() })
        .is_some());
    assert!(notifier.notify(&NotificationType::ConnectionLost).is_some());
    assert!(notifier.notify(&NotificationType::ConnectionRestored).is_some());
    assert!(notifier
        .notify(&NotificationType::Error { message: "err".into() })
        .is_some());
    assert!(notifier
        .notify(&NotificationType::Warning { message: "warn".into() })
        .is_some());
    assert!(notifier
        .notify(&NotificationType::Info { message: "info".into() })
        .is_some());

    assert!(notifier
        .notify(&NotificationType::Info { message: "info2".into() })
        .is_none());
    assert!(notifier
        .notify(&NotificationType::Warning { message: "warn2".into() })
        .is_none());
    assert!(notifier.notify(&NotificationType::ConnectionLost).is_none());
}
