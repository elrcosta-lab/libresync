#[derive(Debug, Clone)]
pub struct SyncConfig {
    pub max_parallel_uploads: u32,
    pub max_parallel_downloads: u32,
    pub max_retries: u32,
    pub backoff_base_secs: u64,
    pub backoff_max_secs: u64,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            max_parallel_uploads: 4,
            max_parallel_downloads: 4,
            max_retries: 5,
            backoff_base_secs: 1,
            backoff_max_secs: 300,
        }
    }
}
