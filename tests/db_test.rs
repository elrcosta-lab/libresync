use libresync_core::auth::models::{Account, AccountStatus};
use libresync_core::db::{
    clear_completed_jobs, delete_account, delete_job, delete_sync_state, get_account,
    get_account_by_email, get_job, get_sync_state, insert_account, insert_job, list_accounts,
    list_jobs, list_sync_states, set_active_account, update_account, update_job_state,
    upsert_sync_state, Database, SyncStateEntry, DbError,
};
use libresync_core::sync::job::{JobState, JobType, SyncJob};

fn setup_db() -> Database {
    let db = Database::open(":memory:").expect("failed to create in-memory database");
    db
}

#[test]
fn test_create_tables() {
    let db = setup_db();
    let conn = db.conn();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM accounts", [], |row| row.get(0))
        .expect("accounts table should exist");
    assert_eq!(count, 0);

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM sync_state", [], |row| row.get(0))
        .expect("sync_state table should exist");
    assert_eq!(count, 0);

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM jobs", [], |row| row.get(0))
        .expect("jobs table should exist");
    assert_eq!(count, 0);
}

fn make_account(id: &str, email: &str) -> Account {
    let mut acc = Account::new(id.to_string(), email.to_string(), "Test User".into());
    acc.scope = "drive.file".into();
    acc.quota_total = Some(15 * 1024 * 1024 * 1024);
    acc.quota_used = Some(2 * 1024 * 1024 * 1024);
    acc
}

#[test]
fn test_insert_and_get_account() {
    let db = setup_db();
    let acc = make_account("a1", "user@example.com");

    insert_account(&db, &acc).expect("insert should succeed");

    let found = get_account(&db, "a1")
        .expect("get_account should succeed")
        .expect("account should exist");
    assert_eq!(found.id, "a1");
    assert_eq!(found.email, "user@example.com");
    assert_eq!(found.display_name, "Test User");
    assert!(found.is_active);
    assert_eq!(found.status, AccountStatus::Active);
    assert_eq!(found.quota_total, Some(15 * 1024 * 1024 * 1024));
}

#[test]
fn test_account_crud() {
    let db = setup_db();

    let acc = make_account("a2", "crud@example.com");
    insert_account(&db, &acc).expect("insert");

    let fetched = get_account_by_email(&db, "crud@example.com")
        .expect("get_by_email")
        .expect("should exist");
    assert_eq!(fetched.id, "a2");

    let mut updated = acc.clone();
    updated.display_name = "Updated Name".into();
    updated.status = AccountStatus::Revoked;
    updated.is_active = false;
    update_account(&db, &updated).expect("update");

    let fetched = get_account(&db, "a2")
        .expect("get")
        .expect("should exist");
    assert_eq!(fetched.display_name, "Updated Name");
    assert_eq!(fetched.status, AccountStatus::Revoked);
    assert!(!fetched.is_active);

    let acc2 = make_account("a3", "other@example.com");
    insert_account(&db, &acc2).expect("insert second");

    let all = list_accounts(&db).expect("list");
    assert_eq!(all.len(), 2);

    set_active_account(&db, "a3").expect("set active");

    let a2 = get_account(&db, "a2").expect("get a2").unwrap();
    let a3 = get_account(&db, "a3").expect("get a3").unwrap();
    assert!(!a2.is_active);
    assert!(a3.is_active);

    delete_account(&db, "a2").expect("delete");
    assert!(get_account(&db, "a2").expect("get deleted").is_none());
    assert_eq!(list_accounts(&db).expect("list after delete").len(), 1);

    let result = delete_account(&db, "nonexistent");
    assert!(result.is_err());
    match result.unwrap_err() {
        DbError::AccountNotFound(_) => {}
        other => panic!("expected AccountNotFound, got {:?}", other),
    }
}

#[test]
fn test_sync_state_upsert() {
    let db = setup_db();

    let entry = SyncStateEntry {
        path: "/home/test/file.txt".into(),
        local_modified_at: Some(1000),
        remote_modified_at: Some(2000),
        local_hash: Some("abc".into()),
        remote_hash: Some("def".into()),
        last_sync_at: 3000,
    };

    upsert_sync_state(&db, &entry).expect("upsert");

    let fetched = get_sync_state(&db, "/home/test/file.txt")
        .expect("get")
        .expect("should exist");
    assert_eq!(fetched.path, "/home/test/file.txt");
    assert_eq!(fetched.local_hash, Some("abc".into()));
    assert_eq!(fetched.last_sync_at, 3000);

    let updated = SyncStateEntry {
        local_hash: Some("xyz".into()),
        remote_hash: None,
        ..entry
    };
    upsert_sync_state(&db, &updated).expect("upsert update");

    let fetched = get_sync_state(&db, "/home/test/file.txt")
        .expect("get")
        .expect("should exist");
    assert_eq!(fetched.local_hash, Some("xyz".into()));
    assert_eq!(fetched.remote_hash, None);

    let all = list_sync_states(&db).expect("list");
    assert_eq!(all.len(), 1);

    delete_sync_state(&db, "/home/test/file.txt").expect("delete");
    assert!(get_sync_state(&db, "/home/test/file.txt")
        .expect("get after delete")
        .is_none());

    let result = delete_sync_state(&db, "/nonexistent");
    match result.unwrap_err() {
        DbError::SyncStateNotFound(_) => {}
        other => panic!("expected SyncStateNotFound, got {:?}", other),
    }
}

#[test]
fn test_job_crud() {
    let db = setup_db();

    let mut job = SyncJob::new("/path/to/file", JobType::Upload);
    job.priority = 5;
    let job_id = job.id.clone();

    insert_job(&db, &job).expect("insert");

    let fetched = get_job(&db, &job_id)
        .expect("get")
        .expect("should exist");
    assert_eq!(fetched.file_path, "/path/to/file");
    assert_eq!(fetched.job_type, JobType::Upload);
    assert_eq!(fetched.priority, 5);
    assert_eq!(fetched.state, JobState::Queued);

    update_job_state(&db, &job_id, JobState::Running, None).expect("update state");
    let fetched = get_job(&db, &job_id)
        .expect("get")
        .expect("should exist");
    assert_eq!(fetched.state, JobState::Running);

    update_job_state(&db, &job_id, JobState::Failed, Some("network error")).expect("update failed");
    let fetched = get_job(&db, &job_id)
        .expect("get")
        .expect("should exist");
    assert_eq!(fetched.state, JobState::Failed);
    assert_eq!(fetched.error_message, Some("network error".into()));

    delete_job(&db, &job_id).expect("delete");
    assert!(get_job(&db, &job_id)
        .expect("get after delete")
        .is_none());

    let result = delete_job(&db, "nonexistent");
    match result.unwrap_err() {
        DbError::JobNotFound(_) => {}
        other => panic!("expected JobNotFound, got {:?}", other),
    }
}

#[test]
fn test_list_by_state() {
    let db = setup_db();

    let mut j1 = SyncJob::new("/a", JobType::Upload);
    j1.state = JobState::Queued;
    let mut j2 = SyncJob::new("/b", JobType::Download);
    j2.state = JobState::Running;
    let mut j3 = SyncJob::new("/c", JobType::Upload);
    j3.state = JobState::Completed;
    let mut j4 = SyncJob::new("/d", JobType::Delete);
    j4.state = JobState::Queued;
    j4.priority = 20;

    insert_job(&db, &j1).expect("insert j1");
    insert_job(&db, &j2).expect("insert j2");
    insert_job(&db, &j3).expect("insert j3");
    insert_job(&db, &j4).expect("insert j4");

    let queued = list_jobs(&db, Some(JobState::Queued)).expect("list queued");
    assert_eq!(queued.len(), 2);
    assert_eq!(queued[0].priority, 20);
    assert_eq!(queued[1].priority, 10);

    let all = list_jobs(&db, None).expect("list all");
    assert_eq!(all.len(), 4);

    clear_completed_jobs(&db).expect("clear completed");
    let all = list_jobs(&db, None).expect("list after clear");
    assert_eq!(all.len(), 3);

    let completed = list_jobs(&db, Some(JobState::Completed)).expect("list completed");
    assert!(completed.is_empty());
}
