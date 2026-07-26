use std::sync::Arc;
use std::time::Duration;

use libresync_core::transfer::token_bucket::TokenBucket;

#[tokio::test]
async fn test_zero_rate_no_limit() {
    let bucket = TokenBucket::new(0);
    for _ in 0..100 {
        assert!(bucket.consume(1).await);
    }
}

#[tokio::test]
async fn test_normal_rate_respected() {
    let bucket = Arc::new(TokenBucket::new(1000));
    let start = tokio::time::Instant::now();
    for _ in 0..100 {
        bucket.consume(1).await;
    }
    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(90), "elapsed: {:?}", elapsed);
}

#[tokio::test]
async fn test_multiple_consumers_share_bucket() {
    let bucket = Arc::new(TokenBucket::new(2000));
    let mut handles = Vec::new();
    for _ in 0..4 {
        let b = bucket.clone();
        handles.push(tokio::spawn(async move {
            for _ in 0..50 {
                b.consume(1).await;
            }
        }));
    }
    let start = tokio::time::Instant::now();
    for h in handles {
        h.await.unwrap();
    }
    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(90), "elapsed: {:?}", elapsed);
}

#[tokio::test]
async fn test_set_rate_affects_future_consumption() {
    let bucket = Arc::new(TokenBucket::new(10000));
    // Fill bucket with tokens at high rate
    for _ in 0..50 {
        bucket.consume(1).await;
    }
    // Slow down to 100 tokens/sec; cap reduces accumulated tokens to 100
    bucket.set_rate(100).await;
    // Drain the 100 burst tokens to reach steady state
    for _ in 0..100 {
        bucket.consume(1).await;
    }
    // Now bucket is empty; measure consumption at new rate
    let start = tokio::time::Instant::now();
    for _ in 0..10 {
        bucket.consume(1).await;
    }
    let elapsed = start.elapsed();
    // 10 tokens at 100/sec should take ~100ms
    assert!(elapsed >= Duration::from_millis(80), "elapsed: {:?}", elapsed);
}

#[tokio::test]
async fn test_set_rate_to_zero_removes_limit() {
    let bucket = Arc::new(TokenBucket::new(10));
    bucket.set_rate(0).await;
    let start = tokio::time::Instant::now();
    for _ in 0..1000 {
        bucket.consume(1).await;
    }
    let elapsed = start.elapsed();
    assert!(elapsed < Duration::from_millis(100), "elapsed: {:?}", elapsed);
}
