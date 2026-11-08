use crate::modules::workspace::{LaunchStrategy, Workspace};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

fn default_false() -> bool {
    false
}

fn default_true() -> bool {
    true
}

fn default_timeout() -> u64 {
    6
}

/// User application preferences.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_false")]
    pub autostart: bool,
    #[serde(default)]
    pub default_strategy: LaunchStrategy,
    #[serde(default = "default_timeout")]
    pub default_window_timeout_secs: u64,
    #[serde(default = "default_true")]
    pub enable_notifications: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            autostart: false,
            default_strategy: LaunchStrategy::Sequential,
            default_window_timeout_secs: 6,
            enable_notifications: true,
        }
    }
}

/// Root container stored in `workspaces.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StorageData {
    pub schema_version: u32,
    pub workspaces: Vec<Workspace>,
    #[serde(default)]
    pub settings: AppSettings,
}

impl Default for StorageData {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            workspaces: Vec::new(),
            settings: AppSettings::default(),
        }
    }
}

/// Helper to inspect Windows Run registry key.
pub fn check_registry_autostart() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run") {
        let val: Result<String, _> = key.get_value("Layr");
        return val.is_ok();
    }
    false
}

/// Helper to configure Windows Run registry key.
pub fn set_registry_autostart(enable: bool) -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
        .map_err(|e| e.to_string())?;

    if enable {
        let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let exe_str = current_exe.to_string_lossy().to_string();
        key.set_value("Layr", &exe_str).map_err(|e| e.to_string())?;
    } else {
        let _ = key.delete_value("Layr");
    }
    Ok(())
}

/// Errors occurring during storage operations.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Corrupt data file and backup unreadable: {0}")]
    Corrupt(String),
    #[error("Unsupported schema version: {0}")]
    UnsupportedSchema(u32),
}

/// Storage manager handling atomic persistence, backups, and schema migrations.
pub struct StorageManager {
    base_dir: PathBuf,
    file_name: String,
}

impl StorageManager {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            file_name: "workspaces.json".to_string(),
        }
    }

    /// Resolves the default app data directory for Layr.
    pub fn default_dir() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Layr")
    }

    pub fn primary_path(&self) -> PathBuf {
        self.base_dir.join(&self.file_name)
    }

    pub fn backup_path(&self) -> PathBuf {
        self.base_dir.join(format!("{}.bak", self.file_name))
    }

    pub fn temp_path(&self) -> PathBuf {
        let rand_suffix: u32 = fastrand::u32(..);
        self.base_dir
            .join(format!(".{}.tmp-{}", self.file_name, rand_suffix))
    }

    /// Load storage data, applying migrations and falling back to backup on corruption.
    pub fn load(&self) -> Result<StorageData, StorageError> {
        let primary = self.primary_path();

        if !primary.exists() {
            // First run, check if a backup exists
            let backup = self.backup_path();
            if backup.exists() {
                warn!("Primary file missing, attempting recovery from backup: {:?}", backup);
                return self.load_from_path(&backup);
            }
            // Return empty default
            return Ok(StorageData::default());
        }

        match self.load_from_path(&primary) {
            Ok(data) => Ok(data),
            Err(err) => {
                error!("Primary file corrupted ({:?}): {}. Trying backup...", primary, err);
                let backup = self.backup_path();
                if backup.exists() {
                    match self.load_from_path(&backup) {
                        Ok(data) => {
                            warn!("Successfully recovered data from backup: {:?}", backup);
                            // Restore primary from backup
                            let _ = self.save(&data);
                            return Ok(data);
                        }
                        Err(backup_err) => {
                            error!("Backup file also unreadable: {}", backup_err);
                        }
                    }
                }
                Err(StorageError::Corrupt(format!(
                    "Primary data corrupted ({}) and backup unavailable",
                    err
                )))
            }
        }
    }

    fn load_from_path(&self, path: &Path) -> Result<StorageData, StorageError> {
        let mut file = File::open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        let raw: serde_json::Value = serde_json::from_str(&content)?;
        self.migrate_and_deserialize(raw)
    }

    /// Save storage data atomically: write to a temporary file in the same directory,
    /// flush and sync, copy existing primary to backup, then atomically rename temp to primary.
    pub fn save(&self, data: &StorageData) -> Result<(), StorageError> {
        fs::create_dir_all(&self.base_dir)?;

        let temp = self.temp_path();
        let primary = self.primary_path();
        let backup = self.backup_path();

        // 1. Serialize to pretty JSON
        let json = serde_json::to_string_pretty(data)?;

        // 2. Write and sync to temporary file in the same directory (ensures same filesystem)
        {
            let mut file = File::create(&temp)?;
            file.write_all(json.as_bytes())?;
            file.sync_all()?;
        }

        // 3. Update backup copy (if primary exists, copy it; otherwise copy temp)
        if primary.exists() {
            let _ = fs::copy(&primary, &backup);
        } else {
            let _ = fs::copy(&temp, &backup);
        }

        // 4. Atomic rename temp -> primary (replaces destination on Windows/Unix)
        if let Err(err) = fs::rename(&temp, &primary) {
            // Cleanup temp file on failure
            let _ = fs::remove_file(&temp);
            return Err(StorageError::Io(err));
        }

        info!("Storage atomically saved to {:?}", primary);
        Ok(())
    }

    /// Schema migration hook.
    fn migrate_and_deserialize(&self, raw: serde_json::Value) -> Result<StorageData, StorageError> {
        let version = raw
            .get("schema_version")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as u32;

        if version > CURRENT_SCHEMA_VERSION {
            return Err(StorageError::UnsupportedSchema(version));
        }

        let _current_ver = version;

        // Future migration steps can be chained here:
        // while current_ver < CURRENT_SCHEMA_VERSION {
        //     match current_ver {
        //         1 => { raw = migrate_v1_to_v2(raw)?; current_ver = 2; }
        //         _ => break,
        //     }
        // }

        let data: StorageData = serde_json::from_value(raw)?;
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_settings_backwards_compatibility_and_defaults() {
        // Raw JSON without `settings` field
        let raw_json = r#"{"schema_version": 1, "workspaces": []}"#;
        let data: StorageData = serde_json::from_str(raw_json).unwrap();
        assert!(!data.settings.autostart);
        assert_eq!(data.settings.default_window_timeout_secs, 6);
        assert!(data.settings.enable_notifications);
    }

    #[test]
    fn test_storage_roundtrip() {
        let dir = tempdir().unwrap();
        let manager = StorageManager::new(dir.path());

        let mut data = StorageData::default();
        let mut ws = Workspace::new("Dev Workspace");
        ws.description = "Primary dev environment".to_string();
        ws.shortcut = Some("Ctrl+Alt+D".to_string());
        data.workspaces.push(ws);

        // Save
        manager.save(&data).expect("save should succeed");

        // Load
        let loaded = manager.load().expect("load should succeed");
        assert_eq!(loaded.workspaces.len(), 1);
        assert_eq!(loaded.workspaces[0].name, "Dev Workspace");
        assert_eq!(loaded.workspaces[0].description, "Primary dev environment");
        assert_eq!(loaded.workspaces[0].shortcut, Some("Ctrl+Alt+D".to_string()));
    }

    #[test]
    fn test_atomic_write_simulation() {
        let dir = tempdir().unwrap();
        let manager = StorageManager::new(dir.path());

        let mut data = StorageData::default();
        data.workspaces.push(Workspace::new("Original"));
        manager.save(&data).unwrap();

        // Simulate crash: an orphaned temp file was left over
        let orphan_temp = dir.path().join(".workspaces.json.tmp-999");
        fs::write(&orphan_temp, b"partially written garbage").unwrap();

        // Save new data
        let mut new_data = StorageData::default();
        new_data.workspaces.push(Workspace::new("Updated"));
        manager.save(&new_data).unwrap();

        // Loading should yield the updated data without corruption
        let loaded = manager.load().unwrap();
        assert_eq!(loaded.workspaces[0].name, "Updated");
    }

    #[test]
    fn test_corrupt_file_recovery_from_backup() {
        let dir = tempdir().unwrap();
        let manager = StorageManager::new(dir.path());

        let mut data = StorageData::default();
        data.workspaces.push(Workspace::new("Important Data"));
        manager.save(&data).unwrap();

        assert!(manager.backup_path().exists(), "Backup should have been created on save");

        // Simulate primary file corruption (truncated/garbage JSON)
        fs::write(manager.primary_path(), b"{\"schema_version\": 1, invalid json...").unwrap();

        // Load should detect corruption and fall back to backup
        let loaded = manager.load().expect("Should recover from backup");
        assert_eq!(loaded.workspaces.len(), 1);
        assert_eq!(loaded.workspaces[0].name, "Important Data");
    }

    #[test]
    fn test_corrupt_without_backup_errors_never_silent_wiping() {
        let dir = tempdir().unwrap();
        let manager = StorageManager::new(dir.path());

        // Corrupt file with NO backup
        fs::write(manager.primary_path(), b"garbage binary data").unwrap();

        // Must return error, NOT an empty/silently wiped default!
        let result = manager.load();
        assert!(result.is_err(), "Must return error when data is corrupt and no backup exists");
    }

    #[test]
    fn test_schema_migration_hook() {
        let dir = tempdir().unwrap();
        let manager = StorageManager::new(dir.path());

        // Write a raw JSON with schema_version 1
        let raw_json = r#"{
            "schema_version": 1,
            "workspaces": [
                {
                    "id": "test-id-1",
                    "name": "Legacy Workspace",
                    "icon": "App",
                    "description": "",
                    "enabled": true,
                    "apps": [],
                    "created_at": 1000,
                    "updated_at": 1000
                }
            ]
        }"#;
        fs::write(manager.primary_path(), raw_json).unwrap();

        let loaded = manager.load().expect("Legacy schema v1 should load cleanly");
        assert_eq!(loaded.schema_version, 1);
        assert_eq!(loaded.workspaces[0].name, "Legacy Workspace");
    }
}
