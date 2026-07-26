use std::sync::Mutex;

use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::image::Image;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::{Manager, RunEvent, Wry};

use libresync_core::sync::engine::SyncEngine;
use libresync_core::ui::config::UIConfig;
use libresync_core::ui::state::{AppUiState, SyncActivity};
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
    pub engine: Mutex<Option<SyncEngine>>,
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
    let mut ui = state.ui_state.lock().map_err(|e| e.to_string())?;
    ui.config = settings;
    Ok(true)
}

#[tauri::command]
async fn login(_app: tauri::AppHandle<Wry>) -> Result<String, String> {
    let client_id = std::env::var("GOOGLE_CLIENT_ID")
        .or_else(|_| {
            // Try to read from config file
            let config = libresync_core::config::LibreSyncConfig::load()
                .map(|c| c.google.client_id)
                .unwrap_or_default();
            if config.is_empty() {
                Err("GOOGLE_CLIENT_ID not configured")
            } else {
                Ok(config)
            }
        })
        .map_err(|_| "GOOGLE_CLIENT_ID não configurado. Edite ~/.config/libresync/config.toml ou execute: export GOOGLE_CLIENT_ID=...")?;

    let url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri=http://localhost:65432/callback&response_type=code&scope=https://www.googleapis.com/auth/drive.file&access_type=offline&prompt=consent",
        client_id
    );

    if open::that(&url).is_err() {
        return Err("Não foi possível abrir o navegador. Acesse manualmente:".to_string());
    }

    Ok(url)
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
    let state = AppState {
        engine: Mutex::new(Some(engine)),
        ui_state: Mutex::new(ui_state),
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
    let pause = MenuItemBuilder::with_id("pause", "Pause Sync").build(app)?;
    let preferences = MenuItemBuilder::with_id("preferences", "Preferences").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit LibreSync").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&pause)
        .separator()
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
