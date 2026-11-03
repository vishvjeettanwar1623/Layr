// Layr - Main Entry Point

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
pub mod modules;
mod window_spike;

use commands::*;
use log::{info, warn};
use modules::engine::ExecutionEngine;
use modules::shortcut::{ShortcutManager, TauriShortcutProvider};
use modules::storage::StorageManager;
use std::sync::{Arc, Mutex};
use tauri::{
    tray::TrayIconEvent, AppHandle, Manager, RunEvent, State, WebviewUrl, WebviewWindowBuilder,
    WindowEvent,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

/// Create or focus the main window.
fn show_main_window(app: &AppHandle) -> bool {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.set_focus();
        false
    } else {
        let _win = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
            .title("Layr")
            .inner_size(980.0, 680.0)
            .min_inner_size(700.0, 480.0)
            .center()
            .visible(true)
            .decorations(true)
            .build()
            .expect("failed to build main window");
        info!("Main window created");
        true
    }
}

fn main() {
    // Single-instance enforcement using Win32 named mutex
    #[cfg(windows)]
    unsafe {
        use windows::core::w;
        use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
        use windows::Win32::System::Threading::CreateMutexW;
        use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE};

        let _mutex = CreateMutexW(None, true, w!("Global\\LayrSingleInstanceMutex"));
        if GetLastError() == ERROR_ALREADY_EXISTS {
            // Find existing main window and bring to front
            if let Ok(hwnd) = FindWindowW(None, w!("Layr")) {
                if !hwnd.0.is_null() {
                    let _ = ShowWindow(hwnd, SW_RESTORE);
                    let _ = SetForegroundWindow(hwnd);
                }
            }
            return;
        }
    }

    // Rolling log file: cap at 5 MB
    if let Ok(metadata) = std::fs::metadata("layr.log") {
        if metadata.len() > 5 * 1024 * 1024 {
            let _ = std::fs::rename("layr.log", "layr.log.old");
        }
    }

    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("layr.log")
        .unwrap();

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .init();

    info!("Starting Layr...");

    let app = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_workspaces,
            get_workspace,
            create_workspace,
            update_workspace,
            duplicate_workspace,
            toggle_workspace,
            delete_workspace,
            get_presets,
            discover_apps,
            get_monitors,
            launch_app,
            launch_workspace,
            get_settings,
            update_settings,
            export_config,
            import_config,
        ])
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        let shortcut_str = shortcut.to_string();
                        info!("[Global Hotkey] Triggered: {}", shortcut_str);

                        // Check if it's the test spike hotkey
                        if shortcut_str.to_lowercase() == "control+alt+digit9"
                            || shortcut_str.to_lowercase() == "ctrl+alt+9"
                        {
                            window_spike::run_spike(app);
                            return;
                        }

                        // Look up workspace in state
                        let state: State<AppState> = app.state();
                        if let Some(sc_mgr) = &state.shortcuts {
                            if let Some(ws_id) = sc_mgr.get_workspace_for_shortcut(&shortcut_str) {
                                info!(
                                    "Executing workspace '{}' via global hotkey '{}'",
                                    ws_id, shortcut_str
                                );
                                let handle = app.clone();
                                std::thread::spawn(move || {
                                    let state: State<AppState> = handle.state();
                                    if let Err(e) = execute_workspace_internal(&handle, &state, &ws_id) {
                                        warn!("Hotkey execution failed for workspace {}: {}", ws_id, e);
                                    }
                                });
                            }
                        }
                    }
                })
                .build(),
        )
        .setup(|app| {
            let storage_dir = StorageManager::default_dir();
            let storage_manager = StorageManager::new(storage_dir);

            let provider = TauriShortcutProvider::new(app.handle().clone());
            let shortcut_manager = Arc::new(ShortcutManager::new(provider));
            let engine = ExecutionEngine::new();

            // Load stored workspaces and register enabled shortcuts
            if let Ok(data) = storage_manager.load() {
                for ws in &data.workspaces {
                    if ws.enabled {
                        if let Some(sc) = &ws.shortcut {
                            if !sc.trim().is_empty() {
                                if let Err(e) =
                                    shortcut_manager.bind_workspace_shortcut(&ws.id, sc)
                                {
                                    warn!(
                                        "Could not register shortcut '{}' for workspace '{}': {}",
                                        sc, ws.name, e
                                    );
                                }
                            }
                        }
                    }
                }
            }

            // Register global hotkey Ctrl+Alt+9 for spike testing
            if let Ok(spike_sc) = "ctrl+alt+9".parse::<Shortcut>() {
                let _ = app.global_shortcut().register(spike_sc);
            }

            // Manage AppState with shortcut_manager and engine
            app.manage(AppState {
                storage: Mutex::new(storage_manager),
                shortcuts: Some(Arc::clone(&shortcut_manager)),
                engine,
            });

            // Check if CLI flags were passed for automated testing / spikes
            let args: Vec<String> = std::env::args().collect();
            if args.iter().any(|a| a == "--spike") {
                info!("CLI flag --spike detected, running spike immediately...");
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    window_spike::run_spike(&handle);
                });
            }

            if args.iter().any(|a| a == "--test-cycle") {
                info!("CLI flag --test-cycle detected, testing 5x open/close webview lifecycle...");
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    for i in 1..=5 {
                        std::thread::sleep(std::time::Duration::from_millis(1500));
                        info!("Cycle {}: Opening main window...", i);
                        show_main_window(&handle);
                        std::thread::sleep(std::time::Duration::from_millis(2000));
                        info!("Cycle {}: Closing and destroying main window...", i);
                        if let Some(win) = handle.get_webview_window("main") {
                            let _ = win.destroy();
                        }
                    }
                    info!("5x open/close cycle complete!");
                });
            }

            // Set up tray icon event handler
            let tray = app.tray_by_id("main-tray").expect("tray not found");
            let handle = app.handle().clone();
            tray.on_tray_icon_event(move |_tray, event| {
                if let TrayIconEvent::Click { button, .. } = event {
                    if button == tauri::tray::MouseButton::Left {
                        show_main_window(&handle);
                    }
                }
            });

            info!("Layr started (tray only, no window)");
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        match &event {
            RunEvent::WindowEvent {
                label,
                event: WindowEvent::CloseRequested { .. },
                ..
            } => {
                if label == "main" {
                    // Destroy the webview window on close to free memory
                    if let Some(win) = app_handle.get_webview_window("main") {
                        info!("Destroying main window webview");
                        let _ = win.destroy();
                    }
                }
            }
            RunEvent::ExitRequested { api, .. } => {
                // Prevent the app from exiting when the last window closes
                api.prevent_exit();
            }
            _ => {}
        }
    });
}
