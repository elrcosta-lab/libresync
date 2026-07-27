use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::image::Image;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::{Manager, RunEvent, Wry};

use libresync_core::auth::provider::{AuthProvider, GoogleAuthProvider};
use libresync_core::auth::server::CallbackServer;
use libresync_core::auth::session::PkceSession;
use libresync_core::keyring::storage::TokenStorage;
use libresync_core::sync::engine::SyncEngine;
use libresync_core::ui::config::UIConfig;
use libresync_core::ui::state::{AccountInfo, AppScreen, AppUiState, SyncActivity, SyncStatus};
use libresync_core::ui::tray;

const STATUS_ICONS: &[(&str, &[u8])] = &[
    ("synced", include_bytes!("../resources/icons/status/32x32/synced.png")),
    ("syncing", include_bytes!("../resources/icons/status/32x32/syncing.png")),
    ("error", include_bytes!("../resources/icons/status/32x32/error.png")),
    ("paused", include_bytes!("../resources/icons/status/32x32/paused.png")),
    ("offline", include_bytes!("../resources/icons/status/32x32/offline.png")),
];

fn icon_bytes(name: &str) -> &[u8] {
    STATUS_ICONS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, b)| *b)
        .unwrap_or(STATUS_ICONS[0].1)
}

fn update_tray(tray: &TrayIcon<Wry>, ui: &AppUiState) {
    let icon_name = tray::tray_icon_for_state(ui);
    if let Ok(img) = Image::from_bytes(icon_bytes(icon_name)) {
        let _ = tray.set_icon(Some(img));
    }
    let _ = tray.set_tooltip(Some(format!("LibreSync - {}", tray::tray_status_text(ui))));
}

pub struct AppState {
    #[allow(dead_code)]
    pub engine: Arc<tokio::sync::Mutex<Option<SyncEngine>>>,
    pub ui_state: Arc<Mutex<AppUiState>>,
}

struct TrayHolder(Mutex<Option<TrayIcon<Wry>>>);

#[tauri::command]
async fn get_state(state: tauri::State<'_, AppState>) -> Result<AppUiState, String> {
    Ok(state.ui_state.lock().map_err(|e| e.to_string())?.clone())
}

#[tauri::command]
async fn toggle_pause(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let mut ui = state.ui_state.lock().map_err(|e| e.to_string())?;
    Ok(ui.toggle_pause())
}

#[tauri::command]
async fn get_activity(
    state: tauri::State<'_, AppState>,
    limit: usize,
) -> Result<Vec<SyncActivity>, String> {
    let ui = state.ui_state.lock().map_err(|e| e.to_string())?;
    let len = ui.activity.len();
    if len == 0 || limit == 0 {
        return Ok(Vec::new());
    }
    let start = len.saturating_sub(limit);
    Ok(ui.activity[start..].to_vec())
}

#[tauri::command]
async fn get_settings(state: tauri::State<'_, AppState>) -> Result<UIConfig, String> {
    Ok(state
        .ui_state
        .lock()
        .map_err(|e| e.to_string())?
        .config
        .clone())
}

#[tauri::command]
async fn update_settings(
    state: tauri::State<'_, AppState>,
    settings: UIConfig,
) -> Result<bool, String> {
    // Save client_id to config.toml
    if !settings.client_id.is_empty() {
        let mut config = libresync_core::config::LibreSyncConfig::load()
            .unwrap_or_default();
        config.google.client_id = settings.client_id.clone();
        config.save().map_err(|e| format!("Erro ao salvar config: {}", e))?;
    }
    let mut ui = state.ui_state.lock().map_err(|e| e.to_string())?;
    ui.config = settings;
    Ok(true)
}

#[tauri::command]
async fn login(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let client_id = {
        let ui = state.ui_state.lock().map_err(|e| e.to_string())?;
        let cid = ui.config.client_id.clone();
        if !cid.is_empty() { cid } else { String::new() }
    };

    let client_id = if !client_id.is_empty() {
        client_id
    } else if let Ok(id) = std::env::var("GOOGLE_CLIENT_ID") {
        id
    } else {
        return Err("GOOGLE_CLIENT_ID não configurado. Vá em Configurações e cole seu Client ID.".to_string());
    };

    run_oauth_flow(&client_id).await?;

    let mut ui = state.ui_state.lock().map_err(|e| e.to_string())?;
    ui.add_account(AccountInfo::new("default".into(), "Google Drive".into(), "Conta Google".into()));
    Ok("Autenticação concluída!".to_string())
}

async fn run_oauth_flow(client_id: &str) -> Result<(), String> {
    let session = PkceSession::new(client_id);
    let redirect_uri = "http://localhost:65432/callback";
    let auth_url = session.authorization_url(redirect_uri);
    let server = CallbackServer::new().with_timeout(std::time::Duration::from_secs(300));

    let expected_state = session.state.clone();
    let callback_task = tauri::async_runtime::spawn(async move {
        server.wait_for_callback(&expected_state).await
    });

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Envolver open::that em spawn_blocking para não bloquear o runtime
    let url_clone = auth_url.clone();
    tokio::task::spawn_blocking(move || {
        let _ = open::that(&url_clone);
    })
    .await
    .ok();

    let cb = callback_task
        .await
        .map_err(|e| format!("Callback task: {}", e))?
        .map_err(|e| format!("Erro no callback: {}", e))?;

    let provider = GoogleAuthProvider::new();
    let token_resp = provider.exchange_code(
        client_id, &cb.code, &session.code_verifier, redirect_uri
    ).await.map_err(|e| format!("Erro na troca de código: {}", e))?;

    let refresh_token = token_resp.refresh_token
        .ok_or_else(|| "Google não retornou refresh_token.".to_string())?;

    let token_json = serde_json::json!({
        "access_token": token_resp.access_token,
        "refresh_token": refresh_token,
    }).to_string();

    let _ = TokenStorage::new().await.store("default", &token_json).await;

    let mut config = libresync_core::config::LibreSyncConfig::load().unwrap_or_default();
    config.google.refresh_token = Some(refresh_token);
    config.google.client_id = client_id.to_string();
    config.save().ok();

    Ok(())
}

#[tauri::command]
async fn logout(
    state: tauri::State<'_, AppState>,
    account_id: String,
) -> Result<bool, String> {
    let mut ui = state.ui_state.lock().map_err(|e| e.to_string())?;
    ui.remove_account(&account_id);
    Ok(true)
}

#[tauri::command]
async fn complete_welcome(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle<Wry>,
    client_id: String,
    client_secret: String,
) -> Result<bool, String> {
    let mut cfg = libresync_core::config::LibreSyncConfig::load().unwrap_or_default();
    if !client_id.is_empty() {
        cfg.google.client_id = client_id.clone();
    }
    if !client_secret.is_empty() {
        cfg.google.client_secret = Some(client_secret);
    }
    cfg.first_run = false;
    cfg.save().map_err(|e| format!("Erro ao salvar config: {}", e))?;

    // Also update UIConfig so frontend reflects saved client_id
    let mut ui = state.ui_state.lock().map_err(|e| e.to_string())?;
    if !client_id.is_empty() {
        ui.config.client_id = client_id;
    }

    // Hide the window
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.hide();
    }

    Ok(true)
}

pub fn run_tray(engine: Arc<tokio::sync::Mutex<Option<SyncEngine>>>, ui_state: Arc<Mutex<AppUiState>>) {
    let state = AppState {
        engine: engine.clone(),
        ui_state: ui_state.clone(),
    };

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            get_state,
            toggle_pause,
            get_activity,
            get_settings,
            update_settings,
            login,
            logout,
            complete_welcome,
        ])
        .setup(|app: &mut tauri::App<Wry>| {
            let handle = app.handle();
            let tray = build_tray(handle)?;
            {
                let app_state = handle.state::<AppState>();
                let ui = app_state.ui_state.lock().unwrap();
                update_tray(&tray, &ui);
            }
            handle.manage(TrayHolder(Mutex::new(Some(tray))));

            // First-run detection: open welcome screen if first_run is true
            let is_first_run = {
                let cfg = libresync_core::config::LibreSyncConfig::load().unwrap_or_default();
                cfg.first_run
            };
            if is_first_run {
                let app_state = handle.state::<AppState>();
                let mut ui = app_state.ui_state.lock().unwrap();
                ui.set_screen(AppScreen::Onboarding { step: 1 });
                drop(ui);
                if let Some(window) = handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }

            // Window close handler: hide instead of closing
            if let Some(window) = handle.get_webview_window("main") {
                let h = handle.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        if let Some(w) = h.get_webview_window("main") {
                            let _ = w.hide();
                        }
                    }
                });
            }

            // Spawn sync loop with access to app handle
            let handle_clone = handle.clone();
            tauri::async_runtime::spawn(async move {
                sync_loop(handle_clone).await;
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Tauri app")
        .run(|_app_handle, event| {
            if let RunEvent::ExitRequested { .. } = event {
            }
        });
}

fn build_tray(app: &tauri::AppHandle<Wry>) -> tauri::Result<TrayIcon<Wry>> {
    let login = MenuItemBuilder::with_id("login", "Conectar conta Google").build(app)?;
    let config_id = MenuItemBuilder::with_id("config_id", "Configurar Client ID").build(app)?;
    let config_secret = MenuItemBuilder::with_id("config_secret", "Configurar Client Secret").build(app)?;
    let welcome = MenuItemBuilder::with_id("welcome", "Boas-vindas").build(app)?;
    let pause = MenuItemBuilder::with_id("pause", "Pause Sync").build(app)?;
    let preferences = MenuItemBuilder::with_id("preferences", "Preferences").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit LibreSync").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&login)
        .item(&config_id)
        .item(&config_secret)
        .separator()
        .item(&welcome)
        .item(&pause)
        .item(&preferences)
        .separator()
        .item(&quit)
        .build()?;

    let default_icon =
        Image::from_bytes(include_bytes!("../resources/icons/status/32x32/synced.png"))
            .expect("default icon");

    let tray = TrayIconBuilder::new()
        .icon(default_icon)
        .tooltip("LibreSync")
        .menu(&menu)
        .on_menu_event(|app, event| {
            let id = event.id();
            match id.as_ref() {
                "login" => {
                    let handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let cid = get_client_id(&handle).await;
                        if cid.is_empty() { return; }
                        let eng = {
                            let state = handle.state::<AppState>();
                            state.engine.clone()
                        };
                        do_oauth_flow(&cid, &eng).await.ok();
                    });
                }
                "config_id" => {
                    tauri::async_runtime::spawn(async {
                        let input = tokio::task::spawn_blocking(|| {
                            std::process::Command::new("zenity")
                                .args(&["--entry", "--title=LibreSync", "--text=Cole seu Google Client ID:", "--width=500"])
                                .output()
                        })
                        .await
                        .ok()
                        .and_then(|r| r.ok());
                        
                        if let Some(input) = input {
                            let cid = String::from_utf8_lossy(&input.stdout).trim().to_string();
                            if !cid.is_empty() {
                                let mut cfg = libresync_core::config::LibreSyncConfig::load().unwrap_or_default();
                                cfg.google.client_id = cid;
                                cfg.save().ok();
                            }
                        }
                    });
                }
                "config_secret" => {
                    tauri::async_runtime::spawn(async {
                        let input = tokio::task::spawn_blocking(|| {
                            std::process::Command::new("zenity")
                                .args(&["--entry", "--title=LibreSync", "--text=Cole seu Google Client Secret:", "--width=500"])
                                .output()
                        })
                        .await
                        .ok()
                        .and_then(|r| r.ok());
                        
                        if let Some(input) = input {
                            let secret = String::from_utf8_lossy(&input.stdout).trim().to_string();
                            if !secret.is_empty() {
                                let mut cfg = libresync_core::config::LibreSyncConfig::load().unwrap_or_default();
                                cfg.google.client_secret = Some(secret);
                                cfg.save().ok();
                            }
                        }
                    });
                }
                "welcome" => {
                    let state = app.state::<AppState>();
                    let mut ui = state.ui_state.lock().unwrap();
                    ui.set_screen(AppScreen::Onboarding { step: 1 });
                    drop(ui);
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "pause" => {
                    let state = app.state::<AppState>();
                    let mut ui = state.ui_state.lock().unwrap();
                    ui.toggle_pause();
                    if let Some(tray) = app.state::<TrayHolder>().0.lock().unwrap().as_ref() {
                        update_tray(tray, &ui);
                    }
                }
                "preferences" => {
                    if let Some(window) = app.get_webview_window("main") {
                        window.show().ok();
                        window.set_focus().ok();
                    }
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                let state = app.state::<AppState>();
                let mut ui = state.ui_state.lock().unwrap();
                ui.toggle_pause();
                if let Some(tray) = app.state::<TrayHolder>().0.lock().unwrap().as_ref() {
                    update_tray(tray, &ui);
                }
            }
        })
        .build(app)?;

    Ok(tray)
}

async fn sync_loop(handle: tauri::AppHandle<Wry>) {
    let mut last_engine_id = 0usize;
    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;

        let (engine, ui_state) = {
            let app_state = handle.state::<AppState>();
            (app_state.engine.clone(), app_state.ui_state.clone())
        };

        let mut eng_lock = engine.lock().await;

        if let Some(ref mut engine) = *eng_lock {
            let engine_ptr = engine as *const _ as usize;
            if engine_ptr != last_engine_id {
                println!("[sync] Engine atualizado (ptr: {})", engine_ptr);
                last_engine_id = engine_ptr;
            }

            update_tray_with_status(&handle, SyncStatus::Syncing, &ui_state);

            println!("[sync] Iniciando detect_changes...");
            match engine.detect_changes().await {
                Ok(()) => {
                    let queue_len = engine.queue_len();
                    println!("[sync] detect_changes OK, {} jobs na fila", queue_len);
                    if queue_len > 0 {
                        println!("[sync] Processando fila...");
                        match engine.process_queue().await {
                            Ok(()) => {
                                println!("[sync] process_queue OK");
                                update_tray_with_status(&handle, SyncStatus::Synced, &ui_state);
                            }
                            Err(e) => {
                                eprintln!("[sync] ERRO process_queue: {}", e);
                                update_tray_with_status(
                                    &handle,
                                    SyncStatus::Error(format!("{}", e)),
                                    &ui_state,
                                );
                                let body = format!("Erro ao processar sync: {}", e);
                                let _ = tokio::task::spawn_blocking(move || {
                                    let _ = notify_rust::Notification::new()
                                        .summary("LibreSync")
                                        .body(&body)
                                        .icon("dialog-error")
                                        .timeout(notify_rust::Timeout::Milliseconds(5000))
                                        .show();
                                })
                                .await;
                            }
                        }
                    } else {
                        update_tray_with_status(&handle, SyncStatus::Synced, &ui_state);
                    }
                }
                Err(e) => {
                    let error_msg = format!("{}", e);
                    eprintln!("[sync] ERRO detect_changes: {}", error_msg);

                    if error_msg.contains("401") || error_msg.contains("Unauthorized") {
                        update_tray_with_status(
                            &handle,
                            SyncStatus::Error("Token expirado. Faça login novamente.".into()),
                            &ui_state,
                        );
                        let _ = tokio::task::spawn_blocking(|| {
                            let _ = notify_rust::Notification::new()
                                .summary("LibreSync")
                                .body("Token expirado. Faça login novamente pelo menu do tray.")
                                .icon("dialog-error")
                                .timeout(notify_rust::Timeout::Milliseconds(10000))
                                .show();
                        })
                        .await;
                    } else {
                        update_tray_with_status(
                            &handle,
                            SyncStatus::Error(format!("{}", e)),
                            &ui_state,
                        );
                        let body = format!("Erro ao detectar mudanças: {}", e);
                        let _ = tokio::task::spawn_blocking(move || {
                            let _ = notify_rust::Notification::new()
                                .summary("LibreSync")
                                .body(&body)
                                .icon("dialog-error")
                                .timeout(notify_rust::Timeout::Milliseconds(5000))
                                .show();
                        })
                        .await;
                    }
                }
            }
        }
        drop(eng_lock);
    }
}

fn update_tray_with_status(
    handle: &tauri::AppHandle<Wry>,
    status: SyncStatus,
    ui_state: &Arc<Mutex<AppUiState>>,
) {
    let mut ui = ui_state.lock().unwrap();
    ui.set_sync_status(status);
    if let Some(tray) = handle.state::<TrayHolder>().0.lock().unwrap().as_ref() {
        update_tray(tray, &ui);
    }
}

async fn get_client_id(app: &tauri::AppHandle<Wry>) -> String {
    let state = app.state::<AppState>();
    if let Ok(ui) = state.ui_state.lock() {
        if !ui.config.client_id.is_empty() {
            return ui.config.client_id.clone();
        }
    }
    if let Ok(id) = std::env::var("GOOGLE_CLIENT_ID") {
        return id;
    }
    if let Ok(cfg) = libresync_core::config::LibreSyncConfig::load() {
        if !cfg.google.client_id.is_empty() {
            return cfg.google.client_id;
        }
    }
    
    // Envolver chamada síncrona em spawn_blocking
    let input = tokio::task::spawn_blocking(|| {
        std::process::Command::new("zenity")
            .args(&["--entry", "--title=LibreSync", "--text=Cole seu Google Client ID:", "--width=500"])
            .output()
    })
    .await
    .ok()
    .and_then(|r| r.ok());
    
    if let Some(input) = input {
        let cid = String::from_utf8_lossy(&input.stdout).trim().to_string();
        if !cid.is_empty() {
            let mut cfg = libresync_core::config::LibreSyncConfig::load().unwrap_or_default();
            cfg.google.client_id = cid.clone();
            cfg.save().ok();
            return cid;
        }
    }
    let _ = tokio::task::spawn_blocking(|| {
        let _ = notify_rust::Notification::new()
            .summary("LibreSync")
            .body("GOOGLE_CLIENT_ID não configurado.")
            .icon("dialog-error")
            .timeout(notify_rust::Timeout::Milliseconds(5000))
            .show();
    })
    .await;
    String::new()
}

async fn do_oauth_flow(client_id: &str, engine: &Arc<tokio::sync::Mutex<Option<SyncEngine>>>) -> Result<(), String> {
    use libresync_core::auth::provider::GoogleAuthProvider;
    use libresync_core::auth::server::CallbackServer;
    use libresync_core::auth::session::PkceSession;
    use libresync_core::config::LibreSyncConfig;
    use libresync_core::drive::client::DriveApiClient;
    use libresync_core::drive::DriveApi;
    use libresync_core::sync::config::SyncConfig;
    use libresync_core::sync::engine::SyncEngine;
    use std::sync::Arc;

    println!("[oauth] Iniciando fluxo OAuth para client_id: {}...", 
        if client_id.len() > 10 { &client_id[..10] } else { client_id });

    let session = PkceSession::new(client_id);
    let redirect_uri = "http://localhost:65432/callback";
    let auth_url = session.authorization_url(redirect_uri);
    let server = CallbackServer::new().with_timeout(std::time::Duration::from_secs(300));

    let expected_state = session.state.clone();
    let callback_task = tauri::async_runtime::spawn(async move {
        server.wait_for_callback(&expected_state).await
    });

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Envolver open::that em spawn_blocking para não bloquear o runtime
    let url_clone = auth_url.clone();
    tokio::task::spawn_blocking(move || {
        let _ = open::that(&url_clone);
    })
    .await
    .ok();

    println!("[oauth] Aguardando callback...");

    let cb = callback_task
        .await
        .map_err(|e| format!("Callback task: {}", e))?
        .map_err(|e| format!("Callback: {}", e))?;

    println!("[oauth] Callback recebido, trocando código por token...");

    // Ler client_secret do config
    let client_secret = {
        let cfg = LibreSyncConfig::load().unwrap_or_default();
        cfg.google.client_secret.clone()
    };

    let provider = if let Some(ref secret) = client_secret {
        println!("[oauth] Usando client_secret do config");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build reqwest Client");
        GoogleAuthProvider::with_client_secret(client, secret)
    } else {
        println!("[oauth] Sem client_secret, usando PKCE puro");
        GoogleAuthProvider::new()
    };
    
    println!("[oauth] Chamando exchange_code...");
    let token = provider.exchange_code(client_id, &cb.code, &session.code_verifier, redirect_uri)
        .await.map_err(|e| {
            eprintln!("[oauth] ERRO exchange_code: {}", e);
            format!("Token: {}", e)
        })?;

    println!("[oauth] exchange_code OK, extraindo refresh_token...");
    let rt = token.refresh_token.unwrap_or_default();
    println!("[oauth] Token obtido. refresh_token: {} chars", rt.len());

    if rt.is_empty() {
        eprintln!("[oauth] ERRO: Google não retornou refresh_token");
        return Err("Google não retornou refresh_token. Verifique as permissões do OAuth.".to_string());
    }

    // Save tokens and recreate engine with real credentials
    let mut cfg = LibreSyncConfig::load().unwrap_or_default();
    cfg.google.client_id = client_id.to_string();
    cfg.google.refresh_token = Some(rt.clone());
    cfg.save().ok();
    println!("[oauth] Tokens salvos em config.toml");

    println!("[oauth] Criando novo DriveApiClient...");
    let auth = Arc::new(provider);
    let drive_api: Arc<dyn DriveApi> = Arc::new(DriveApiClient::new(auth, client_id, &rt));
    let sync_config = SyncConfig::default();
    let sync_dir = cfg.sync.local_dir.to_string_lossy().to_string();
    let db = libresync_core::db::Database::open_default().ok().map(|d| Arc::new(d));
    println!("[oauth] Criando novo SyncEngine...");
    let new_engine = SyncEngine::new(drive_api, sync_config, &sync_dir, db);
    println!("[oauth] Novo engine criado com credenciais reais");

    // Replace engine in global state with real credentials
    println!("[oauth] Substituindo engine no estado global...");
    let mut eng = engine.lock().await;
    *eng = Some(new_engine);
    drop(eng);
    println!("[oauth] Engine substituído no estado global");

    let _ = tokio::task::spawn_blocking(|| {
        let _ = notify_rust::Notification::new()
            .summary("LibreSync")
            .body("Autenticação concluída! Sincronização iniciada.")
            .icon("dialog-information")
            .timeout(notify_rust::Timeout::Milliseconds(5000))
            .show();
    })
    .await;
    Ok(())
}
