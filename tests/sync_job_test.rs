use libresync_core::sync::error::SyncError;
use libresync_core::sync::job::{JobQueue, JobState, JobType, SyncJob};

fn make_job(id: &str, priority: u8, job_type: JobType) -> SyncJob {
    let mut job = SyncJob::new(id, job_type).with_priority(priority);
    job.id = id.to_string();
    job
}

#[test]
fn test_enqueue_and_dequeue_respects_priority() {
    let mut queue = JobQueue::new();
    queue.enqueue(make_job("low", 5, JobType::Upload));
    queue.enqueue(make_job("high", 20, JobType::Upload));
    queue.enqueue(make_job("mid", 10, JobType::Upload));

    assert_eq!(queue.dequeue().unwrap().id, "high");
    assert_eq!(queue.dequeue().unwrap().id, "mid");
    assert_eq!(queue.dequeue().unwrap().id, "low");
}

#[test]
fn test_highest_priority_comes_first() {
    let mut queue = JobQueue::new();
    queue.enqueue(make_job("a", 0, JobType::Upload));
    queue.enqueue(make_job("b", 20, JobType::Download));

    let first = queue.dequeue().unwrap();
    assert_eq!(first.id, "b");
    assert_eq!(first.priority, 20);
}

#[test]
fn test_cancel_job_changes_state() {
    let mut queue = JobQueue::new();
    let job = make_job("to-cancel", 10, JobType::Upload);
    let id = job.id.clone();
    queue.enqueue(job);

    queue.cancel(&id).unwrap();
    let jobs = queue.get_by_state(JobState::Cancelled);
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].id, id);
}

#[test]
fn test_cancel_nonexistent_job_returns_error() {
    let mut queue = JobQueue::new();
    let result = queue.cancel("nonexistent");
    assert!(result.is_err());
    match result {
        Err(SyncError::JobNotFound) => {}
        _ => panic!("expected JobNotFound"),
    }
}

#[test]
fn test_retry_increments_counter() {
    let mut queue = JobQueue::new();
    let job = make_job("retry-me", 10, JobType::Upload);
    let id = job.id.clone();
    queue.enqueue(job);

    queue.retry(&id).unwrap();
    let jobs = queue.get_by_state(JobState::Queued);
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].retry_count, 1);
}

#[test]
fn test_empty_queue_returns_none() {
    let mut queue = JobQueue::new();
    assert!(queue.dequeue().is_none());
    assert!(queue.peek().is_none());
}

#[test]
fn test_peek_does_not_remove_job() {
    let mut queue = JobQueue::new();
    queue.enqueue(make_job("peek-test", 15, JobType::Upload));

    let peeked = queue.peek();
    assert!(peeked.is_some());
    assert_eq!(peeked.unwrap().id, "peek-test");
    assert_eq!(queue.len(), 1);
}

#[test]
fn test_cancelled_jobs_not_dequeued() {
    let mut queue = JobQueue::new();
    queue.enqueue(make_job("cancel-me", 20, JobType::Upload));
    queue.enqueue(make_job("normal", 10, JobType::Upload));

    queue.cancel("cancel-me").unwrap();
    let dequeued = queue.dequeue().unwrap();
    assert_eq!(dequeued.id, "normal");
}

#[test]
fn test_get_by_state_filters_correctly() {
    let mut queue = JobQueue::new();
    queue.enqueue(make_job("j1", 10, JobType::Upload));
    queue.enqueue(make_job("j2", 10, JobType::Download));
    queue.enqueue(make_job("j3", 10, JobType::Delete));

    let queued = queue.get_by_state(JobState::Queued);
    assert_eq!(queued.len(), 3);

    let dequeued = queue.dequeue().unwrap();
    assert_eq!(dequeued.state, JobState::Running);
    assert_eq!(dequeued.id, "j3");

    let queued_after = queue.get_by_state(JobState::Queued);
    assert_eq!(queued_after.len(), 2);
}

#[test]
fn test_len_returns_total_jobs() {
    let mut queue = JobQueue::new();
    assert_eq!(queue.len(), 0);
    queue.enqueue(make_job("a", 10, JobType::Upload));
    queue.enqueue(make_job("b", 10, JobType::Download));
    assert_eq!(queue.len(), 2);
}

#[test]
fn test_mixed_priority_ordering() {
    let mut queue = JobQueue::new();
    queue.enqueue(make_job("p10a", 10, JobType::Upload));
    queue.enqueue(make_job("p20", 20, JobType::Upload));
    queue.enqueue(make_job("p10b", 10, JobType::Upload));
    queue.enqueue(make_job("p0", 0, JobType::Upload));

    assert_eq!(queue.dequeue().unwrap().id, "p20");
    let second = queue.dequeue().unwrap();
    assert!(second.id == "p10a" || second.id == "p10b");
    let third = queue.dequeue().unwrap();
    assert!(third.id == "p10a" || third.id == "p10b");
    assert_ne!(second.id, third.id);
    assert_eq!(queue.dequeue().unwrap().id, "p0");
}
