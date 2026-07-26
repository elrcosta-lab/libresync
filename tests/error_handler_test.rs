use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use libresync_core::error_handler::connectivity::{ConnectivityChecker, ConnectivityState};
use libresync_core::error_handler::retry::{IsRetryable, RetryPolicy, with_retry};

#[derive(Debug, Clone)]
struct TestError {
    kind: &'static str,
}

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl IsRetryable for TestError {
    fn is_retryable(&self) -> bool {
        matches!(self.kind, "network" | "429" | "500")
    }
}

// --- Retry tests ---

#[tokio::test]
async fn test_backoff_doubles_each_attempt() {
    let policy = RetryPolicy {
        max_attempts: 5,
        base_delay_ms: 100,
        max_delay_ms: 10_000,
        jitter: false,
    };

    let d0 = policy.backoff(0).as_millis();
    let d1 = policy.backoff(1).as_millis();
    let d2 = policy.backoff(2).as_millis();

    assert_eq!(d0, 100, "backoff(0) should be base");
    assert_eq!(d1, 200, "backoff(1) should be 2x base");
    assert_eq!(d2, 400, "backoff(2) should be 4x base");
}

#[tokio::test]
async fn test_jitter_adds_variation() {
    let policy = RetryPolicy {
        max_attempts: 5,
        base_delay_ms: 10_000,
        max_delay_ms: 100_000,
        jitter: true,
    };

    let mut values = Vec::new();
    for _ in 0..20 {
        values.push(policy.backoff(0).as_millis());
    }
    let all_same = values.windows(2).all(|w| w[0] == w[1]);
    assert!(!all_same, "jitter should produce varying delays");
}

#[tokio::test]
async fn test_should_retry_true_for_network_error() {
    let policy = RetryPolicy::default();
    let err = TestError { kind: "network" };
    assert!(policy.should_retry(0, &err));
}

#[tokio::test]
async fn test_should_retry_false_for_404() {
    let policy = RetryPolicy::default();
    let err = TestError { kind: "404" };
    assert!(!policy.should_retry(0, &err));
}

#[tokio::test]
async fn test_should_retry_true_for_429() {
    let policy = RetryPolicy::default();
    let err = TestError { kind: "429" };
    assert!(policy.should_retry(0, &err));
}

#[tokio::test]
async fn test_with_retry_success_on_third_attempt() {
    let policy = RetryPolicy {
        max_attempts: 5,
        base_delay_ms: 10,
        max_delay_ms: 100,
        jitter: false,
    };

    let attempts = Arc::new(AtomicU32::new(0));
    let a = Arc::clone(&attempts);

    let result = with_retry(&policy, move || {
        let a = Arc::clone(&a);
        async move {
            let prev = a.fetch_add(1, Ordering::SeqCst);
            if prev >= 2 {
                Ok(42)
            } else {
                Err(TestError { kind: "network" })
            }
        }
    })
    .await;

    assert_eq!(result.unwrap(), 42);
    assert_eq!(attempts.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn test_with_retry_fails_after_max_attempts() {
    let policy = RetryPolicy {
        max_attempts: 3,
        base_delay_ms: 10,
        max_delay_ms: 100,
        jitter: false,
    };

    let result = with_retry(&policy, || async {
        Err::<(), _>(TestError { kind: "network" })
    })
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_max_delay_respected() {
    let policy = RetryPolicy {
        max_attempts: 10,
        base_delay_ms: 10_000,
        max_delay_ms: 5_000,
        jitter: false,
    };

    let delay = policy.backoff(5).as_millis();
    assert!(
        delay <= 5_000,
        "backoff should not exceed max_delay_ms: got {}",
        delay
    );
}

// --- Connectivity tests ---

#[test]
fn test_initial_state_is_online() {
    let checker = ConnectivityChecker::new(3, Duration::from_secs(30));
    assert_eq!(checker.state(), ConnectivityState::Online);
}

#[test]
fn test_consecutive_failures_change_to_offline() {
    let mut checker = ConnectivityChecker::new(3, Duration::from_secs(30));

    checker.on_failure();
    assert_eq!(checker.state(), ConnectivityState::Degraded);

    checker.on_failure();
    assert_eq!(checker.state(), ConnectivityState::Degraded);

    checker.on_failure();
    assert_eq!(checker.state(), ConnectivityState::Offline);
}

#[test]
fn test_success_resets_counter() {
    let mut checker = ConnectivityChecker::new(3, Duration::from_secs(30));

    checker.on_failure();
    checker.on_failure();
    assert_eq!(checker.consecutive_failures(), 2);

    checker.on_success();
    assert_eq!(checker.state(), ConnectivityState::Online);
    assert_eq!(checker.consecutive_failures(), 0);
}

#[test]
fn test_configurable_threshold() {
    let mut checker = ConnectivityChecker::new(1, Duration::from_secs(30));

    assert_eq!(checker.state(), ConnectivityState::Online);

    checker.on_failure();
    assert_eq!(checker.state(), ConnectivityState::Offline);
}

#[tokio::test]
async fn test_check_updates_state() {
    let mut checker = ConnectivityChecker::new(2, Duration::from_secs(30));

    let state = checker
        .check(|| async { Err::<(), ()>(()) })
        .await;
    assert_eq!(state, ConnectivityState::Degraded);

    let state = checker
        .check(|| async { Err::<(), ()>(()) })
        .await;
    assert_eq!(state, ConnectivityState::Offline);

    let state = checker
        .check(|| async { Ok::<(), ()>(()) })
        .await;
    assert_eq!(state, ConnectivityState::Online);
}
