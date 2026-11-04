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
        {/* Top Header Bar */}
        <header className="h-14 px-6 border-b border-zinc-800/80 flex items-center justify-between shrink-0 bg-zinc-950/40">
          <div className="flex items-center gap-3 w-80">
            <div className="relative w-full">
              <svg
                className="w-4 h-4 text-zinc-500 absolute left-3 top-2.5"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
                />
              </svg>
              <input
                type="text"
                placeholder="Search workspaces (name, shortcut)..."
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                className="w-full bg-zinc-900 border border-zinc-800 rounded-md pl-9 pr-3 py-1.5 text-xs text-zinc-100 placeholder-zinc-600 focus:outline-none focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500 transition-colors"
              />
            </div>
          </div>

          <div className="flex items-center gap-2.5">
            <Button
              variant="primary"
              size="sm"
              onClick={handleCreateNew}
              className="gap-1.5"
            >
              <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2.5} d="M12 4v16m8-8H4" />
              </svg>
              <span>New Workspace</span>
            </Button>
          </div>
        </header>

        {/* Tab View */}
        <div className="flex-1 overflow-y-auto p-6">
          {activeTab === "workspaces" && (
            <div>
              <div className="mb-6 flex items-center justify-between">
                <div>
                  <h1 className="text-xl font-semibold tracking-tight text-zinc-100">
                    Saved Workspaces
                  </h1>
                  <p className="text-xs text-zinc-500 mt-1">
                    Manage layouts, application presets, and global launch bindings.
                  </p>
                </div>
              </div>

              {loading ? (
                <div className="h-64 flex items-center justify-center text-xs text-zinc-500">
                  Loading workspaces...
                </div>
              ) : filteredWorkspaces.length === 0 ? (
                <div className="h-80 border border-dashed border-zinc-800/80 rounded-xl flex flex-col items-center justify-center text-center p-6 bg-zinc-950/20">
                  <div className="w-10 h-10 rounded-full bg-zinc-900 border border-zinc-800 flex items-center justify-center text-zinc-400 mb-3">
                    <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.8} d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
                    </svg>
                  </div>
                  <h3 className="text-sm font-semibold text-zinc-200">
                    {search ? "No matching workspaces" : "No workspaces saved yet"}
                  </h3>
                  <p className="text-xs text-zinc-500 max-w-sm mt-1 mb-4">
                    {search
                      ? "Try searching for a different keyword or clear your query."
                      : "Create your first workspace to group apps, configure window placement, and assign a shortcut."}
                  </p>
                  {!search && (
                    <Button variant="primary" size="sm" onClick={handleCreateNew}>
                      Create Workspace
                    </Button>
                  )}
                </div>
              ) : (
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                  {filteredWorkspaces.map((ws) => (
                    <WorkspaceCard
                      key={ws.id}
                      workspace={ws}
                      onEdit={handleEdit}
                      onDuplicate={handleDuplicate}
                      onToggle={handleToggle}
                      onDelete={handleDelete}
                      onLaunch={handleLaunch}
                    />
                  ))}
                </div>
              )}
            </div>
          )}

          {activeTab === "applications" && (
            <div className="h-64 flex flex-col items-center justify-center text-center p-6 border border-dashed border-zinc-800 rounded-xl">
              <span className="text-sm font-medium text-zinc-300">
                Application Picker & Presets
              </span>
              <span className="text-xs text-zinc-500 mt-1">
                Coming in Stage 2: automatic scanning of Start Menu, App Paths, and registry presets.
              </span>
            </div>
          )}

          {activeTab === "settings" && (
            <SettingsView
              onWorkspacesImported={(imported) => {
                setWorkspaces(imported);
              }}
            />
          )}
        </div>
      </main>

      {/* Workspace Editor Modal */}
      <WorkspaceEditor
        open={editorOpen}
        onClose={() => setEditorOpen(false)}
        onSave={handleSaveWorkspace}
        initialWorkspace={editingWorkspace}
      />
    </div>
  );
}

export default App;
