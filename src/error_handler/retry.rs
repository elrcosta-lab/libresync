use rand::Rng;
use std::time::Duration;

pub trait IsRetryable {
    fn is_retryable(&self) -> bool;
}

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            base_delay_ms: 1000,
            max_delay_ms: 30_000,
            jitter: true,
        }
    }
}

impl RetryPolicy {
    pub fn backoff(&self, attempt: u32) -> Duration {
        let exponential = self.base_delay_ms.saturating_mul(2u64.saturating_pow(attempt));
        let clamped = exponential.min(self.max_delay_ms);
        if self.jitter {
            let jitter_amount = rand::thread_rng().gen_range(0..=clamped / 4);
            Duration::from_millis(clamped + jitter_amount)
        } else {
            Duration::from_millis(clamped)
        }
    }

    pub fn should_retry<E: IsRetryable>(&self, attempt: u32, error: &E) -> bool {
        if attempt >= self.max_attempts.saturating_sub(1) {
            return false;
        }
        error.is_retryable()
    }

    pub fn is_retryable<E: IsRetryable>(&self, error: &E) -> bool {
        error.is_retryable()
    }
}

pub async fn with_retry<F, Fut, T, E>(policy: &RetryPolicy, f: F) -> Result<T, E>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display + IsRetryable,
{
    let attempts = policy.max_attempts.max(1);
    let mut last_error = None;

    for attempt in 0..attempts {
        match f().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                if policy.should_retry(attempt, &e) {
                    let delay = policy.backoff(attempt);
                    tokio::time::sleep(delay).await;
                    last_error = Some(e);
                } else {
                    return Err(e);
                }
            }
        }
    }

    Err(last_error.expect("with_retry: no attempts executed"))
}

impl IsRetryable for crate::drive::error::DriveError {
    fn is_retryable(&self) -> bool {
        match self {
            Self::Network(_) => true,
            Self::RateLimited { .. } => true,
            Self::Api { status, .. } if *status >= 500 => true,
            _ => false,
        }
    }
}
