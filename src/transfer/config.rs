#[derive(Debug, Clone)]
pub struct TransferConfig {
    pub max_parallel_uploads: u32,
    pub max_parallel_downloads: u32,
    pub chunk_size: u64,
    pub bandwidth_upload_kbps: Option<u64>,
    pub bandwidth_download_kbps: Option<u64>,
}

impl Default for TransferConfig {
    fn default() -> Self {
        Self {
            max_parallel_uploads: 4,
            max_parallel_downloads: 4,
            chunk_size: 5_242_880,
            bandwidth_upload_kbps: None,
            bandwidth_download_kbps: None,
        }
    }
}
