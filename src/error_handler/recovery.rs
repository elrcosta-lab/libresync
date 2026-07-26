use crate::error_handler::connectivity::ConnectivityChecker;
use crate::error_handler::error::HandlerError;
use crate::error_handler::retry::{IsRetryable, RetryPolicy, with_retry};

pub struct RecoveryManager {
    retry_policy: RetryPolicy,
    connectivity: ConnectivityChecker,
}

impl RecoveryManager {
    pub fn new(retry_policy: RetryPolicy, connectivity: ConnectivityChecker) -> Self {
        Self {
            retry_policy,
            connectivity,
        }
    }

    pub fn retry_policy(&self) -> &RetryPolicy {
        &self.retry_policy
    }

    pub fn connectivity(&self) -> &ConnectivityChecker {
        &self.connectivity
    }

    pub fn connectivity_mut(&mut self) -> &mut ConnectivityChecker {
        &mut self.connectivity
    }

    pub async fn execute_with_recovery<F, Fut, T, E>(
        &mut self,
        operation: F,
    ) -> Result<T, HandlerError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Display + IsRetryable,
    {
        match with_retry(&self.retry_policy, operation).await {
            Ok(val) => {
                self.connectivity.on_success();
                Ok(val)
            }
            Err(e) => {
                self.connectivity.on_failure();
                Err(HandlerError::RetryExhausted(e.to_string()))
            }
        }
    }
}
