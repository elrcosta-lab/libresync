use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::mpsc;

use crate::watcher::config::WatcherConfig;
use crate::watcher::error::WatcherError;
use crate::watcher::event::FileEvent;
use crate::watcher::ignore::IgnoreRules;

#[allow(dead_code)]
pub struct FileWatcher {
    config: WatcherConfig,
    ignore_rules: IgnoreRules,
    output_tx: mpsc::UnboundedSender<FileEvent>,
    pending: Arc<Mutex<HashMap<String, FileEvent>>>,
    debounce_active: Arc<AtomicBool>,
    active: Arc<AtomicBool>,
}

impl FileWatcher {
    pub fn new(
        config: WatcherConfig,
        ignore_rules: IgnoreRules,
        output_tx: mpsc::UnboundedSender<FileEvent>,
    ) -> Self {
        Self {
            config,
            ignore_rules,
            output_tx,
            pending: Arc::new(Mutex::new(HashMap::new())),
            debounce_active: Arc::new(AtomicBool::new(false)),
            active: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn config(&self) -> &WatcherConfig {
        &self.config
    }

    pub fn watch(&self, path: &str) -> Result<(), WatcherError> {
        if !std::path::Path::new(path).exists() {
            return Err(WatcherError::PathNotFound(path.to_string()));
        }
        Ok(())
    }

    pub fn stop(&self) {
        self.active.store(false, Ordering::SeqCst);
    }

    pub fn on_event(&self, event: FileEvent) {
        if self.ignore_rules.is_ignored(event.path()) {
            return;
        }

        let mut map = self.pending.lock().unwrap();
        map.insert(event.path().to_string(), event);

        if !self.debounce_active.load(Ordering::SeqCst) {
            self.debounce_active.store(true, Ordering::SeqCst);
            drop(map);

            let pending = self.pending.clone();
            let debounce_active = self.debounce_active.clone();
            let output_tx = self.output_tx.clone();
            let debounce_ms = self.config.debounce_ms;
            let active = self.active.clone();

            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(debounce_ms)).await;

                if !active.load(Ordering::SeqCst) {
                    debounce_active.store(false, Ordering::SeqCst);
                    return;
                }

                let mut map = pending.lock().unwrap();
                let events: Vec<FileEvent> = map.drain().map(|(_, v)| v).collect();
                debounce_active.store(false, Ordering::SeqCst);
                drop(map);

                let processed = Self::detect_renames(events);

                for ev in processed {
                    let _ = output_tx.send(ev);
                }
            });
        }
    }

    fn detect_renames(events: Vec<FileEvent>) -> Vec<FileEvent> {
        let mut deletes: Vec<String> = Vec::new();
        let mut creates: Vec<String> = Vec::new();
        let mut others: Vec<FileEvent> = Vec::new();

        for ev in events {
            match &ev {
                FileEvent::Deleted(p) => deletes.push(p.clone()),
                FileEvent::Created(p) => creates.push(p.clone()),
                _ => others.push(ev),
            }
        }

        let mut used_deletes = vec![false; deletes.len()];
        let mut used_creates = vec![false; creates.len()];

        for (di, del) in deletes.iter().enumerate() {
            for (ci, cre) in creates.iter().enumerate() {
                if del != cre && !used_deletes[di] && !used_creates[ci] {
                    others.push(FileEvent::Renamed {
                        from: del.clone(),
                        to: cre.clone(),
                    });
                    used_deletes[di] = true;
                    used_creates[ci] = true;
                }
            }
        }

        for (di, del) in deletes.iter().enumerate() {
            if !used_deletes[di] {
                others.push(FileEvent::Deleted(del.clone()));
            }
        }
        for (ci, cre) in creates.iter().enumerate() {
            if !used_creates[ci] {
                others.push(FileEvent::Created(cre.clone()));
            }
        }

        others
    }
}
