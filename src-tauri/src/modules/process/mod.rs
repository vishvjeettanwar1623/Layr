use crate::modules::workspace::{AppEntry, DuplicatePolicy};
use log::info;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

/// Structured error returned on process spawn failures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum ProcessError {
    #[error("Executable not found: {0}")]
    MissingExecutable(String),
    #[error("Access denied to execute: {0}")]
    AccessDenied(String),
    #[error("Failed to spawn process '{path}': {reason}")]
    SpawnFailed { path: String, reason: String },
}

/// Result of launching an application entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchResult {
    pub app_id: String,
    pub name: String,
    pub pid: Option<u32>,
    pub skipped: bool,
    pub error: Option<String>,
}

/// Quotes a single command line argument according to Windows (MSVC) parsing rules:
/// 1. If empty or contains whitespace/tabs/quotes, it must be enclosed in double quotes.
/// 2. Double quotes inside must be escaped with a backslash.
/// 3. Backslashes preceding a double quote must be doubled.
/// 4. Trailing backslashes before the closing quote must be doubled.
pub fn quote_windows_arg(arg: &str) -> String {
    if arg.is_empty() {
        return "\"\"".to_string();
    }

    let needs_quotes = arg.chars().any(|c| c == ' ' || c == '\t' || c == '\"');
    if !needs_quotes {
        return arg.to_string();
    }

    let mut result = String::with_capacity(arg.len() + 4);
    result.push('"');

    let mut backslash_count = 0;
    for c in arg.chars() {
        if c == '\\' {
            backslash_count += 1;
        } else if c == '"' {
            // Backslashes preceding quote are doubled + quote is escaped
            for _ in 0..backslash_count * 2 + 1 {
                result.push('\\');
            }
            result.push('"');
            backslash_count = 0;
        } else {
            for _ in 0..backslash_count {
                result.push('\\');
            }
            result.push(c);
            backslash_count = 0;
        }
    }

    // Trailing backslashes before closing quote must be doubled
    for _ in 0..backslash_count * 2 {
        result.push('\\');
    }
    result.push('"');

    result
}

/// Checks if an application is currently running by its executable filename.
pub fn is_app_running(exe_name_or_path: &str) -> bool {
    let target = Path::new(exe_name_or_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| exe_name_or_path.to_lowercase());

    // Use tasklist or snapshot check
    let output = Command::new("tasklist")
        .args(["/FI", &format!("IMAGENAME eq {}", target), "/NH"])
        .output();

    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
        text.contains(&target)
    } else {
        false
    }
}

/// Spawns an application defined by an AppEntry.
pub fn spawn_app(app: &AppEntry) -> Result<LaunchResult, ProcessError> {
    let path = &app.executable_path;

    // 1. Verify executable exists (unless it's an environment executable like 'cmd.exe', 'notepad.exe')
    if !path.is_empty() && path.contains(std::path::MAIN_SEPARATOR) && !Path::new(path).exists() {
        return Err(ProcessError::MissingExecutable(path.clone()));
    }

    // 2. Check duplicate instance policy
    if (app.duplicate_policy == DuplicatePolicy::SkipIfRunning
        || app.duplicate_policy == DuplicatePolicy::ReuseExistingWindow)
        && is_app_running(path)
    {
        info!("App '{}' is already running; skipping launch per policy {:?}", app.name, app.duplicate_policy);
        return Ok(LaunchResult {
            app_id: app.id.clone(),
            name: app.name.clone(),
            pid: None,
            skipped: true,
            error: None,
        });
    }

    // 3. Assemble arguments: explicitly configured args + URL (if flagged as supporting URL)
    let mut final_args = app.arguments.clone();
    if app.supports_url {
        if let Some(ref url) = app.url {
            if !url.trim().is_empty() {
                final_args.push(url.trim().to_string());
            }
        }
    }

    // 4. Build command
    let mut cmd = Command::new(path);
    cmd.args(&final_args);

    if let Some(ref wd) = app.working_directory {
        if !wd.trim().is_empty() && Path::new(wd).exists() {
            cmd.current_dir(wd);
        }
    }

    // 5. Spawn
    match cmd.spawn() {
        Ok(child) => {
            let pid = child.id();
            info!("Spawned '{}' (PID: {})", app.name, pid);
            Ok(LaunchResult {
                app_id: app.id.clone(),
                name: app.name.clone(),
                pid: Some(pid),
                skipped: false,
                error: None,
            })
        }
        Err(err) => {
            let err_kind = err.kind();
            if err_kind == std::io::ErrorKind::NotFound {
                Err(ProcessError::MissingExecutable(path.clone()))
            } else if err_kind == std::io::ErrorKind::PermissionDenied {
                Err(ProcessError::AccessDenied(path.clone()))
            } else {
                Err(ProcessError::SpawnFailed {
                    path: path.clone(),
                    reason: err.to_string(),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quote_simple_no_spaces() {
        assert_eq!(quote_windows_arg("notepad.exe"), "notepad.exe");
        assert_eq!(quote_windows_arg("abc123"), "abc123");
    }

    #[test]
    fn test_quote_spaces() {
        assert_eq!(
            quote_windows_arg("C:\\Program Files\\App\\run.exe"),
            "\"C:\\Program Files\\App\\run.exe\""
        );
    }

    #[test]
    fn test_quote_trailing_backslash() {
        // "C:\Directory\" -> "\"C:\\Directory\\\\\""
        assert_eq!(
            quote_windows_arg("C:\\Directory with spaces\\"),
            "\"C:\\Directory with spaces\\\\\""
        );
    }

    #[test]
    fn test_quote_inner_quotes_and_unicode() {
        // Spaces and unicode
        assert_eq!(
            quote_windows_arg("Hello 世界 with spaces"),
            "\"Hello 世界 with spaces\""
        );

        // Quotes inside
        assert_eq!(
            quote_windows_arg("a \"quoted\" string"),
            "\"a \\\"quoted\\\" string\""
        );
    }

    #[test]
    fn test_missing_executable_error() {
        let entry = AppEntry::new("Ghost App", "C:\\non_existent_folder_12345\\app.exe");
        let result = spawn_app(&entry);
        assert!(matches!(result, Err(ProcessError::MissingExecutable(_))));
    }
}
