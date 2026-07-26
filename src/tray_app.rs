use std::sync::{Arc, Mutex};

use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::{Manager, RunEvent, Runtime, Wry};

use libresync_core::sync::engine::SyncEngine;
use libresync_core::ui::config::UIConfig;
use libresync_core::ui::state::{AppUiState, SyncActivity, SyncStatus};

pub struct AppState {
    #[allow(dead_code)]
    pub engine: Mutex<Option<SyncEngine>>,
    pub ui_state: Mutex<AppUiState>,
}

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
async fn get_activity(state: tauri::State<'_, AppState>, limit: usize) -> Result<Vec<SyncActivity>, String> {
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
    Ok(state.ui_state.lock().map_err(|e| e.to_string())?.config.clone())
}

#[tauri::command]
async fn update_settings(state: tauri::State<'_, AppState>, settings: UIConfig) -> Result<bool, String> {
    let mut ui = state.ui_state.lock().map_err(|e| e.to_string())?;
    ui.config = settings;
    Ok(true)
}

#[tauri::command]
async fn login(app: tauri::AppHandle<Wry>) -> Result<bool, String> {
    let url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri=http://localhost:65432/callback&response_type=code&scope=https://www.googleapis.com/auth/drive.file&access_type=offline&prompt=consent",
        std::env::var("GOOGLE_CLIENT_ID").unwrap_or_default()
    );
    let _ = open::that(&url);
    Ok(true)
}

#[tauri::command]
async fn logout(state: tauri::State<'_, AppState>, account_id: String) -> Result<bool, String> {
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
            build_tray(app.handle())?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Tauri app")
        .run(|_app_handle, event| {
            if let RunEvent::ExitRequested { .. } = event {
            }
        });
}

fn build_tray<R: Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<()> {
    let status = MenuItemBuilder::with_id("status", "Status: Idle")
        .enabled(false)
        .build(app)?;
    let pause = MenuItemBuilder::with_id("pause", "Pause Sync").build(app)?;
    let preferences = MenuItemBuilder::with_id("preferences", "Preferences").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit LibreSync").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&status)
        .separator()
        .item(&pause)
        .item(&preferences)
        .separator()
        .item(&quit)
        .build()?;

    TrayIconBuilder::new()
        .tooltip("LibreSync")
        .menu(&menu)
        .on_menu_event(|app, event| {
            let id = event.id();
            match id.as_ref() {
                "pause" => {
                    let state = app.state::<AppState>();
                    let mut ui = state.ui_state.lock().unwrap();
                    ui.toggle_pause();
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
            }
        })
        .build(app)?;

    Ok(())
}
