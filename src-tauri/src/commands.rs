use crate::modules::discovery::{self, DiscoveredApp};
use crate::modules::engine::{ExecutionEngine, LiveEngineRunner, WorkspaceExecutionResult};
use crate::modules::process::{self, LaunchResult};
use crate::modules::shortcut::{ShortcutManager, TauriShortcutProvider};
use crate::modules::storage::StorageManager;
use crate::modules::window::{self, layout_math::MonitorWorkArea};
use crate::modules::workspace::{AppEntry, DuplicatePolicy, Workspace};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{Emitter, State};
use tauri_plugin_notification::NotificationExt;

pub struct AppState {
    pub storage: Mutex<StorageManager>,
    pub shortcuts: Option<Arc<ShortcutManager<TauriShortcutProvider>>>,
    pub engine: ExecutionEngine,
}

#[tauri::command]
pub fn get_presets() -> Vec<DiscoveredApp> {
    discovery::resolve_presets()
}

#[tauri::command]
pub fn discover_apps() -> Vec<DiscoveredApp> {
    discovery::scan_start_menu()
}

#[tauri::command]
pub fn get_monitors() -> Vec<MonitorWorkArea> {
    window::get_monitor_work_areas()
}

#[tauri::command]
pub fn launch_app(app: AppEntry) -> Result<LaunchResult, String> {
    // 1. Check reuse existing policy
    if app.duplicate_policy == DuplicatePolicy::ReuseExistingWindow {
        if let Some(existing) = window::find_existing_window(&app.executable_path) {
            if let Some(ref win_cfg) = app.window_config {
                let _ = window::position_window_by_placement(existing.hwnd, win_cfg);
            }
            return Ok(LaunchResult {
                app_id: app.id,
                name: app.name,
                pid: Some(existing.pid),
                skipped: true,
                error: None,
            });
        }
    }

    // 2. Snapshot top-level windows before spawn
    let before = window::snapshot_top_level_windows();

    // 3. Spawn app
    let result = process::spawn_app(&app).map_err(|e| e.to_string())?;

    // 4. If window_config is present and not skipped, detect and position
    if !result.skipped {
        if let Some(win_cfg) = &app.window_config {
            if let Ok(win) = window::wait_for_new_window(&before, &app.executable_path, None, Duration::from_secs(5)) {
                let _ = window::position_window_by_placement(win.hwnd, win_cfg);
            }
        }
    }

    Ok(result)
}

pub fn execute_workspace_internal(
    app: &tauri::AppHandle,
    state: &AppState,
    id: &str,
) -> Result<WorkspaceExecutionResult, String> {
    let workspace = {
        let storage = state.storage.lock().map_err(|e| e.to_string())?;
        let data = storage.load().map_err(|e| e.to_string())?;
        data.workspaces
            .into_iter()
            .find(|w| w.id == id)
            .ok_or_else(|| format!("Workspace with id '{}' not found", id))?
    };

    if !workspace.enabled {
        return Err(format!("Workspace '{}' is disabled", workspace.name));
    }

    if state.engine.is_running(&workspace.id) {
        log::warn!(
            "Workspace '{}' is already launching; ignoring request to prevent re-entrancy",
            workspace.name
        );
        return Err(format!(
            "Workspace '{}' is already launching",
            workspace.name
        ));
    }

    let _ = app
        .notification()
        .builder()
        .title("Layr")
        .body(format!("Launching workspace '{}'...", workspace.name))
        .show();

    let app_handle = app.clone();
    let runner = Arc::new(LiveEngineRunner);
    let result = state
        .engine
        .execute_workspace(
            &workspace,
            runner,
            move |ws_id, app_id, status| {
                let _ = app_handle.emit(
                    "workspace-launch-progress",
                    serde_json::json!({
                        "workspace_id": ws_id,
                        "app_id": app_id,
                        "status": status,
                    }),
                );
            },
        )
        .map_err(|e| e.to_string())?;

    if result.success {
        let _ = app
            .notification()
            .builder()
            .title("Layr")
            .body(format!("Workspace '{}' ready", workspace.name))
            .show();
    } else {
        let _ = app
            .notification()
            .builder()
            .title("Layr Warning")
            .body(format!(
                "Workspace '{}' launched with warnings",
                workspace.name
            ))
            .show();
    }

    Ok(result)
}

#[tauri::command]
pub fn launch_workspace(
    app: tauri::AppHandle,
    state: State<AppState>,
    id: String,
) -> Result<WorkspaceExecutionResult, String> {
    execute_workspace_internal(&app, &state, &id)
}

#[tauri::command]
pub fn get_workspaces(state: State<AppState>) -> Result<Vec<Workspace>, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let data = storage.load().map_err(|e| e.to_string())?;
    Ok(data.workspaces)
}

#[tauri::command]
pub fn get_workspace(state: State<AppState>, id: String) -> Result<Workspace, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let data = storage.load().map_err(|e| e.to_string())?;
    data.workspaces
        .into_iter()
        .find(|w| w.id == id)
        .ok_or_else(|| format!("Workspace with id '{}' not found", id))
}

#[tauri::command]
pub fn create_workspace(state: State<AppState>, workspace: Workspace) -> Result<Workspace, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut data = storage.load().map_err(|e| e.to_string())?;

    // Validate unique id
    if data.workspaces.iter().any(|w| w.id == workspace.id) {
        return Err(format!("Workspace with id '{}' already exists", workspace.id));
    }

    if let Some(ref sc_mgr) = state.shortcuts {
        if let Some(sc) = &workspace.shortcut {
            if !sc.trim().is_empty() && workspace.enabled {
                sc_mgr
                    .bind_workspace_shortcut(&workspace.id, sc)
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    data.workspaces.push(workspace.clone());
    storage.save(&data).map_err(|e| e.to_string())?;
    Ok(workspace)
}

#[tauri::command]
pub fn update_workspace(state: State<AppState>, workspace: Workspace) -> Result<Workspace, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut data = storage.load().map_err(|e| e.to_string())?;

    let index = data
        .workspaces
        .iter()
        .position(|w| w.id == workspace.id)
        .ok_or_else(|| format!("Workspace with id '{}' not found", workspace.id))?;

    let mut updated = workspace;
    updated.updated_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if let Some(ref sc_mgr) = state.shortcuts {
        if updated.enabled {
            if let Some(sc) = &updated.shortcut {
                if !sc.trim().is_empty() {
                    sc_mgr
                        .bind_workspace_shortcut(&updated.id, sc)
                        .map_err(|e| e.to_string())?;
                } else {
                    let _ = sc_mgr.unbind_workspace_shortcut(&updated.id);
                }
            } else {
                let _ = sc_mgr.unbind_workspace_shortcut(&updated.id);
            }
        } else {
            let _ = sc_mgr.unbind_workspace_shortcut(&updated.id);
        }
    }

    data.workspaces[index] = updated.clone();
    storage.save(&data).map_err(|e| e.to_string())?;
    Ok(updated)
}

#[tauri::command]
pub fn duplicate_workspace(state: State<AppState>, id: String) -> Result<Workspace, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut data = storage.load().map_err(|e| e.to_string())?;

    let original = data
        .workspaces
        .iter()
        .find(|w| w.id == id)
        .ok_or_else(|| format!("Workspace with id '{}' not found", id))?
        .clone();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let duplicated = Workspace {
        id: uuid::Uuid::new_v4().to_string(),
        name: format!("{} (Copy)", original.name),
        icon: original.icon,
        description: original.description,
        shortcut: None, // shortcuts must be unique, so don't copy the binding
        enabled: original.enabled,
        strategy: original.strategy,
        apps: original.apps,
        created_at: now,
        updated_at: now,
    };

    data.workspaces.push(duplicated.clone());
    storage.save(&data).map_err(|e| e.to_string())?;
    Ok(duplicated)
}

#[tauri::command]
pub fn toggle_workspace(state: State<AppState>, id: String, enabled: bool) -> Result<Workspace, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut data = storage.load().map_err(|e| e.to_string())?;

    let index = data
        .workspaces
        .iter()
        .position(|w| w.id == id)
        .ok_or_else(|| format!("Workspace with id '{}' not found", id))?;

    if let Some(ref sc_mgr) = state.shortcuts {
        if enabled {
            if let Some(sc) = &data.workspaces[index].shortcut {
                if !sc.trim().is_empty() {
                    let _ = sc_mgr.bind_workspace_shortcut(&id, sc);
                }
            }
        } else {
            let _ = sc_mgr.unbind_workspace_shortcut(&id);
        }
    }

    data.workspaces[index].enabled = enabled;
    data.workspaces[index].updated_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let updated = data.workspaces[index].clone();
    storage.save(&data).map_err(|e| e.to_string())?;
    Ok(updated)
}

#[tauri::command]
pub fn delete_workspace(state: State<AppState>, id: String) -> Result<bool, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut data = storage.load().map_err(|e| e.to_string())?;

    let before_len = data.workspaces.len();
    data.workspaces.retain(|w| w.id != id);

    if data.workspaces.len() == before_len {
        return Err(format!("Workspace with id '{}' not found", id));
    }

    if let Some(ref sc_mgr) = state.shortcuts {
        let _ = sc_mgr.unbind_workspace_shortcut(&id);
    }

    storage.save(&data).map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<crate::modules::storage::AppSettings, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut data = storage.load().map_err(|e| e.to_string())?;
    data.settings.autostart = crate::modules::storage::check_registry_autostart();
    Ok(data.settings)
}

#[tauri::command]
pub fn update_settings(
    state: State<AppState>,
    settings: crate::modules::storage::AppSettings,
) -> Result<crate::modules::storage::AppSettings, String> {
    let _ = crate::modules::storage::set_registry_autostart(settings.autostart);

    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut data = storage.load().map_err(|e| e.to_string())?;
    data.settings = settings.clone();
    storage.save(&data).map_err(|e| e.to_string())?;
    Ok(settings)
}

#[tauri::command]
pub fn export_config(state: State<AppState>) -> Result<String, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let data = storage.load().map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&data).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_config(state: State<AppState>, json_content: String) -> Result<Vec<Workspace>, String> {
    let imported: crate::modules::storage::StorageData =
        serde_json::from_str(&json_content).map_err(|e| format!("Invalid configuration JSON: {}", e))?;

    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    storage.save(&imported).map_err(|e| e.to_string())?;

    if let Some(ref sc_mgr) = state.shortcuts {
        for ws in &imported.workspaces {
            if ws.enabled {
                if let Some(sc) = &ws.shortcut {
                    if !sc.trim().is_empty() {
                        let _ = sc_mgr.bind_workspace_shortcut(&ws.id, sc);
                    }
                }
            }
        }
    }

    Ok(imported.workspaces)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn setup_test_state() -> (tempfile::TempDir, AppState) {
        let dir = tempdir().unwrap();
        let manager = StorageManager::new(dir.path());
        let state = AppState {
            storage: Mutex::new(manager),
            shortcuts: None,
            engine: ExecutionEngine::new(),
        };
        (dir, state)
    }

    #[test]
    fn test_crud_lifecycle() {
        let (_dir, state) = setup_test_state();

        // 1. Initially empty
        let initial = {
            let storage = state.storage.lock().unwrap();
            storage.load().unwrap().workspaces
        };
        assert!(initial.is_empty());

        // 2. Create workspace
        let mut ws = Workspace::new("Focus Flow");
        ws.description = "Deep work setup".to_string();
        ws.shortcut = Some("Ctrl+Alt+1".to_string());
        let ws_id = ws.id.clone();

        {
            let storage = state.storage.lock().unwrap();
            let mut data = storage.load().unwrap();
            data.workspaces.push(ws.clone());
            storage.save(&data).unwrap();
        }

        // 3. Read back
        {
            let storage = state.storage.lock().unwrap();
            let data = storage.load().unwrap();
            assert_eq!(data.workspaces.len(), 1);
            assert_eq!(data.workspaces[0].name, "Focus Flow");
        }

        // 4. Update workspace
        {
            let storage = state.storage.lock().unwrap();
            let mut data = storage.load().unwrap();
            let idx = data.workspaces.iter().position(|w| w.id == ws_id).unwrap();
            data.workspaces[idx].name = "Focus Flow Updated".to_string();
            storage.save(&data).unwrap();
        }

        {
            let storage = state.storage.lock().unwrap();
            let data = storage.load().unwrap();
            assert_eq!(data.workspaces[0].name, "Focus Flow Updated");
        }

        // 5. Duplicate workspace
        let dup_id = {
            let storage = state.storage.lock().unwrap();
            let mut data = storage.load().unwrap();
            let original = data.workspaces[0].clone();
            let dup = Workspace {
                id: "dup-123".to_string(),
                name: format!("{} (Copy)", original.name),
                icon: original.icon,
                description: original.description,
                shortcut: None,
                enabled: original.enabled,
                strategy: original.strategy,
                apps: original.apps,
                created_at: 100,
                updated_at: 100,
            };
            data.workspaces.push(dup.clone());
            storage.save(&data).unwrap();
            dup.id
        };

        {
            let storage = state.storage.lock().unwrap();
            let data = storage.load().unwrap();
            assert_eq!(data.workspaces.len(), 2);
            assert_eq!(data.workspaces[1].name, "Focus Flow Updated (Copy)");
            assert!(data.workspaces[1].shortcut.is_none());
        }

        // 6. Disable / Toggle workspace
        {
            let storage = state.storage.lock().unwrap();
            let mut data = storage.load().unwrap();
            let idx = data.workspaces.iter().position(|w| w.id == dup_id).unwrap();
            data.workspaces[idx].enabled = false;
            storage.save(&data).unwrap();
        }

        {
            let storage = state.storage.lock().unwrap();
            let data = storage.load().unwrap();
            let dup_ws = data.workspaces.iter().find(|w| w.id == dup_id).unwrap();
            assert!(!dup_ws.enabled);
        }

        // 7. Delete workspace
        {
            let storage = state.storage.lock().unwrap();
            let mut data = storage.load().unwrap();
            data.workspaces.retain(|w| w.id != ws_id);
            storage.save(&data).unwrap();
        }

        {
            let storage = state.storage.lock().unwrap();
            let data = storage.load().unwrap();
            assert_eq!(data.workspaces.len(), 1);
            assert_eq!(data.workspaces[0].id, dup_id);
        }
    }
}
