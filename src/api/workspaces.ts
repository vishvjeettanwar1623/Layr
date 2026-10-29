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
