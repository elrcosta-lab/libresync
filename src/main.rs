use std::sync::Arc;
use std::time::Duration;

use libresync_core::auth::provider::GoogleAuthProvider;
use libresync_core::config::LibreSyncConfig;
use libresync_core::drive::client::DriveApiClient;
use libresync_core::drive::DriveApi;
use libresync_core::sync::engine::SyncEngine;
use libresync_core::sync::config::SyncConfig;

#[tokio::main]
async fn main() {
    println!("LibreSync v{}", env!("CARGO_PKG_VERSION"));
    println!();

    let config = load_config();
    let sync_dir = config.sync.local_dir.clone();

    let auth = Arc::new(GoogleAuthProvider::new());
    let refresh_token = config
        .google
        .refresh_token
        .clone()
        .or_else(|| std::env::var("GOOGLE_REFRESH_TOKEN").ok());

    let client_id = if !config.google.client_id.is_empty() {
        config.google.client_id.clone()
    } else {
        std::env::var("GOOGLE_CLIENT_ID").expect("GOOGLE_CLIENT_ID required (config or env)")
    };

    let refresh = match refresh_token {
        Some(rt) => rt,
        None => {
            eprintln!("ERROR: No refresh_token configured.");
            eprintln!("Set GOOGLE_REFRESH_TOKEN env var or add it to config.toml");
            eprintln!();
            eprintln!("Run: GOOGLE_CLIENT_ID=... cargo run --bin get_refresh_token");
            std::process::exit(1);
        }
    };

    let drive_api: Arc<dyn DriveApi> = Arc::new(DriveApiClient::new(auth, &client_id, &refresh));
    let sync_config = SyncConfig {
        max_parallel_uploads: 4,
        max_parallel_downloads: 4,
        max_retries: 5,
        backoff_base_secs: 1,
        backoff_max_secs: 300,
    };
    let mut engine = SyncEngine::new(drive_api, sync_config);

    println!("Sync directory: {}", sync_dir.display());
    println!("Poll interval:  {}s", config.sync.poll_interval_secs);
    println!();

    if let Err(e) = std::fs::create_dir_all(&sync_dir) {
        eprintln!("WARNING: could not create sync dir: {}", e);
    }

    let poll = Duration::from_secs(config.sync.poll_interval_secs);

    println!("Starting sync loop... (Ctrl+C to stop)");
    println!();

    let mut tick: u64 = 0;
    loop {
        tick += 1;
        println!("[{}] Scanning...", tick);

        match engine.detect_changes().await {
            Ok(()) => {
                let qlen = engine.queue_len();
                if qlen > 0 {
                    println!("[{}] {} changes detected, processing...", tick, qlen);
                    match engine.process_queue().await {
                        Ok(()) => println!("[{}] Sync complete", tick),
                        Err(e) => eprintln!("[{}] Sync error: {}", tick, e),
                    }
                } else {
                    println!("[{}] No changes", tick);
                }
            }
            Err(e) => {
                eprintln!("[{}] Scan error: {}", tick, e);
            }
        }

        println!("[{}] Waiting {}s...", tick, poll.as_secs());
        tokio::time::sleep(poll).await;
    }
}

fn load_config() -> LibreSyncConfig {
    let path = LibreSyncConfig::config_path();
    if path.exists() {
        match LibreSyncConfig::load() {
            Ok(cfg) => {
                println!("Config loaded from: {}", path.display());
                return cfg;
            }
            Err(e) => {
                eprintln!("WARNING: config error ({}), using env vars", e);
            }
        }
    } else {
        println!("No config file at {}", path.display());
        println!("Using environment variables (GOOGLE_CLIENT_ID, GOOGLE_REFRESH_TOKEN)");
        println!();
        println!("To create a config file:");
        println!("  mkdir -p {}", path.parent().unwrap().display());
        println!("  # create {} with:", path.display());
        println!("  [google]");
        println!("  client_id = \"...\"");
        println!("  refresh_token = \"...\"");
        println!("  [sync]");
        println!("  local_dir = \"~/LibreSync\"");
        println!();
    }

    LibreSyncConfig::default()
}
