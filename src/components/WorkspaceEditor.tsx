import React, { useState, useEffect, lazy, Suspense } from "react";
import { AppEntry, DuplicatePolicy, Workspace } from "../types/workspace";
import { Dialog } from "./ui/Dialog";
import { Input } from "./ui/Input";
import { Button } from "./ui/Button";
import { Toggle } from "./ui/Toggle";
import { AppPickerModal } from "./AppPickerModal";

const VisualLayoutEditor = lazy(() => import("./VisualLayoutEditor"));

interface WorkspaceEditorProps {
  open: boolean;
  onClose: () => void;
  onSave: (workspace: Workspace) => void;
  initialWorkspace?: Workspace | null;
}

export const WorkspaceEditor: React.FC<WorkspaceEditorProps> = ({
  open,
  onClose,
  onSave,
  initialWorkspace,
}) => {
  const [editorTab, setEditorTab] = useState<"details" | "layout">("details");
  const [name, setName] = useState("");
  const [icon, setIcon] = useState("WS");
  const [description, setDescription] = useState("");
  const [shortcut, setShortcut] = useState("");
  const [enabled, setEnabled] = useState(true);
  const [apps, setApps] = useState<AppEntry[]>([]);
  const [nameError, setNameError] = useState("");
  const [pickerOpen, setPickerOpen] = useState(false);
  const [selectedAppForLayout, setSelectedAppForLayout] = useState<string | undefined>();

  useEffect(() => {
    if (initialWorkspace) {
      setName(initialWorkspace.name);
      setIcon(initialWorkspace.icon || "WS");
      setDescription(initialWorkspace.description || "");
      setShortcut(initialWorkspace.shortcut || "");
      setEnabled(initialWorkspace.enabled);
      setApps(initialWorkspace.apps || []);
    } else {
      setName("");
      setIcon("WS");
      setDescription("");
      setShortcut("");
      setEnabled(true);
      setApps([]);
    }
    setNameError("");
  }, [initialWorkspace, open]);

  const handleAddApp = (newApp: AppEntry) => {
    setApps((prev) => [
      ...prev,
      { ...newApp, launch_order: prev.length },
    ]);
  };

  const handleRemoveApp = (id: string) => {
    setApps((prev) =>
      prev
        .filter((a) => a.id !== id)
        .map((a, idx) => ({ ...a, launch_order: idx }))
    );
  };

  const handleMoveApp = (index: number, direction: "up" | "down") => {
    const targetIndex = direction === "up" ? index - 1 : index + 1;
    if (targetIndex < 0 || targetIndex >= apps.length) return;

    const newApps = [...apps];
    const temp = newApps[index];
    newApps[index] = newApps[targetIndex];
    newApps[targetIndex] = temp;

    setApps(newApps.map((a, idx) => ({ ...a, launch_order: idx })));
  };

  const handleUpdateApp = (id: string, updates: Partial<AppEntry>) => {
    setApps((prev) =>
      prev.map((a) => (a.id === id ? { ...a, ...updates } : a))
    );
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) {
      setNameError("Workspace name is required");
      return;
    }

    const now = Math.floor(Date.now() / 1000);
    const workspace: Workspace = {
      id: initialWorkspace?.id || crypto.randomUUID(),
      name: name.trim(),
      icon: icon.trim() || "WS",
      description: description.trim(),
      shortcut: shortcut.trim() ? shortcut.trim() : undefined,
      enabled,
      apps,
      created_at: initialWorkspace?.created_at || now,
      updated_at: now,
    };

    onSave(workspace);
    onClose();
  };

  return (
    <>
      <Dialog
        open={open}
        onClose={onClose}
        title={initialWorkspace ? "Edit Workspace" : "New Workspace"}
        description="Configure apps, arguments, launch delays, and instance policies."
      >
        {/* Tab switcher: Apps & Settings vs Visual Layout */}
        <div className="flex border-b border-zinc-800 mb-3 gap-3">
          <button
            type="button"
            onClick={() => setEditorTab("details")}
            className={`pb-2 text-xs font-medium transition-colors border-b-2 -mb-px ${
              editorTab === "details"
                ? "border-emerald-500 text-emerald-400"
                : "border-transparent text-zinc-400 hover:text-zinc-200"
            }`}
          >
            Applications & Settings
          </button>
          <button
            type="button"
            onClick={() => setEditorTab("layout")}
            className={`pb-2 text-xs font-medium transition-colors border-b-2 -mb-px flex items-center gap-1.5 ${
              editorTab === "layout"
                ? "border-emerald-500 text-emerald-400"
                : "border-transparent text-zinc-400 hover:text-zinc-200"
            }`}
          >
            <span>Visual Layout Canvas</span>
            {apps.length > 0 && (
              <span className="text-[10px] px-1.5 py-0.2 bg-zinc-800 text-zinc-300 rounded font-mono">
                {apps.length}
              </span>
            )}
          </button>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4 max-h-[75vh] overflow-y-auto pr-1">
          {editorTab === "layout" ? (
            <Suspense
              fallback={
                <div className="p-12 text-center text-xs text-zinc-500 font-mono">
                  Loading visual layout editor...
                </div>
              }
            >
              <VisualLayoutEditor
                apps={apps}
                onChangeApps={setApps}
                selectedAppId={selectedAppForLayout}
                onSelectApp={setSelectedAppForLayout}
              />
            </Suspense>
          ) : (
            <>
              {/* Identity details */}
              <div className="flex gap-3">
                <div className="w-20">
                  <Input
                    label="Icon"
                    value={icon}
                    maxLength={4}
                    onChange={(e) => setIcon(e.target.value)}
                    placeholder="DEV"
                    className="font-mono uppercase text-center"
                  />
                </div>
                <div className="flex-1">
                  <Input
                    label="Workspace Name"
                    value={name}
                    onChange={(e) => {
                      setName(e.target.value);
                      if (nameError) setNameError("");
                    }}
                    error={nameError}
                    placeholder="e.g. Code & Research"
                    autoFocus
                  />
                </div>
              </div>

              <Input
                label="Description (optional)"
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                placeholder="Brief summary of this layout"
              />

              <Input
                label="Global Shortcut (optional)"
                value={shortcut}
                onChange={(e) => setShortcut(e.target.value)}
                placeholder="e.g. Ctrl+Alt+1"
              />

              {/* Applications list */}
              <div className="pt-2 border-t border-zinc-800">
                <div className="flex items-center justify-between mb-2">
                  <span className="text-xs font-semibold text-zinc-300">
                    Applications ({apps.length})
                  </span>
                  <Button
                    type="button"
                    variant="secondary"
                    size="sm"
                    onClick={() => setPickerOpen(true)}
                    className="text-[11px] py-1 px-2.5 gap-1"
                  >
                    <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2.5} d="M12 4v16m8-8H4" />
                    </svg>
                    <span>Add App</span>
                  </Button>
                </div>

                {apps.length === 0 ? (
                  <div className="p-4 rounded-md border border-dashed border-zinc-800 text-center text-xs text-zinc-500 bg-zinc-900/30">
                    No apps added to this workspace yet. Click "Add App" above.
                  </div>
                ) : (
                  <div className="space-y-3">
                    {apps.map((app, index) => (
                      <div
                        key={app.id}
                        className="p-3 rounded-lg bg-zinc-900/80 border border-zinc-800 space-y-2.5 text-xs"
                      >
                        {/* Header: Name, Reorder, Delete */}
                        <div className="flex items-center justify-between">
                          <div className="flex items-center gap-2">
                            <span className="font-mono text-[10px] text-zinc-500 w-4">
                              #{index + 1}
                            </span>
                            <span className="font-medium text-zinc-200">{app.name}</span>
                          </div>

                          <div className="flex items-center gap-1">
                            <button
                              type="button"
                              onClick={() => {
                                setSelectedAppForLayout(app.id);
                                setEditorTab("layout");
                              }}
                              className="px-2 py-0.5 text-[10px] bg-zinc-800 hover:bg-zinc-700 text-emerald-400 rounded transition-colors"
                              title="Visual Placement"
                            >
                              Layout
                            </button>
                            <button
                              type="button"
                              disabled={index === 0}
                              onClick={() => handleMoveApp(index, "up")}
                              className="p-1 text-zinc-500 hover:text-zinc-200 disabled:opacity-20 transition-colors"
                              title="Move up"
                            >
                              <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 15l7-7 7 7" />
                              </svg>
                            </button>
                            <button
                              type="button"
                              disabled={index === apps.length - 1}
                              onClick={() => handleMoveApp(index, "down")}
                              className="p-1 text-zinc-500 hover:text-zinc-200 disabled:opacity-20 transition-colors"
                              title="Move down"
                            >
                              <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                              </svg>
                            </button>
                            <button
                              type="button"
                              onClick={() => handleRemoveApp(app.id)}
                              className="p-1 text-red-400 hover:text-red-300 transition-colors"
                              title="Remove app"
                            >
                              <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                              </svg>
                            </button>
                          </div>
                        </div>

                        {/* App Path */}
                        <div className="font-mono text-[10px] text-zinc-500 truncate">
                          {app.executable_path}
                        </div>

                        {/* Delay & Instance Policy row */}
                        <div className="grid grid-cols-2 gap-2 pt-1">
                          <div>
                            <label className="text-[10px] text-zinc-400 block mb-1">
                              Launch Delay (ms)
                            </label>
                            <input
                              type="number"
                              min={0}
                              step={100}
                              value={app.launch_delay_ms}
                              onChange={(e) =>
                                handleUpdateApp(app.id, {
                                  launch_delay_ms: Math.max(0, parseInt(e.target.value) || 0),
                                })
                              }
                              className="w-full bg-zinc-950 border border-zinc-800 rounded px-2 py-1 text-xs text-zinc-200 focus:outline-none focus:border-emerald-500"
                            />
                          </div>

                          <div>
                            <label className="text-[10px] text-zinc-400 block mb-1">
                              If Already Running
                            </label>
                            <select
                              value={app.duplicate_policy}
                              onChange={(e) =>
                                handleUpdateApp(app.id, {
                                  duplicate_policy: e.target.value as DuplicatePolicy,
                                })
                              }
                              className="w-full bg-zinc-950 border border-zinc-800 rounded px-2 py-1 text-xs text-zinc-200 focus:outline-none focus:border-emerald-500"
                            >
                              <option value="always_launch">Always Launch New</option>
                              <option value="reuse_existing_window">Reuse Window</option>
                              <option value="skip_if_running">Skip If Running</option>
                            </select>
                          </div>
                        </div>

                        {/* URL Field (conditional) */}
                        <div className="pt-1">
                          {app.supports_url ? (
                            <div>
                              <label className="text-[10px] text-zinc-400 block mb-1">
                                URL / Web Address
                              </label>
                              <input
                                type="text"
                                placeholder="https://github.com or https://linear.app"
                                value={app.url || ""}
                                onChange={(e) =>
                                  handleUpdateApp(app.id, { url: e.target.value })
                                }
                                className="w-full bg-zinc-950 border border-zinc-800 rounded px-2 py-1 text-xs text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-emerald-500"
                              />
                            </div>
                          ) : (
                            <div className="text-[10px] text-zinc-500 italic">
                              URLs not supported for this executable.
                            </div>
                          )}
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>

              <div className="pt-2 flex items-center justify-between border-t border-zinc-800">
                <Toggle
                  checked={enabled}
                  onChange={setEnabled}
                  label={enabled ? "Workspace is enabled" : "Workspace is disabled"}
                />
              </div>
            </>
          )}

          <div className="pt-3 border-t border-zinc-800 flex justify-end gap-2.5">
            <Button type="button" variant="secondary" onClick={onClose}>
              Cancel
            </Button>
            <Button type="submit" variant="primary">
              {initialWorkspace ? "Save Changes" : "Create Workspace"}
            </Button>
          </div>
        </form>
      </Dialog>

      {/* Nested App Picker Modal */}
      <AppPickerModal
        open={pickerOpen}
        onClose={() => setPickerOpen(false)}
        onSelectApp={handleAddApp}
      />
    </>
  );
};
