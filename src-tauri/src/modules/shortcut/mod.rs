use log::{info, warn};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri_plugin_global_shortcut::Shortcut;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Clone)]
pub enum ShortcutError {
    #[error("Invalid shortcut format: '{0}'. Expected format like 'Ctrl+Alt+W'")]
    InvalidFormat(String),
    #[error("Shortcut '{0}' is already assigned to workspace '{1}'")]
    DuplicateAssignment(String, String),
    #[error("Shortcut '{0}' is already registered by another application or the OS")]
    SystemConflict(String),
    #[error("Failed to register shortcut: {0}")]
    RegistrationFailed(String),
    #[error("Shortcut not found: '{0}'")]
    NotFound(String),
}

/// Abstract provider for interacting with the OS global shortcut manager.
/// Allows mocking in unit tests without OS hotkey APIs.
pub trait OsShortcutProvider: Send + Sync {
    fn is_registered(&self, shortcut: &Shortcut) -> bool;
    fn register(&self, shortcut: Shortcut) -> Result<(), String>;
    fn unregister(&self, shortcut: &Shortcut) -> Result<(), String>;
}

pub struct TauriShortcutProvider {
    app: tauri::AppHandle,
}

impl TauriShortcutProvider {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self { app }
    }
}

impl OsShortcutProvider for TauriShortcutProvider {
    fn is_registered(&self, shortcut: &Shortcut) -> bool {
        use tauri_plugin_global_shortcut::GlobalShortcutExt;
        self.app.global_shortcut().is_registered(*shortcut)
    }

    fn register(&self, shortcut: Shortcut) -> Result<(), String> {
        use tauri_plugin_global_shortcut::GlobalShortcutExt;
        self.app
            .global_shortcut()
            .register(shortcut)
            .map_err(|e| e.to_string())
    }

    fn unregister(&self, shortcut: &Shortcut) -> Result<(), String> {
        use tauri_plugin_global_shortcut::GlobalShortcutExt;
        self.app
            .global_shortcut()
            .unregister(*shortcut)
            .map_err(|e| e.to_string())
    }
}

/// In-memory shortcut registry to track workspace-to-shortcut bindings, prevent internal duplicates,
/// handle safe rebinding with rollback, and coordinate with the OS provider.
pub struct ShortcutManager<P: OsShortcutProvider> {
    provider: P,
    // Maps normalized shortcut string -> workspace_id
    assigned_shortcuts: Mutex<HashMap<String, String>>,
    // Maps workspace_id -> (normalized shortcut string, parsed Shortcut)
    workspace_shortcuts: Mutex<HashMap<String, (String, Shortcut)>>,
}

impl<P: OsShortcutProvider> ShortcutManager<P> {
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            assigned_shortcuts: Mutex::new(HashMap::new()),
            workspace_shortcuts: Mutex::new(HashMap::new()),
        }
    }

    /// Normalizes and validates a shortcut string.
    pub fn parse_and_normalize(shortcut_str: &str) -> Result<(String, Shortcut), ShortcutError> {
        let trimmed = shortcut_str.trim();
        if trimmed.is_empty() {
            return Err(ShortcutError::InvalidFormat(
                "Shortcut cannot be empty".to_string(),
            ));
        }

        let parsed: Shortcut = trimmed.parse().map_err(|e| {
            ShortcutError::InvalidFormat(format!("{}: {}", trimmed, e))
        })?;

        // Standardize string representation to lowercase
        let normalized = parsed.to_string().to_lowercase();
        Ok((normalized, parsed))
    }

    /// Get the workspace ID assigned to a shortcut.
    pub fn get_workspace_for_shortcut(&self, shortcut_str: &str) -> Option<String> {
        let (normalized, _) = Self::parse_and_normalize(shortcut_str).ok()?;
        self.assigned_shortcuts
            .lock()
            .ok()?
            .get(&normalized)
            .cloned()
    }

    /// Register or rebind a shortcut for a workspace.
    /// Safely unbinds old shortcut if exists, and rolls back if new registration fails.
    pub fn bind_workspace_shortcut(
        &self,
        workspace_id: &str,
        shortcut_str: &str,
    ) -> Result<String, ShortcutError> {
        let (normalized, parsed) = Self::parse_and_normalize(shortcut_str)?;

        let mut assigned = self
            .assigned_shortcuts
            .lock()
            .map_err(|e| ShortcutError::RegistrationFailed(e.to_string()))?;
        let mut by_ws = self
            .workspace_shortcuts
            .lock()
            .map_err(|e| ShortcutError::RegistrationFailed(e.to_string()))?;

        // 1. Check if assigned to another workspace in Layr
        if let Some(existing_ws) = assigned.get(&normalized) {
            if existing_ws != workspace_id {
                return Err(ShortcutError::DuplicateAssignment(
                    normalized,
                    existing_ws.clone(),
                ));
            }
            // Same workspace already has this shortcut bound; nothing to do
            return Ok(normalized);
        }

        // 2. Remember previous binding if re-binding
        let previous = by_ws.get(workspace_id).cloned();

        // 3. Unbind previous if exists
        if let Some((prev_norm, prev_parsed)) = &previous {
            let _ = self.provider.unregister(prev_parsed);
            assigned.remove(prev_norm);
        }

        // 4. Attempt to register new shortcut with OS
        if let Err(err) = self.provider.register(parsed) {
            warn!(
                "Failed to register shortcut '{}' with OS: {}",
                normalized, err
            );
            // Roll back previous binding if it existed
            if let Some((prev_norm, prev_parsed)) = previous {
                let _ = self.provider.register(prev_parsed);
                assigned.insert(prev_norm.clone(), workspace_id.to_string());
                by_ws.insert(workspace_id.to_string(), (prev_norm, prev_parsed));
            }

            return Err(ShortcutError::SystemConflict(format!(
                "{}: {}",
                normalized, err
            )));
        }

        // 5. Update local state
        assigned.insert(normalized.clone(), workspace_id.to_string());
        by_ws.insert(workspace_id.to_string(), (normalized.clone(), parsed));

        info!(
            "Successfully bound shortcut '{}' to workspace '{}'",
            normalized, workspace_id
        );
        Ok(normalized)
    }

    /// Unbind shortcut for a workspace (e.g. when workspace is disabled or deleted).
    pub fn unbind_workspace_shortcut(
        &self,
        workspace_id: &str,
    ) -> Result<(), ShortcutError> {
        let mut assigned = self
            .assigned_shortcuts
            .lock()
            .map_err(|e| ShortcutError::RegistrationFailed(e.to_string()))?;
        let mut by_ws = self
            .workspace_shortcuts
            .lock()
            .map_err(|e| ShortcutError::RegistrationFailed(e.to_string()))?;

        if let Some((norm, parsed)) = by_ws.remove(workspace_id) {
            let _ = self.provider.unregister(&parsed);
            assigned.remove(&norm);
            info!("Unbound shortcut '{}' from workspace '{}'", norm, workspace_id);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[derive(Default)]
    struct MockOsProvider {
        registered: Mutex<HashSet<String>>,
        system_blocked: Mutex<HashSet<String>>,
    }

    impl OsShortcutProvider for MockOsProvider {
        fn is_registered(&self, shortcut: &Shortcut) -> bool {
            self.registered
                .lock()
                .unwrap()
                .contains(&shortcut.to_string().to_lowercase())
        }

        fn register(&self, shortcut: Shortcut) -> Result<(), String> {
            let norm = shortcut.to_string().to_lowercase();
            if self.system_blocked.lock().unwrap().contains(&norm) {
                return Err("Hotkey already registered by OS/another application".to_string());
            }
            self.registered.lock().unwrap().insert(norm);
            Ok(())
        }

        fn unregister(&self, shortcut: &Shortcut) -> Result<(), String> {
            let norm = shortcut.to_string().to_lowercase();
            self.registered.lock().unwrap().remove(&norm);
            Ok(())
        }
    }

    #[test]
    fn test_valid_and_invalid_shortcut_formats() {
        let valid = ShortcutManager::<MockOsProvider>::parse_and_normalize("Ctrl+Alt+W");
        assert!(valid.is_ok());

        let invalid = ShortcutManager::<MockOsProvider>::parse_and_normalize("invalid-combo");
        assert!(matches!(invalid.err(), Some(ShortcutError::InvalidFormat(_))));

        let empty = ShortcutManager::<MockOsProvider>::parse_and_normalize("");
        assert!(matches!(empty.err(), Some(ShortcutError::InvalidFormat(_))));
    }

    #[test]
    fn test_duplicate_detection_between_workspaces() {
        let provider = MockOsProvider::default();
        let manager = ShortcutManager::new(provider);

        // Workspace 1 binds Ctrl+Alt+1
        let res1 = manager.bind_workspace_shortcut("ws-1", "Ctrl+Alt+1");
        assert!(res1.is_ok());

        // Workspace 2 attempts to bind the same shortcut -> rejected
        let res2 = manager.bind_workspace_shortcut("ws-2", "Ctrl+Alt+1");
        assert!(matches!(
            res2.err(),
            Some(ShortcutError::DuplicateAssignment(_, ws)) if ws == "ws-1"
        ));
    }

    #[test]
    fn test_system_conflict_handling_and_rollback() {
        let provider = MockOsProvider::default();
        // Simulate OS already owning Ctrl+Alt+D
        let (blocked_norm, _) =
            ShortcutManager::<MockOsProvider>::parse_and_normalize("Ctrl+Alt+D").unwrap();
        provider
            .system_blocked
            .lock()
            .unwrap()
            .insert(blocked_norm);

        let manager = ShortcutManager::new(provider);

        // 1. Initial bind to Ctrl+Alt+1 works
        let res1 = manager.bind_workspace_shortcut("ws-1", "Ctrl+Alt+1");
        assert!(res1.is_ok());

        // 2. Rebinding to blocked combo fails with SystemConflict
        let res2 = manager.bind_workspace_shortcut("ws-1", "Ctrl+Alt+D");
        assert!(matches!(res2.err(), Some(ShortcutError::SystemConflict(_))));

        // 3. Confirm rollback: previous binding Ctrl+Alt+1 is still active
        assert_eq!(
            manager.get_workspace_for_shortcut("Ctrl+Alt+1"),
            Some("ws-1".to_string())
        );
    }

    #[test]
    fn test_safe_rebinding_and_unbinding() {
        let provider = MockOsProvider::default();
        let manager = ShortcutManager::new(provider);

        // Bind Ctrl+Alt+1
        let _ = manager.bind_workspace_shortcut("ws-1", "Ctrl+Alt+1").unwrap();
        assert_eq!(
            manager.get_workspace_for_shortcut("Ctrl+Alt+1"),
            Some("ws-1".to_string())
        );

        // Rebind to Ctrl+Alt+2
        let _ = manager.bind_workspace_shortcut("ws-1", "Ctrl+Alt+2").unwrap();
        assert_eq!(manager.get_workspace_for_shortcut("Ctrl+Alt+1"), None);
        assert_eq!(
            manager.get_workspace_for_shortcut("Ctrl+Alt+2"),
            Some("ws-1".to_string())
        );

        // Unbind
        manager.unbind_workspace_shortcut("ws-1").unwrap();
        assert_eq!(manager.get_workspace_for_shortcut("Ctrl+Alt+2"), None);
    }
}
