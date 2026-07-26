use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectivityState {
    Online,
    Offline,
    Degraded,
}

pub struct ConnectivityChecker {
    state: ConnectivityState,
    consecutive_failures: u32,
    threshold: u32,
    check_interval: Duration,
}

impl ConnectivityChecker {
    pub fn new(threshold: u32, check_interval: Duration) -> Self {
        Self {
            state: ConnectivityState::Online,
            consecutive_failures: 0,
            threshold,
            check_interval,
        }
    }

    pub fn state(&self) -> ConnectivityState {
        self.state.clone()
    }

    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures
    }

    pub fn threshold(&self) -> u32 {
        self.threshold
    }

    pub fn check_interval(&self) -> Duration {
        self.check_interval
    }

    pub fn on_success(&mut self) {
        self.consecutive_failures = 0;
        self.state = ConnectivityState::Online;
    }

    pub fn on_failure(&mut self) {
        self.consecutive_failures += 1;
        if self.consecutive_failures >= self.threshold {
            self.state = ConnectivityState::Offline;
        } else {
            self.state = ConnectivityState::Degraded;
        }
    }

    pub async fn check<F, Fut>(&mut self, check_fn: F) -> ConnectivityState
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<(), ()>>,
    {
        match check_fn().await {
            Ok(()) => {
                self.on_success();
            }
            Err(()) => {
                self.on_failure();
            }
        }
        self.state.clone()
    }

    pub async fn wait_for_recovery<F, Fut>(&mut self, check_fn: F)
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<(), ()>>,
    {
        loop {
            if let ConnectivityState::Online = self.check(&check_fn).await {
                return;
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }
}
