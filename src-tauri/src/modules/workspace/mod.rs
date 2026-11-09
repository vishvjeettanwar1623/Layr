use crate::modules::window::layout_math::{PositionPreset, WindowPlacement};
use serde::{Deserialize, Serialize};

/// Policy for handling already running instances of an application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DuplicatePolicy {
    #[default]
    AlwaysLaunch,
    ReuseExistingWindow,
    SkipIfRunning,
}

/// An application launch entry inside a workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub executable_path: String,
    #[serde(default)]
    pub arguments: Vec<String>,
    #[serde(default)]
    pub working_directory: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub supports_url: bool,
    #[serde(default)]
    pub launch_order: u32,
    #[serde(default)]
    pub launch_delay_ms: u64,
    #[serde(default)]
    pub duplicate_policy: DuplicatePolicy,
    #[serde(default)]
    pub window_config: Option<WindowPlacement>,
}

impl AppEntry {
    pub fn new(name: impl Into<String>, executable_path: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            executable_path: executable_path.into(),
            arguments: Vec::new(),
            working_directory: None,
            url: None,
            supports_url: false,
            launch_order: 0,
            launch_delay_ms: 0,
            duplicate_policy: DuplicatePolicy::AlwaysLaunch,
            window_config: Some(WindowPlacement::preset(PositionPreset::LeftHalf)),
        }
    }
}

/// Strategy for launching applications in a workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LaunchStrategy {
    #[default]
    Sequential,
    Parallel,
}

/// A user workspace configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub shortcut: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub strategy: LaunchStrategy,
    #[serde(default)]
    pub apps: Vec<AppEntry>,
    #[serde(default = "current_timestamp")]
    pub created_at: u64,
    #[serde(default = "current_timestamp")]
    pub updated_at: u64,
}

fn default_true() -> bool {
    true
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

impl Workspace {
    pub fn new(name: impl Into<String>) -> Self {
        let now = current_timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            icon: "WS".to_string(),
            description: String::new(),
            shortcut: None,
            enabled: true,
            strategy: LaunchStrategy::Sequential,
            apps: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}
