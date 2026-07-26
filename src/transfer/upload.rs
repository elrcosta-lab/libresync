use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, oneshot, Mutex, RwLock};

use crate::transfer::config::TransferConfig;
use crate::transfer::token_bucket::TokenBucket;

pub type ProgressFn = Arc<dyn Fn(String, u64, u64) + Send + Sync>;

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum UJobState {
    Queued,
    Running,
    Completed,
    Failed(String),
    Cancelled,
}

#[derive(Clone)]
#[allow(dead_code)]
struct UJob {
    job_id: String,
    file_path: String,
    remote_parent_id: String,
    state: Arc<RwLock<UJobState>>,
    callback: Option<ProgressFn>,
    retry_count: Arc<RwLock<u32>>,
}

#[allow(dead_code)]
enum UCmd {
    Enqueue {
        file_path: String,
        remote_parent_id: String,
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

pub struct UploadManager {
    tx: mpsc::UnboundedSender<UCmd>,
    jobs: Arc<RwLock<HashMap<String, UJob>>>,
    paused: Arc<AtomicBool>,
    pub(crate) active_count: Arc<AtomicUsize>,
    fail_count: Arc<Mutex<u32>>,
    bucket: TokenBucket,
}

impl Clone for UploadManager {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            jobs: self.jobs.clone(),
            paused: self.paused.clone(),
            active_count: self.active_count.clone(),
            fail_count: self.fail_count.clone(),
            bucket: self.bucket.clone(),
        }
    }
}

impl UploadManager {
    pub fn new(config: TransferConfig) -> Self {
        Self::new_with_params(config, 0)
    }

    pub fn new_with_fail_count(config: TransferConfig, fail_count: u32) -> Self {
        Self::new_with_params(config, fail_count)
    }

    fn new_with_params(config: TransferConfig, fail_count: u32) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let jobs = Arc::new(RwLock::new(HashMap::new()));
        let paused = Arc::new(AtomicBool::new(false));
        let active_count = Arc::new(AtomicUsize::new(0));
        let fail_cnt = Arc::new(Mutex::new(fail_count));

        let bucket_rate = config.bandwidth_upload_kbps.unwrap_or(0);
        let bucket = TokenBucket::new(bucket_rate);

        let coordinator = UploadCoordinator {
            config: config.clone(),
            jobs: jobs.clone(),
            paused: paused.clone(),
            active_count: active_count.clone(),
            fail_count: fail_cnt.clone(),
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
            bucket,
        }
    }

    pub async fn enqueue(
        &self,
        file_path: &str,
        remote_parent_id: &str,
        callback: Option<ProgressFn>,
    ) -> Option<String> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(UCmd::Enqueue {
                file_path: file_path.to_string(),
                remote_parent_id: remote_parent_id.to_string(),
                callback,
                resp: resp_tx,
            })
            .ok()?;
        resp_rx.await.ok()
    }

    pub async fn cancel(&self, job_id: &str) {
        let _ = self.tx.send(UCmd::Cancel {
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
            .send(UCmd::Status {
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

struct UploadCoordinator {
    config: TransferConfig,
    jobs: Arc<RwLock<HashMap<String, UJob>>>,
    paused: Arc<AtomicBool>,
    active_count: Arc<AtomicUsize>,
    fail_count: Arc<Mutex<u32>>,
    bucket: TokenBucket,
    tx: mpsc::UnboundedSender<UCmd>,
    rx: mpsc::UnboundedReceiver<UCmd>,
    queue: VecDeque<String>,
}

impl UploadCoordinator {
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

    async fn handle_msg(&mut self, msg: UCmd) {
        match msg {
            UCmd::Enqueue {
                file_path,
                remote_parent_id,
                callback,
                resp,
            } => {
                let job_id = uuid::Uuid::new_v4().to_string();
                let job = UJob {
                    job_id: job_id.clone(),
                    file_path,
                    remote_parent_id,
                    state: Arc::new(RwLock::new(UJobState::Queued)),
                    callback,
                    retry_count: Arc::new(RwLock::new(0)),
                };
                self.jobs.write().await.insert(job_id.clone(), job);
                self.queue.push_back(job_id.clone());
                let _ = resp.send(job_id);
            }
            UCmd::Cancel { job_id } => {
                if let Some(job) = self.jobs.read().await.get(&job_id) {
                    let mut state = job.state.write().await;
                    if matches!(
                        *state,
                        UJobState::Queued | UJobState::Running
                    ) {
                        *state = UJobState::Cancelled;
                    }
                }
                self.queue.retain(|id| id != &job_id);
            }
            UCmd::Status { job_id, resp } => {
                let status = if let Some(job) = self.jobs.read().await.get(&job_id) {
                    match &*job.state.read().await {
                        UJobState::Queued => "queued",
                        UJobState::Running => "running",
                        UJobState::Completed => "completed",
                        UJobState::Failed(_) => "failed",
                        UJobState::Cancelled => "cancelled",
                    }
                    .to_string()
                } else {
                    "not_found".to_string()
                };
                let _ = resp.send(status);
            }
            UCmd::WorkerDone { .. } => {
                self.active_count.fetch_sub(1, Ordering::SeqCst);
            }
        }
    }

    async fn dispatch_workers(&mut self) {
        while !self.paused.load(Ordering::SeqCst)
            && self.active_count.load(Ordering::SeqCst) < self.config.max_parallel_uploads as usize
            && !self.queue.is_empty()
        {
            let job_id = self.queue.pop_front().unwrap();
            let should_skip = {
                let jobs = self.jobs.read().await;
                if let Some(job) = jobs.get(&job_id) {
                    let state = job.state.read().await;
                    matches!(*state, UJobState::Cancelled)
                } else {
                    true
                }
            };
            if should_skip {
                continue;
            }

            if let Some(job) = self.jobs.read().await.get(&job_id) {
                let mut state = job.state.write().await;
                *state = UJobState::Running;
            }

            self.active_count.fetch_add(1, Ordering::SeqCst);

            let config = self.config.clone();
            let jobs = self.jobs.clone();
            let paused = self.paused.clone();
            let fail_count = self.fail_count.clone();
            let bucket = self.bucket.clone();
            let tx = self.tx.clone();
            let jid = job_id.clone();

            tokio::spawn(async move {
                upload_worker(config, jobs, paused, fail_count, bucket, tx, jid).await;
            });
        }
    }
}

async fn upload_worker(
    config: TransferConfig,
    jobs: Arc<RwLock<HashMap<String, UJob>>>,
    paused: Arc<AtomicBool>,
    fail_count: Arc<Mutex<u32>>,
    bucket: TokenBucket,
    tx: mpsc::UnboundedSender<UCmd>,
    job_id: String,
) {
    let max_retries = 5;

    for attempt in 0..max_retries {
        if is_cancelled(&jobs, &job_id).await {
            break;
        }

        wait_if_paused(&jobs, &job_id, &paused).await;

        if is_cancelled(&jobs, &job_id).await {
            break;
        }

        let file_size: u64 = 1024 * 1024; // Simulated 1MB
        let chunk_size = config.chunk_size;

        if file_size < chunk_size {
            if let Some(kbps) = config.bandwidth_upload_kbps {
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
            #[allow(clippy::manual_div_ceil)]
            let num_chunks = file_size.div_ceil(chunk_size);
            for i in 0..num_chunks {
                if is_cancelled(&jobs, &job_id).await {
                    return;
                }

                wait_if_paused(&jobs, &job_id, &paused).await;

                let offset = i * chunk_size;
                let this_chunk = std::cmp::min(chunk_size, file_size - offset);

                if let Some(kbps) = config.bandwidth_upload_kbps {
                    if kbps > 0 {
                        let tokens = this_chunk / 128;
                        bucket.consume(tokens).await;
                    }
                }

                tokio::time::sleep(Duration::from_millis(30)).await;

                if let Some(job) = jobs.read().await.get(&job_id) {
                    if let Some(ref cb) = job.callback {
                        cb(job_id.clone(), offset + this_chunk, file_size);
                    }
                }
            }
        }

        if is_cancelled(&jobs, &job_id).await {
            break;
        }

        {
            let mut fc = fail_count.lock().await;
            if *fc > 0 {
                *fc -= 1;
                let backoff_secs = 1u64 << attempt.min(5);
                tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
                continue;
            }
        }

        mark_completed(&jobs, &job_id).await;
        let _ = tx.send(UCmd::WorkerDone {
            job_id: job_id.clone(),
        });
        return;
    }

    if !is_cancelled(&jobs, &job_id).await {
        mark_failed(&jobs, &job_id, "max retries exceeded").await;
    }
    let _ = tx.send(UCmd::WorkerDone { job_id });
}

async fn is_cancelled(jobs: &Arc<RwLock<HashMap<String, UJob>>>, job_id: &str) -> bool {
    if let Some(job) = jobs.read().await.get(job_id) {
        matches!(*job.state.read().await, UJobState::Cancelled)
    } else {
        true
    }
}

async fn wait_if_paused(
    jobs: &Arc<RwLock<HashMap<String, UJob>>>,
    job_id: &str,
    paused: &AtomicBool,
) {
    while paused.load(Ordering::SeqCst) {
        tokio::time::sleep(Duration::from_millis(50)).await;
        if is_cancelled(jobs, job_id).await {
            break;
        }
    }
}

async fn mark_completed(jobs: &Arc<RwLock<HashMap<String, UJob>>>, job_id: &str) {
    if let Some(job) = jobs.read().await.get(job_id) {
        let mut state = job.state.write().await;
        if matches!(*state, UJobState::Running) {
            *state = UJobState::Completed;
        }
    }
}

async fn mark_failed(jobs: &Arc<RwLock<HashMap<String, UJob>>>, job_id: &str, reason: &str) {
    if let Some(job) = jobs.read().await.get(job_id) {
        let mut state = job.state.write().await;
        if matches!(*state, UJobState::Running | UJobState::Queued) {
            *state = UJobState::Failed(reason.to_string());
        }
    }
}
