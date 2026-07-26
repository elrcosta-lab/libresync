use std::sync::Arc;
use std::time::Instant;

use tokio::sync::Mutex;

struct BucketState {
    rate: u64,
    tokens: f64,
    last_update: Instant,
}

pub struct TokenBucket {
    state: Arc<Mutex<BucketState>>,
}

impl TokenBucket {
    pub fn new(rate: u64) -> Self {
        Self {
            state: Arc::new(Mutex::new(BucketState {
                rate,
                tokens: 0.0,
                last_update: Instant::now(),
            })),
        }
    }

    pub async fn consume(&self, tokens: u64) -> bool {
        let mut state = self.state.lock().await;
        if state.rate == 0 {
            return true;
        }
        loop {
            let now = Instant::now();
            let elapsed = now.duration_since(state.last_update).as_secs_f64();
            let refill = elapsed * state.rate as f64;
            state.tokens = (state.tokens + refill).min(state.rate as f64);
            state.last_update = now;

            if state.tokens >= tokens as f64 {
                state.tokens -= tokens as f64;
                return true;
            }

            let deficit = tokens as f64 - state.tokens;
            let wait = deficit / state.rate as f64;
            drop(state);
            tokio::time::sleep(std::time::Duration::from_secs_f64(wait)).await;
            state = self.state.lock().await;
        }
    }

    pub async fn set_rate(&self, rate: u64) {
        let mut state = self.state.lock().await;
        state.rate = rate;
        if rate > 0 {
            state.tokens = state.tokens.min(rate as f64);
        }
    }
}

impl Clone for TokenBucket {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}
