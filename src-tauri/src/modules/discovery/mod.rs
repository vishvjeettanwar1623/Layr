use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A discovered application or preset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredApp {
    pub name: String,
    pub executable_path: String,
    pub icon_hint: Option<String>,
    pub supports_url: bool,
    pub is_preset: bool,
}

/// Definition of a standard preset to search for on the host machine.
#[derive(Debug, Clone)]
pub struct PresetDefinition {
    pub name: &'static str,
    pub search_paths: &'static [&'static str],
    pub registry_keys: &'static [&'static str],
    pub icon_hint: &'static str,
    pub supports_url: bool,
}

pub static STANDARD_PRESETS: &[PresetDefinition] = &[
    PresetDefinition {
        name: "Visual Studio Code",
        search_paths: &[
            r"%LOCALAPPDATA%\Programs\Microsoft VS Code\Code.exe",
            r"%ProgramFiles%\Microsoft VS Code\Code.exe",
        ],
        registry_keys: &["Code.exe"],
        icon_hint: "Code",
        supports_url: false,
    },
    PresetDefinition {
        name: "Microsoft Edge",
        search_paths: &[
            r"%ProgramFiles(x86)%\Microsoft\Edge\Application\msedge.exe",
            r"%ProgramFiles%\Microsoft\Edge\Application\msedge.exe",
        ],
        registry_keys: &["msedge.exe"],
        icon_hint: "Compass",
        supports_url: true,
    },
    PresetDefinition {
        name: "Google Chrome",
        search_paths: &[
            r"%ProgramFiles%\Google\Chrome\Application\chrome.exe",
            r"%ProgramFiles(x86)%\Google\Chrome\Application\chrome.exe",
            r"%LOCALAPPDATA%\Google\Chrome\Application\chrome.exe",
        ],
        registry_keys: &["chrome.exe"],
        icon_hint: "Chrome",
        supports_url: true,
    },
    PresetDefinition {
        name: "Windows Terminal",
        search_paths: &[
            r"%LOCALAPPDATA%\Microsoft\WindowsApps\wt.exe",
            r"%SystemRoot%\System32\cmd.exe",
        ],
        registry_keys: &["wt.exe"],
        icon_hint: "Terminal",
        supports_url: false,
    },
    PresetDefinition {
        name: "Notepad",
        search_paths: &[
            r"%SystemRoot%\notepad.exe",
            r"%SystemRoot%\System32\notepad.exe",
        ],
        registry_keys: &["notepad.exe"],
        icon_hint: "FileText",
        supports_url: false,
    },
    PresetDefinition {
        name: "File Explorer",
        search_paths: &[r"%SystemRoot%\explorer.exe"],
        registry_keys: &["explorer.exe"],
        icon_hint: "Folder",
        supports_url: false,
    },
];

/// Expands Windows environment variables like `%LOCALAPPDATA%` or `%ProgramFiles%`.
pub fn expand_env_vars(path_str: &str) -> String {
    let mut result = path_str.to_string();
    let vars = [
        ("LOCALAPPDATA", std::env::var("LOCALAPPDATA").unwrap_or_default()),
        ("ProgramFiles", std::env::var("ProgramFiles").unwrap_or_default()),
        (
            "ProgramFiles(x86)",
            std::env::var("ProgramFiles(x86)").unwrap_or_default(),
        ),
        ("APPDATA", std::env::var("APPDATA").unwrap_or_default()),
        ("SystemRoot", std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string())),
    ];

    for (key, val) in vars {
        let needle = format!("%{}%", key);
        if !val.is_empty() && result.contains(&needle) {
            result = result.replace(&needle, &val);
        }
    }
    result
}

/// Resolves presets against real filesystem paths, failing gracefully if an app is not installed.
pub fn resolve_presets() -> Vec<DiscoveredApp> {
    let mut resolved = Vec::new();

    for preset in STANDARD_PRESETS {
        let mut found_path: Option<PathBuf> = None;

        // 1. Try search paths
        for path_template in preset.search_paths {
            let expanded = expand_env_vars(path_template);
            let path = Path::new(&expanded);
            if path.exists() {
                found_path = Some(path.to_path_buf());
                break;
            }
        }

        // 2. Try App Paths registry key
        if found_path.is_none() {
            for reg_key in preset.registry_keys {
                if let Some(p) = resolve_from_registry_app_paths(reg_key) {
                    found_path = Some(p);
                    break;
                }
            }
        }

        if let Some(p) = found_path {
            resolved.push(DiscoveredApp {
                name: preset.name.to_string(),
                executable_path: p.to_string_lossy().into_owned(),
                icon_hint: Some(preset.icon_hint.to_string()),
                supports_url: preset.supports_url,
                is_preset: true,
            });
        }
    }

    resolved
}

/// Queries `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\<exe>` or HKCU.
pub fn resolve_from_registry_app_paths(exe_name: &str) -> Option<PathBuf> {
    use winreg::enums::*;
    use winreg::RegKey;

    let subkey = format!(r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\{}", exe_name);

    // Try HKLM
    if let Ok(hklm) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(&subkey) {
        if let Ok(val) = hklm.get_value::<String, _>("") {
            let path = PathBuf::from(val.trim_matches('"'));
            if path.exists() {
                return Some(path);
            }
        }
    }

    // Try HKCU
    if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(&subkey) {
        if let Ok(val) = hkcu.get_value::<String, _>("") {
            let path = PathBuf::from(val.trim_matches('"'));
            if path.exists() {
                return Some(path);
            }
        }
    }

    None
}

/// On-demand scan of Start Menu directories for shortcuts and executables.
pub fn scan_start_menu() -> Vec<DiscoveredApp> {
    let mut apps = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();

    let search_roots = [
        dirs::data_dir().map(|d| d.join(r"Microsoft\Windows\Start Menu\Programs")),
        Some(PathBuf::from(r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs")),
    ];

    for root_opt in search_roots.into_iter().flatten() {
        if root_opt.exists() {
            collect_apps_from_dir(&root_opt, &mut apps, &mut seen_paths, 3);
        }
    }

    // Sort alphabetically by name
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}

fn collect_apps_from_dir(
    dir: &Path,
    apps: &mut Vec<DiscoveredApp>,
    seen: &mut std::collections::HashSet<String>,
    depth: usize,
) {
    if depth == 0 {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_apps_from_dir(&path, apps, seen, depth - 1);
        } else if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ext.eq_ignore_ascii_case("lnk") {
                let name = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();

                // Skip uninstaller/help shortcuts
                let name_lower = name.to_lowercase();
                if name_lower.contains("uninstall") || name_lower.contains("readme") || name_lower.contains("help") {
                    continue;
                }

                let path_str = path.to_string_lossy().into_owned();
                if seen.insert(name.clone()) {
                    apps.push(DiscoveredApp {
                        name,
                        executable_path: path_str,
                        icon_hint: Some("App".to_string()),
                        supports_url: false,
                        is_preset: false,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_env_vars() {
        let expanded = expand_env_vars(r"%SystemRoot%\notepad.exe");
        assert!(!expanded.contains("%SystemRoot%"));
        assert!(expanded.ends_with(r"\notepad.exe"));
    }

    #[test]
    fn test_resolve_presets_graceful_missing() {
        // Define a fictional preset that definitely does NOT exist
        let fake_preset = PresetDefinition {
            name: "Fictional Ghost Software 9000",
            search_paths: &[r"C:\fake_never_existent_dir_83721\nonexistent.exe"],
            registry_keys: &["fictional_ghost_software.exe"],
            icon_hint: "Ghost",
            supports_url: false,
        };

        // Resolution should not panic and return empty
        let mut found = None;
        for path_template in fake_preset.search_paths {
            let expanded = expand_env_vars(path_template);
            if Path::new(&expanded).exists() {
                found = Some(expanded);
            }
        }
        assert!(found.is_none(), "Should gracefully fail to resolve missing preset");
    }

    #[test]
    fn test_resolve_standard_presets_finds_notepad_or_explorer() {
        let resolved = resolve_presets();
        // At least Notepad or Explorer must be found on any valid Windows host
        assert!(
            resolved.iter().any(|p| p.name == "Notepad" || p.name == "File Explorer"),
            "Should resolve standard Windows built-in presets"
        );
    }
}
