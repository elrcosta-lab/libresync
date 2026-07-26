use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use libresync_core::transfer::config::TransferConfig;
use libresync_core::transfer::upload::UploadManager;

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
async fn test_small_file_upload_completes() {
    let config = small_config();
    let manager = UploadManager::new(config);
    let completed = Arc::new(AtomicUsize::new(0));
    let c = completed.clone();

    let cb = Arc::new(move |_file_id: String, _uploaded: u64, _total: u64| {
        c.fetch_add(1, Ordering::SeqCst);
    });

    let job_id = manager
        .enqueue("/tmp/test_small.txt", "remote_parent_123", Some(cb))
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
async fn test_upload_with_retry_after_failure() {
    let config = small_config();
    let manager = UploadManager::new_with_fail_count(config, 1);
    let job_id = manager
        .enqueue("/tmp/test_retry.txt", "remote_parent_456", None)
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
async fn test_cancelled_upload_does_not_complete() {
    let config = TransferConfig {
        max_parallel_uploads: 1,
        bandwidth_upload_kbps: Some(1),
        ..small_config()
    };
    let manager = UploadManager::new(config);
    let job_id = manager
        .enqueue("/tmp/test_cancel_large.bin", "remote_parent_789", None)
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
async fn test_parallel_uploads_respect_max() {
    let config = TransferConfig {
        max_parallel_uploads: 2,
        ..small_config()
    };
    let manager = UploadManager::new(config);
    let max_seen = Arc::new(AtomicUsize::new(0));
    let ms = max_seen.clone();
    let mgr = manager.clone();

    let monitor = tokio::spawn(async move {
        for _ in 0..40 {
            let count = mgr.active_count();
            ms.fetch_max(count, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    });

    for i in 0..6 {
        manager
            .enqueue(
                &format!("/tmp/test_parallel_{}.txt", i),
                "remote_parent_parallel",
                None,
            )
            .await;
    }

    monitor.await.unwrap();
    let max = max_seen.load(Ordering::SeqCst);
    assert!(
        max <= 2,
        "max concurrent uploads was {}, expected <= 2",
        max
    );
}

#[tokio::test]
async fn test_progress_callback_called() {
    let config = TransferConfig {
        chunk_size: 512, // Force chunking for more callbacks
        ..small_config()
    };
    let manager = UploadManager::new(config);
    let call_count = Arc::new(AtomicUsize::new(0));
    let cc = call_count.clone();

    let cb = Arc::new(move |_file_id: String, _uploaded: u64, _total: u64| {
        cc.fetch_add(1, Ordering::SeqCst);
    });

    let job_id = manager
        .enqueue("/tmp/test_progress.txt", "remote_parent_progress", Some(cb))
        .await;
    assert!(job_id.is_some());

    tokio::time::sleep(Duration::from_secs(1)).await;

    let count = call_count.load(Ordering::SeqCst);
    // With chunk_size=512 and simulated 1MB file, we should have ~2048 chunks
    assert!(count > 5, "progress callback called only {} times", count);
}

#[tokio::test]
async fn test_pause_resume_workers() {
    let config = TransferConfig {
        max_parallel_uploads: 2,
        chunk_size: 65536, // 64KB chunks → 16 chunks for 1MB file, ~480ms per upload
        ..small_config()
    };
    let manager = UploadManager::new(config);

    let mut ids = Vec::new();
    for i in 0..4 {
        let id = manager
            .enqueue(
                &format!("/tmp/test_pause_{}.txt", i),
                "remote_parent_pause",
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
