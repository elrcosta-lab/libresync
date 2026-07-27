use std::sync::{Arc, Mutex};

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
use libresync_core::ui::state::{AccountInfo, AppUiState, SyncActivity};
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
    pub engine: Arc<Mutex<Option<SyncEngine>>>,
    pub ui_state: Mutex<AppUiState>,
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

    open::that(&auth_url).map_err(|_| "Não foi possível abrir o navegador.".to_string())?;

    let cb = server.wait_for_callback(&session.state).await
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

pub fn run_tray(engine: SyncEngine, ui_state: AppUiState) {
    let engine = Arc::new(Mutex::new(Some(engine)));
    let state = AppState {
        engine: engine.clone(),
        ui_state: Mutex::new(ui_state),
    };

    // Background sync loop
    let eng = engine.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                if let Ok(mut e) = eng.lock() {
                    if let Some(ref mut engine) = *e {
                        let _ = engine.detect_changes().await;
                        let _ = engine.process_queue().await;
                    }
                }
            }
        });
    });

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
        ])
        .setup(|app: &mut tauri::App<Wry>| {
            let handle = app.handle();
            let tray = build_tray(handle)?;
            let app_state = handle.state::<AppState>();
            let ui = app_state.ui_state.lock().unwrap();
            update_tray(&tray, &ui);
            drop(ui);
            handle.manage(TrayHolder(Mutex::new(Some(tray))));
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
    let pause = MenuItemBuilder::with_id("pause", "Pause Sync").build(app)?;
    let preferences = MenuItemBuilder::with_id("preferences", "Preferences").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit LibreSync").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&login)
        .item(&config_id)
        .separator()
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
                    let _ = std::thread::spawn(move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        rt.block_on(async {
                            let cid = get_client_id(&handle).await;
                            if cid.is_empty() { return; }
                            let eng = {
                                let state = handle.state::<AppState>();
                                state.engine.clone()
                            };
                            do_oauth_flow(&cid, &eng).await.ok();
                        });
                    });
                }
                "config_id" => {
                    if let Ok(input) = std::process::Command::new("zenity")
                        .args(&["--entry", "--title=LibreSync", "--text=Cole seu Google Client ID:", "--width=500"])
                        .output()
                    {
                        let cid = String::from_utf8_lossy(&input.stdout).trim().to_string();
                        if !cid.is_empty() {
                            let mut cfg = libresync_core::config::LibreSyncConfig::load().unwrap_or_default();
                            cfg.google.client_id = cid;
                            cfg.save().ok();
                        }
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
    if let Ok(input) = std::process::Command::new("zenity")
        .args(&["--entry", "--title=LibreSync", "--text=Cole seu Google Client ID:", "--width=500"])
        .output()
    {
        let cid = String::from_utf8_lossy(&input.stdout).trim().to_string();
        if !cid.is_empty() {
            let mut cfg = libresync_core::config::LibreSyncConfig::load().unwrap_or_default();
            cfg.google.client_id = cid.clone();
            cfg.save().ok();
            return cid;
        }
    }
    let _ = notify_rust::Notification::new()
        .summary("LibreSync")
        .body("GOOGLE_CLIENT_ID não configurado.")
        .icon("dialog-error")
        .timeout(notify_rust::Timeout::Milliseconds(5000))
        .show();
    String::new()
}

async fn do_oauth_flow(client_id: &str, engine: &Arc<Mutex<Option<SyncEngine>>>) -> Result<(), String> {
    use libresync_core::auth::provider::GoogleAuthProvider;
    use libresync_core::auth::server::CallbackServer;
    use libresync_core::auth::session::PkceSession;
    use libresync_core::config::LibreSyncConfig;
    use libresync_core::drive::client::DriveApiClient;
    use libresync_core::drive::DriveApi;
    use libresync_core::sync::config::SyncConfig;
    use libresync_core::sync::engine::SyncEngine;
    use std::sync::Arc;

    let session = PkceSession::new(client_id);
    let redirect_uri = "http://localhost:65432/callback";
    let auth_url = session.authorization_url(redirect_uri);
    let server = CallbackServer::new().with_timeout(std::time::Duration::from_secs(300));

    open::that(&auth_url).map_err(|_| "Erro ao abrir navegador".to_string())?;

    let cb = server.wait_for_callback(&session.state).await
        .map_err(|e| format!("Callback: {}", e))?;

    let provider = GoogleAuthProvider::new();
    let token = provider.exchange_code(client_id, &cb.code, &session.code_verifier, redirect_uri)
        .await.map_err(|e| format!("Token: {}", e))?;

    let rt = token.refresh_token.unwrap_or_default();

    // Save tokens and recreate engine with real credentials
    let mut cfg = LibreSyncConfig::load().unwrap_or_default();
    cfg.google.client_id = client_id.to_string();
    cfg.google.refresh_token = Some(rt.clone());
    cfg.save().ok();

    let auth = Arc::new(GoogleAuthProvider::new());
    let drive_api: Arc<dyn DriveApi> = Arc::new(DriveApiClient::new(auth, client_id, &rt));
    let sync_config = SyncConfig::default();
    let sync_dir = cfg.sync.local_dir.to_string_lossy().to_string();
    let db = libresync_core::db::Database::open_default().ok().map(|d| Arc::new(d));
    let new_engine = SyncEngine::new(drive_api, sync_config, &sync_dir, db);

    // Replace engine in global state with real credentials
    let mut eng = engine.lock().unwrap();
    *eng = Some(new_engine);
    drop(eng);

    let _ = notify_rust::Notification::new()
        .summary("LibreSync")
        .body("Autenticação concluída! Sincronização iniciada.")
        .icon("dialog-information")
        .timeout(notify_rust::Timeout::Milliseconds(5000))
        .show();
    Ok(())
}
