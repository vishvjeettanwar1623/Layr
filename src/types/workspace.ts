export interface AppSettings {
  autostart: boolean;
  default_strategy: LaunchStrategy;
  default_window_timeout_secs: number;
  enable_notifications: boolean;
}

export type DuplicatePolicy = "always_launch" | "reuse_existing_window" | "skip_if_running";
export type LaunchStrategy = "sequential" | "parallel";

export type Dimension =
  | { type: "pixels"; value: number }
  | { type: "percent"; value: number };

export interface WindowPlacement {
  monitor_index: number;
  x: Dimension;
  y: Dimension;
  width: Dimension;
  height: Dimension;
  is_maximized: boolean;
}

export interface MonitorWorkArea {
  monitor_index: number;
  x: number;
  y: number;
  width: number;
  height: number;
  scale_factor: number;
  is_primary: boolean;
}

export interface AppEntry {
  id: string;
  name: string;
  executable_path: string;
  arguments: string[];
  working_directory?: string;
  url?: string;
  supports_url: boolean;
  launch_order: number;
  launch_delay_ms: number;
  duplicate_policy: DuplicatePolicy;
  window_config?: WindowPlacement;
}

export interface Workspace {
  id: string;
  name: string;
  icon: string;
  description: string;
  shortcut?: string;
  enabled: boolean;
  strategy?: LaunchStrategy;
  apps: AppEntry[];
  created_at: number;
  updated_at: number;
}

export interface DiscoveredApp {
  name: string;
  executable_path: string;
  icon_hint?: string;
  supports_url: boolean;
  is_preset: boolean;
}

export interface LaunchResult {
  app_id: string;
  name: string;
  pid?: number;
  skipped: boolean;
  error?: string;
}

export type AppLaunchStatus =
  | { type: "pending" }
  | { type: "starting" }
  | { type: "waiting_for_window" }
  | { type: "positioned" }
  | { type: "done" }
  | { type: "skipped" }
  | { type: "failed"; message: string };

export interface AppExecutionResult {
  app_id: string;
  app_name: string;
  status: AppLaunchStatus;
  duration_ms: number;
  hwnd?: number;
}

export interface WorkspaceExecutionResult {
  workspace_id: string;
  workspace_name: string;
  success: boolean;
  total_duration_ms: number;
  app_results: AppExecutionResult[];
}
