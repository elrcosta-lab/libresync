use libresync_core::instance::{InstanceError, InstanceLock};
use tempfile::NamedTempFile;

#[test]
fn test_acquire_lock_succeeds() {
    let tmp = NamedTempFile::new().unwrap();
    let lock = InstanceLock::acquire_at(tmp.path());
    assert!(lock.is_ok());
    let _lock = lock.unwrap();
}

#[test]
fn test_second_lock_fails() {
    let tmp = NamedTempFile::new().unwrap();
    let lock1 = InstanceLock::acquire_at(tmp.path()).unwrap();

    let lock2 = InstanceLock::acquire_at(tmp.path());
    assert!(lock2.is_err());

    match lock2.unwrap_err() {
        InstanceError::AlreadyRunning(pid) => {
            assert_eq!(pid, std::process::id());
        }
        other => panic!("expected AlreadyRunning, got {:?}", other),
    }

    drop(lock1);
}

#[test]
fn test_lock_released_on_drop() {
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();

    {
        let lock = InstanceLock::acquire_at(&path).unwrap();
        drop(lock);
    }

    let lock2 = InstanceLock::acquire_at(&path);
    assert!(lock2.is_ok());
}
