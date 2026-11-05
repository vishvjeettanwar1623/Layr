import { invoke } from "@tauri-apps/api/core";
import {
  AppEntry,
  AppSettings,
  DiscoveredApp,
  LaunchResult,
  MonitorWorkArea,
  Workspace,
  WorkspaceExecutionResult,
} from "../types/workspace";

export async function getWorkspaces(): Promise<Workspace[]> {
  return await invoke<Workspace[]>("get_workspaces");
}

export async function getWorkspace(id: string): Promise<Workspace> {
  return await invoke<Workspace>("get_workspace", { id });
}

export async function createWorkspace(workspace: Workspace): Promise<Workspace> {
  return await invoke<Workspace>("create_workspace", { workspace });
}

export async function updateWorkspace(workspace: Workspace): Promise<Workspace> {
  return await invoke<Workspace>("update_workspace", { workspace });
}

export async function duplicateWorkspace(id: string): Promise<Workspace> {
  return await invoke<Workspace>("duplicate_workspace", { id });
}

export async function toggleWorkspace(id: string, enabled: boolean): Promise<Workspace> {
  return await invoke<Workspace>("toggle_workspace", { id, enabled });
}

export async function deleteWorkspace(id: string): Promise<boolean> {
  return await invoke<boolean>("delete_workspace", { id });
}

export async function getPresets(): Promise<DiscoveredApp[]> {
  return await invoke<DiscoveredApp[]>("get_presets");
}

export async function discoverApps(): Promise<DiscoveredApp[]> {
  return await invoke<DiscoveredApp[]>("discover_apps");
}

export async function getMonitors(): Promise<MonitorWorkArea[]> {
  return await invoke<MonitorWorkArea[]>("get_monitors");
}

export async function launchApp(app: AppEntry): Promise<LaunchResult> {
  return await invoke<LaunchResult>("launch_app", { app });
}

export async function launchWorkspace(id: string): Promise<WorkspaceExecutionResult> {
  return await invoke<WorkspaceExecutionResult>("launch_workspace", { id });
}

export async function getSettings(): Promise<AppSettings> {
  return await invoke<AppSettings>("get_settings");
}

export async function updateSettings(settings: AppSettings): Promise<AppSettings> {
  return await invoke<AppSettings>("update_settings", { settings });
}

export async function exportConfig(): Promise<string> {
  return await invoke<string>("export_config");
}

export async function importConfig(jsonContent: string): Promise<Workspace[]> {
  return await invoke<Workspace[]>("import_config", { jsonContent });
}
