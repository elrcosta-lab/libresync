use std::time::Duration;

use libresync_core::watcher::config::WatcherConfig;
use libresync_core::watcher::error::WatcherError;
use libresync_core::watcher::event::FileEvent;
use libresync_core::watcher::ignore::IgnoreRules;

use libresync_core::watcher::engine::FileWatcher;

fn make_watcher(
    debounce_ms: u64,
    ignore_rules: IgnoreRules,
) -> (
    FileWatcher,
    tokio::sync::mpsc::UnboundedReceiver<FileEvent>,
) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let config = WatcherConfig {
        debounce_ms,
        ..Default::default()
    };
    let watcher = FileWatcher::new(config, ignore_rules, tx);
    (watcher, rx)
}

#[tokio::test]
async fn test_create_watcher_with_config() {
    let (watcher, _rx) = make_watcher(500, IgnoreRules::default());
    let config = WatcherConfig {
        debounce_ms: 500,
        ..Default::default()
    };
    assert_eq!(watcher.config().debounce_ms, config.debounce_ms);
    assert_eq!(
        watcher.config().fallback_polling_interval_s,
        config.fallback_polling_interval_s
    );
}

#[tokio::test]
async fn test_debounce_duplicates_in_window() {
    let (watcher, mut rx) = make_watcher(150, IgnoreRules::new(vec![]));

    watcher.on_event(FileEvent::Modified("f.txt".into()));
    tokio::time::sleep(Duration::from_millis(20)).await;

    watcher.on_event(FileEvent::Modified("f.txt".into()));
    tokio::time::sleep(Duration::from_millis(20)).await;

    watcher.on_event(FileEvent::Created("f.txt".into()));

    tokio::time::sleep(Duration::from_millis(300)).await;
    tokio::task::yield_now().await;

    let result = rx.try_recv();
    assert!(result.is_ok(), "expected event, got {:?}", result.err());
    assert_eq!(result.unwrap(), FileEvent::Created("f.txt".into()));
    assert!(
        rx.try_recv().is_err(),
        "expected only one event after debounce"
    );
}

#[tokio::test]
async fn test_rename_detected() {
    let (watcher, mut rx) = make_watcher(150, IgnoreRules::new(vec![]));

    watcher.on_event(FileEvent::Deleted("old.txt".into()));
    tokio::time::sleep(Duration::from_millis(20)).await;

    watcher.on_event(FileEvent::Created("new.txt".into()));

    tokio::time::sleep(Duration::from_millis(300)).await;
    tokio::task::yield_now().await;

    let result = rx.try_recv();
    assert!(result.is_ok(), "expected rename event");
    assert_eq!(
        result.unwrap(),
        FileEvent::Renamed {
            from: "old.txt".into(),
            to: "new.txt".into()
        }
    );
    assert!(rx.try_recv().is_err(), "only rename should be emitted");
}

#[tokio::test]
async fn test_ignore_rules_applied_before_emit() {
    let mut custom = IgnoreRules::default_patterns();
    custom.push("*.ignored".into());
    let rules = IgnoreRules::new(custom);

    let (watcher, mut rx) = make_watcher(150, rules);

    watcher.on_event(FileEvent::Modified("file.ignored".into()));
    tokio::time::sleep(Duration::from_millis(20)).await;

    watcher.on_event(FileEvent::Modified("important.txt".into()));

    tokio::time::sleep(Duration::from_millis(300)).await;
    tokio::task::yield_now().await;

    let result = rx.try_recv();
    assert!(result.is_ok(), "expected non-ignored event");
    assert_eq!(
        result.unwrap(),
        FileEvent::Modified("important.txt".into())
    );
    assert!(
        rx.try_recv().is_err(),
        "ignored event must not be emitted"
    );
}

#[tokio::test]
async fn test_multiple_paths_not_grouped() {
    let (watcher, mut rx) = make_watcher(150, IgnoreRules::new(vec![]));

    watcher.on_event(FileEvent::Modified("a.txt".into()));
    tokio::time::sleep(Duration::from_millis(20)).await;

    watcher.on_event(FileEvent::Created("b.txt".into()));

    tokio::time::sleep(Duration::from_millis(300)).await;
    tokio::task::yield_now().await;

    let mut received: Vec<FileEvent> = Vec::new();
    while let Ok(ev) = rx.try_recv() {
        received.push(ev);
    }
    received.sort_by(|a, b| a.path().cmp(b.path()));

    assert_eq!(received.len(), 2, "both paths should emit events");
    assert!(received.contains(&FileEvent::Modified("a.txt".into())));
    assert!(received.contains(&FileEvent::Created("b.txt".into())));
}

#[tokio::test]
async fn test_stop_does_not_emit() {
    let (watcher, mut rx) = make_watcher(150, IgnoreRules::new(vec![]));

    watcher.on_event(FileEvent::Modified("f.txt".into()));
    tokio::time::sleep(Duration::from_millis(20)).await;

    watcher.stop();

    tokio::time::sleep(Duration::from_millis(300)).await;
    tokio::task::yield_now().await;

    assert!(
        rx.try_recv().is_err(),
        "stop must prevent event emission"
    );
}

#[tokio::test]
async fn test_on_event_sends_through_channel() {
    let (watcher, mut rx) = make_watcher(100, IgnoreRules::new(vec![]));

    watcher.on_event(FileEvent::Modified("hello.txt".into()));

    tokio::time::sleep(Duration::from_millis(200)).await;
    tokio::task::yield_now().await;

    let ev = rx
        .try_recv()
        .expect("event should be sent through channel");
    assert_eq!(ev, FileEvent::Modified("hello.txt".into()));
}

#[test]
fn test_watch_valid_path() {
    let dir = tempfile::tempdir().unwrap();
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let watcher = FileWatcher::new(WatcherConfig::default(), IgnoreRules::default(), tx);
    let result = watcher.watch(dir.path().to_str().unwrap());
    assert!(result.is_ok());
}

#[test]
fn test_watch_invalid_path() {
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let watcher = FileWatcher::new(WatcherConfig::default(), IgnoreRules::default(), tx);
    let result = watcher.watch("/nonexistent/path/that/does/not/exist");
    assert!(result.is_err());
    match result {
        Err(WatcherError::PathNotFound(p)) => {
            assert_eq!(p, "/nonexistent/path/that/does/not/exist");
        }
        _ => panic!("expected PathNotFound error"),
    }
}
