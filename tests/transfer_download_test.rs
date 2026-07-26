use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use libresync_core::transfer::config::TransferConfig;
use libresync_core::transfer::download::DownloadManager;

fn small_config() -> TransferConfig {
    TransferConfig {
        max_parallel_uploads: 4,
        max_parallel_downloads: 4,
        chunk_size: 5_242_880,
        bandwidth_upload_kbps: None,
        bandwidth_download_kbps: None,
    }
}

#[tokio::test]
async fn test_download_completes() {
    let config = small_config();
    let manager = DownloadManager::new(config);
    let completed = Arc::new(AtomicUsize::new(0));
    let c = completed.clone();

    let cb = Arc::new(move |_file_id: String, _downloaded: u64, _total: u64| {
        c.fetch_add(1, Ordering::SeqCst);
    });

    let job_id = manager
        .enqueue("file_remote_123", "/tmp/test_dl_small.txt", Some(cb))
        .await;
    assert!(job_id.is_some());

    tokio::time::sleep(Duration::from_millis(500)).await;

    let job_id = job_id.unwrap();
    let status = manager.status(&job_id).await;
    assert_eq!(status, "completed", "expected completed, got: {}", status);
    assert!(
        completed.load(Ordering::SeqCst) > 0,
        "progress callback was never called"
    );
}

#[tokio::test]
async fn test_download_with_retry() {
    let config = small_config();
    let manager = DownloadManager::new_with_fail_count(config, 1);
    let job_id = manager
        .enqueue("file_remote_retry", "/tmp/test_dl_retry.txt", None)
        .await;
    assert!(job_id.is_some());

    tokio::time::sleep(Duration::from_secs(4)).await;

    let job_id = job_id.unwrap();
    let status = manager.status(&job_id).await;
    assert_eq!(
        status, "completed",
        "expected completed after retry, got: {}",
        status
    );
}

#[tokio::test]
async fn test_download_sha256_verification_failure_triggers_retry() {
    let config = small_config();
    let manager = DownloadManager::new_with_sha256_fail(config, 1);
    let job_id = manager
        .enqueue("file_sha256_fail", "/tmp/test_dl_sha256.txt", None)
        .await;
    assert!(job_id.is_some());

    tokio::time::sleep(Duration::from_secs(4)).await;

    let job_id = job_id.unwrap();
    let status = manager.status(&job_id).await;
    assert_eq!(
        status, "completed",
        "expected completed after sha256 retry, got: {}",
        status
    );
}

#[tokio::test]
async fn test_download_cancellation() {
    let config = TransferConfig {
        bandwidth_download_kbps: Some(1),
        ..small_config()
    };
    let manager = DownloadManager::new(config);
    let job_id = manager
        .enqueue("file_remote_cancel", "/tmp/test_dl_cancel.bin", None)
        .await;
    assert!(job_id.is_some());

    let job_id = job_id.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    manager.cancel(&job_id).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    let status = manager.status(&job_id).await;
    assert_eq!(status, "cancelled", "expected cancelled, got: {}", status);
}

#[tokio::test]
async fn test_download_pause_resume() {
    let config = TransferConfig {
        max_parallel_downloads: 2,
        chunk_size: 65536, // 64KB chunks for file > chunk_size
        ..small_config()
    };
    let manager = DownloadManager::new(config);

    let mut ids = Vec::new();
    for i in 0..4 {
        let id = manager
            .enqueue(
                &format!("file_remote_pause_{}", i),
                &format!("/tmp/test_dl_pause_{}.txt", i),
                None,
            )
            .await
            .unwrap();
        ids.push(id);
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    manager.pause();

    let mut statuses_before = Vec::new();
    for id in &ids {
        statuses_before.push(manager.status(id).await);
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    for (i, id) in ids.iter().enumerate() {
        let status_after = manager.status(id).await;
        if statuses_before[i] != "completed" {
            assert_ne!(
                status_after, "completed",
                "job {} completed while paused",
                i
            );
        }
    }

    manager.resume();
    tokio::time::sleep(Duration::from_secs(4)).await;

    for (i, id) in ids.iter().enumerate() {
        let status = manager.status(id).await;
        assert_eq!(
            status, "completed",
            "job {} expected completed after resume, got: {}",
            i, status
        );
    }
}

#[tokio::test]
async fn test_download_progress_callback() {
    let config = TransferConfig {
        bandwidth_download_kbps: None,
        ..small_config()
    };
    let manager = DownloadManager::new(config);
    let call_count = Arc::new(AtomicUsize::new(0));
    let cc = call_count.clone();

    let cb = Arc::new(move |_file_id: String, _downloaded: u64, _total: u64| {
        cc.fetch_add(1, Ordering::SeqCst);
    });

    let job_id = manager
        .enqueue("file_remote_progress", "/tmp/test_dl_progress.txt", Some(cb))
        .await;
    assert!(job_id.is_some());

    tokio::time::sleep(Duration::from_millis(500)).await;

    let count = call_count.load(Ordering::SeqCst);
    assert!(count > 0, "progress callback called only {} times", count);
}
