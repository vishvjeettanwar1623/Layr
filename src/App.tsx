import { useState, useEffect } from "react";
import { Workspace } from "./types/workspace";
import * as api from "./api/workspaces";
import { Sidebar, NavTab } from "./components/Sidebar";
import { WorkspaceCard } from "./components/WorkspaceCard";
import { WorkspaceEditor } from "./components/WorkspaceEditor";
import { SettingsView } from "./components/SettingsView";
import { Button } from "./components/ui/Button";

function App() {
  const [activeTab, setActiveTab] = useState<NavTab>("workspaces");
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState("");
  const [editorOpen, setEditorOpen] = useState(false);
  const [editingWorkspace, setEditingWorkspace] = useState<Workspace | null>(null);

  // Load workspaces on mount
  useEffect(() => {
    loadWorkspaces();
  }, []);

  const loadWorkspaces = async () => {
    try {
      setLoading(true);
      const data = await api.getWorkspaces();
      setWorkspaces(data);
    } catch (err) {
      console.error("Failed to load workspaces:", err);
    } finally {
      setLoading(false);
    }
  };

  const handleCreateNew = () => {
    setEditingWorkspace(null);
    setEditorOpen(true);
  };

  const handleEdit = (ws: Workspace) => {
    setEditingWorkspace(ws);
    setEditorOpen(true);
  };

  const handleSaveWorkspace = async (workspace: Workspace) => {
    try {
      if (editingWorkspace) {
        const updated = await api.updateWorkspace(workspace);
        setWorkspaces((prev) =>
          prev.map((w) => (w.id === updated.id ? updated : w))
        );
      } else {
        const created = await api.createWorkspace(workspace);
        setWorkspaces((prev) => [...prev, created]);
      }
    } catch (err) {
      console.error("Failed to save workspace:", err);
    }
  };

  const handleDuplicate = async (id: string) => {
    try {
      const duplicated = await api.duplicateWorkspace(id);
      setWorkspaces((prev) => [...prev, duplicated]);
    } catch (err) {
      console.error("Failed to duplicate workspace:", err);
    }
  };

  const handleToggle = async (id: string, enabled: boolean) => {
    try {
      const updated = await api.toggleWorkspace(id, enabled);
      setWorkspaces((prev) =>
        prev.map((w) => (w.id === updated.id ? updated : w))
      );
    } catch (err) {
      console.error("Failed to toggle workspace:", err);
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await api.deleteWorkspace(id);
      setWorkspaces((prev) => prev.filter((w) => w.id !== id));
    } catch (err) {
      console.error("Failed to delete workspace:", err);
    }
  };

  const handleLaunch = async (id: string) => {
    try {
      const result = await api.launchWorkspace(id);
      if (!result.success) {
        const failedApps = result.app_results
          .filter((r) => r.status.type === "failed")
          .map((r) => r.app_name);
        if (failedApps.length > 0) {
          throw new Error(`Failed to launch: ${failedApps.join(", ")}`);
        }
      }
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      console.error("Launch error:", message);
      throw message;
    }
  };

  const filteredWorkspaces = workspaces.filter(
    (w) =>
      w.name.toLowerCase().includes(search.toLowerCase()) ||
      w.description.toLowerCase().includes(search.toLowerCase()) ||
      (w.shortcut && w.shortcut.toLowerCase().includes(search.toLowerCase()))
  );

  return (
    <div className="flex h-screen bg-black text-zinc-100 font-sans antialiased select-none overflow-hidden">
      {/* Sidebar Navigation */}
      <Sidebar
        activeTab={activeTab}
        onSelectTab={setActiveTab}
        workspaceCount={workspaces.length}
      />

      {/* Main Content Area */}
      <main className="flex-1 flex flex-col h-screen overflow-hidden">
