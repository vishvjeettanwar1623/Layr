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
