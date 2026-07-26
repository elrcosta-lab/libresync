use std::sync::Mutex;

use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::{Manager, RunEvent, Runtime, Wry};

use libresync_core::sync::engine::SyncEngine;

pub struct AppState {
    #[allow(dead_code)]
    pub engine: Mutex<Option<SyncEngine>>,
    pub paused: Mutex<bool>,
}

pub fn run_tray(engine: SyncEngine) {
    let state = AppState {
        engine: Mutex::new(Some(engine)),
        paused: Mutex::new(false),
    };

    tauri::Builder::default()
        .manage(state)
        .setup(|app: &mut tauri::App<Wry>| {
            build_tray(app.handle())?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Tauri app")
        .run(|_app_handle, event| {
            if let RunEvent::ExitRequested { .. } = event {
                // Keep running in background
            }
        });
}

fn build_tray<R: Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<()> {
    let status = MenuItemBuilder::with_id("status", "Status: Idle")
        .enabled(false)
        .build(app)?;

    let pause = MenuItemBuilder::with_id("pause", "Pause Sync").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit LibreSync").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&status)
        .separator()
        .item(&pause)
        .separator()
        .item(&quit)
        .build()?;

    TrayIconBuilder::new()
        .tooltip("LibreSync")
        .menu(&menu)
        .on_menu_event(|app, event| {
            let id = event.id();
            let state = app.state::<AppState>();
            match id.as_ref() {
                "pause" => {
                    let mut paused = state.paused.lock().unwrap();
                    *paused = !*paused;
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
                let mut paused = state.paused.lock().unwrap();
                *paused = !*paused;
            }
        })
        .build(app)?;

    Ok(())
}
