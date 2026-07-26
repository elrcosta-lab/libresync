#[derive(Debug, Clone)]
pub struct WatcherConfig {
    pub debounce_ms: u64,
    pub fallback_polling_interval_s: u64,
    pub max_user_watches: u64,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 500,
            fallback_polling_interval_s: 30,
            max_user_watches: 65536,
        }
    }
}
