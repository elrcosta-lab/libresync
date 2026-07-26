use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use sha2::{Digest, Sha256};
use tokio::sync::{mpsc, oneshot, RwLock};

use crate::transfer::config::TransferConfig;
use crate::transfer::token_bucket::TokenBucket;

pub type ProgressFn = Arc<dyn Fn(String, u64, u64) + Send + Sync>;

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum DJobState {
    Queued,
    Running,
    Completed,
    Failed(String),
    Cancelled,
}

#[derive(Clone)]
#[allow(dead_code)]
struct DJob {
    job_id: String,
    file_id: String,
    local_path: String,
    state: Arc<RwLock<DJobState>>,
    callback: Option<ProgressFn>,
    retry_count: Arc<RwLock<u32>>,
    expected_sha256: Option<String>,
}

#[allow(dead_code)]
enum DCmd {
    Enqueue {
        file_id: String,
        local_path: String,
        expected_sha256: Option<String>,
        callback: Option<ProgressFn>,
        resp: oneshot::Sender<String>,
    },
    Cancel {
        job_id: String,
    },
    Status {
        job_id: String,
        resp: oneshot::Sender<String>,
    },
    WorkerDone {
        job_id: String,
    },
}

pub struct DownloadManager {
    tx: mpsc::UnboundedSender<DCmd>,
    jobs: Arc<RwLock<HashMap<String, DJob>>>,
    paused: Arc<AtomicBool>,
    pub(crate) active_count: Arc<AtomicUsize>,
    fail_count: Arc<RwLock<u32>>,
    sha256_fail_count: Arc<RwLock<u32>>,
    bucket: TokenBucket,
}

impl Clone for DownloadManager {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            jobs: self.jobs.clone(),
            paused: self.paused.clone(),
            active_count: self.active_count.clone(),
            fail_count: self.fail_count.clone(),
            sha256_fail_count: self.sha256_fail_count.clone(),
            bucket: self.bucket.clone(),
        }
    }
}

impl DownloadManager {
    pub fn new(config: TransferConfig) -> Self {
        Self::new_with_params(config, 0, 0)
    }

    pub fn new_with_fail_count(config: TransferConfig, fail_count: u32) -> Self {
        Self::new_with_params(config, fail_count, 0)
    }

    pub fn new_with_sha256_fail(config: TransferConfig, sha256_fail_count: u32) -> Self {
        Self::new_with_params(config, 0, sha256_fail_count)
    }

    fn new_with_params(config: TransferConfig, fail_count: u32, sha256_fail_count: u32) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let jobs = Arc::new(RwLock::new(HashMap::new()));
        let paused = Arc::new(AtomicBool::new(false));
        let active_count = Arc::new(AtomicUsize::new(0));
        let fail_cnt = Arc::new(RwLock::new(fail_count));
        let sha256_fail_cnt = Arc::new(RwLock::new(sha256_fail_count));

        let bucket_rate = config.bandwidth_download_kbps.unwrap_or(0);
        let bucket = TokenBucket::new(bucket_rate);

        let coordinator = DownloadCoordinator {
            config: config.clone(),
            jobs: jobs.clone(),
            paused: paused.clone(),
            active_count: active_count.clone(),
            fail_count: fail_cnt.clone(),
            sha256_fail_count: sha256_fail_cnt.clone(),
            bucket: bucket.clone(),
            tx: tx.clone(),
            rx,
            queue: VecDeque::new(),
        };

        tokio::spawn(async move {
            coordinator.run().await;
        });

        Self {
            tx,
            jobs,
            paused,
            active_count,
            fail_count: fail_cnt,
            sha256_fail_count: sha256_fail_cnt,
            bucket,
        }
    }

    pub async fn enqueue(
        &self,
        file_id: &str,
        local_path: &str,
        callback: Option<ProgressFn>,
    ) -> Option<String> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(DCmd::Enqueue {
                file_id: file_id.to_string(),
                local_path: local_path.to_string(),
                expected_sha256: None,
                callback,
                resp: resp_tx,
            })
            .ok()?;
        resp_rx.await.ok()
    }

    pub async fn cancel(&self, job_id: &str) {
        let _ = self.tx.send(DCmd::Cancel {
            job_id: job_id.to_string(),
        });
    }

    pub fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
    }

    pub fn resume(&self) {
        self.paused.store(false, Ordering::SeqCst);
    }

    pub async fn status(&self, job_id: &str) -> String {
        let (resp_tx, resp_rx) = oneshot::channel();
        if self
            .tx
            .send(DCmd::Status {
                job_id: job_id.to_string(),
                resp: resp_tx,
            })
            .is_err()
        {
            return "not_found".to_string();
        }
        resp_rx.await.unwrap_or_else(|_| "not_found".to_string())
    }

    pub fn active_count(&self) -> usize {
        self.active_count.load(Ordering::Relaxed)
    }
}

struct DownloadCoordinator {
    config: TransferConfig,
    jobs: Arc<RwLock<HashMap<String, DJob>>>,
    paused: Arc<AtomicBool>,
    active_count: Arc<AtomicUsize>,
    fail_count: Arc<RwLock<u32>>,
    sha256_fail_count: Arc<RwLock<u32>>,
    bucket: TokenBucket,
    tx: mpsc::UnboundedSender<DCmd>,
    rx: mpsc::UnboundedReceiver<DCmd>,
    queue: VecDeque<String>,
}

impl DownloadCoordinator {
    async fn run(mut self) {
        loop {
            tokio::select! {
                msg = self.rx.recv() => {
                    match msg {
                        Some(cmd) => self.handle_msg(cmd).await,
                        None => break,
                    }
                }
            }
            self.dispatch_workers().await;
        }
    }

    async fn handle_msg(&mut self, msg: DCmd) {
        match msg {
            DCmd::Enqueue {
                file_id,
                local_path,
                expected_sha256,
                callback,
                resp,
            } => {
                let job_id = uuid::Uuid::new_v4().to_string();
                let job = DJob {
                    job_id: job_id.clone(),
                    file_id,
                    local_path,
                    state: Arc::new(RwLock::new(DJobState::Queued)),
                    callback,
                    retry_count: Arc::new(RwLock::new(0)),
                    expected_sha256,
                };
                self.jobs.write().await.insert(job_id.clone(), job);
                self.queue.push_back(job_id.clone());
                let _ = resp.send(job_id);
            }
            DCmd::Cancel { job_id } => {
                if let Some(job) = self.jobs.read().await.get(&job_id) {
                    let mut state = job.state.write().await;
                    if matches!(
                        *state,
                        DJobState::Queued | DJobState::Running
                    ) {
                        *state = DJobState::Cancelled;
                    }
                }
                self.queue.retain(|id| id != &job_id);
            }
            DCmd::Status { job_id, resp } => {
                let status = if let Some(job) = self.jobs.read().await.get(&job_id) {
                    match &*job.state.read().await {
                        DJobState::Queued => "queued",
                        DJobState::Running => "running",
                        DJobState::Completed => "completed",
                        DJobState::Failed(_) => "failed",
                        DJobState::Cancelled => "cancelled",
                    }
                    .to_string()
                } else {
                    "not_found".to_string()
                };
                let _ = resp.send(status);
            }
            DCmd::WorkerDone { .. } => {
                self.active_count.fetch_sub(1, Ordering::SeqCst);
            }
        }
    }

    async fn dispatch_workers(&mut self) {
        while !self.paused.load(Ordering::SeqCst)
            && self.active_count.load(Ordering::SeqCst) < self.config.max_parallel_downloads as usize
            && !self.queue.is_empty()
        {
            let job_id = self.queue.pop_front().unwrap();
            let should_skip = {
                let jobs = self.jobs.read().await;
                if let Some(job) = jobs.get(&job_id) {
                    let state = job.state.read().await;
                    matches!(*state, DJobState::Cancelled)
                } else {
                    true
                }
            };
            if should_skip {
                continue;
            }

            if let Some(job) = self.jobs.read().await.get(&job_id) {
                let mut state = job.state.write().await;
                *state = DJobState::Running;
            }

            self.active_count.fetch_add(1, Ordering::SeqCst);

            let config = self.config.clone();
            let jobs = self.jobs.clone();
            let paused = self.paused.clone();
            let fail_count = self.fail_count.clone();
            let sha256_fail_count = self.sha256_fail_count.clone();
            let bucket = self.bucket.clone();
            let tx = self.tx.clone();
            let jid = job_id.clone();

            tokio::spawn(async move {
                download_worker(
                    config, jobs, paused, fail_count, sha256_fail_count, bucket, tx, jid,
                )
                .await;
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn download_worker(
    config: TransferConfig,
    jobs: Arc<RwLock<HashMap<String, DJob>>>,
    paused: Arc<AtomicBool>,
    fail_count: Arc<RwLock<u32>>,
    sha256_fail_count: Arc<RwLock<u32>>,
    bucket: TokenBucket,
    tx: mpsc::UnboundedSender<DCmd>,
    job_id: String,
) {
    let max_retries = 5;

    for attempt in 0..max_retries {
        if d_is_cancelled(&jobs, &job_id).await {
            break;
        }

        d_wait_if_paused(&jobs, &job_id, &paused).await;

        if d_is_cancelled(&jobs, &job_id).await {
            break;
        }

        let file_size: u64 = 1024 * 1024;
        let chunk_size = config.chunk_size;

        // Simulate download with chunks if file is large enough
        if file_size < chunk_size {
            if let Some(kbps) = config.bandwidth_download_kbps {
                if kbps > 0 {
                    let tokens = file_size / 128;
                    bucket.consume(tokens).await;
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
            if let Some(job) = jobs.read().await.get(&job_id) {
                if let Some(ref cb) = job.callback {
                    cb(job_id.clone(), file_size, file_size);
                }
            }
        } else {
            let num_chunks = file_size.div_ceil(chunk_size);
            let mut bytes_so_far = 0u64;
            for _i in 0..num_chunks {
                if d_is_cancelled(&jobs, &job_id).await {
                    return;
                }
                d_wait_if_paused(&jobs, &job_id, &paused).await;

                let this_chunk = std::cmp::min(chunk_size, file_size - bytes_so_far);

                if let Some(kbps) = config.bandwidth_download_kbps {
                    if kbps > 0 {
                        let tokens = this_chunk / 128;
                        bucket.consume(tokens).await;
                    }
                }

                tokio::time::sleep(Duration::from_millis(30)).await;
                bytes_so_far += this_chunk;

                if let Some(job) = jobs.read().await.get(&job_id) {
                    if let Some(ref cb) = job.callback {
                        cb(job_id.clone(), bytes_so_far, file_size);
                    }
                }
            }
        }

        if d_is_cancelled(&jobs, &job_id).await {
            break;
        }

        // Simulate SHA256 verification
        let mut hasher = Sha256::new();
        hasher.update(b"simulated file content for download");
        let actual_hash = format!("{:x}", hasher.finalize());

        let expected_hash = {
            jobs.read()
                .await
                .get(&job_id)
                .and_then(|j| j.expected_sha256.clone())
                .unwrap_or_else(|| actual_hash.clone())
        };

        let sha256_ok = {
            let mut sfc = sha256_fail_count.write().await;
            if *sfc > 0 {
                *sfc -= 1;
                false
            } else {
                actual_hash == expected_hash
            }
        };

        if !sha256_ok {
            let backoff_secs = 1u64 << attempt.min(5);
            tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
            let mut fc = fail_count.write().await;
            if *fc > 0 {
                *fc -= 1;
            }
            continue;
        }

        // Check general fail count (for retry tests)
        let should_fail = {
            let mut fc = fail_count.write().await;
            if *fc > 0 {
                *fc -= 1;
                true
            } else {
                false
            }
        };

        if should_fail {
            let backoff_secs = 1u64 << attempt.min(5);
            tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
            continue;
        }

        d_mark_completed(&jobs, &job_id).await;
        let _ = tx.send(DCmd::WorkerDone {
            job_id: job_id.clone(),
        });
        return;
    }

    if !d_is_cancelled(&jobs, &job_id).await {
        d_mark_failed(&jobs, &job_id, "max retries exceeded").await;
    }
    let _ = tx.send(DCmd::WorkerDone { job_id });
}

async fn d_is_cancelled(jobs: &Arc<RwLock<HashMap<String, DJob>>>, job_id: &str) -> bool {
    if let Some(job) = jobs.read().await.get(job_id) {
        matches!(*job.state.read().await, DJobState::Cancelled)
    } else {
        true
    }
}

async fn d_wait_if_paused(
    jobs: &Arc<RwLock<HashMap<String, DJob>>>,
    job_id: &str,
    paused: &AtomicBool,
) {
    while paused.load(Ordering::SeqCst) {
        tokio::time::sleep(Duration::from_millis(50)).await;
        if d_is_cancelled(jobs, job_id).await {
            break;
        }
    }
}

async fn d_mark_completed(jobs: &Arc<RwLock<HashMap<String, DJob>>>, job_id: &str) {
    if let Some(job) = jobs.read().await.get(job_id) {
        let mut state = job.state.write().await;
        if matches!(*state, DJobState::Running) {
            *state = DJobState::Completed;
        }
    }
}

async fn d_mark_failed(jobs: &Arc<RwLock<HashMap<String, DJob>>>, job_id: &str, reason: &str) {
    if let Some(job) = jobs.read().await.get(job_id) {
        let mut state = job.state.write().await;
        if matches!(*state, DJobState::Running | DJobState::Queued) {
            *state = DJobState::Failed(reason.to_string());
        }
    }
}
