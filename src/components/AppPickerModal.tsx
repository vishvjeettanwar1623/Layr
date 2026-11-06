import { useState, useEffect } from "react";
import { AppEntry, DiscoveredApp } from "../types/workspace";
import * as api from "../api/workspaces";
import { Dialog } from "./ui/Dialog";
import { Input } from "./ui/Input";
import { Button } from "./ui/Button";

interface AppPickerModalProps {
  open: boolean;
  onClose: () => void;
  onSelectApp: (app: AppEntry) => void;
}

export const AppPickerModal: React.FC<AppPickerModalProps> = ({
  open,
  onClose,
  onSelectApp,
}) => {
  const [tab, setTab] = useState<"presets" | "installed" | "manual">("presets");
  const [presets, setPresets] = useState<DiscoveredApp[]>([]);
  const [installed, setInstalled] = useState<DiscoveredApp[]>([]);
  const [loading, setLoading] = useState(false);
  const [search, setSearch] = useState("");

  // Manual entry fields
  const [manualName, setManualName] = useState("");
  const [manualPath, setManualPath] = useState("");
  const [manualSupportsUrl, setManualSupportsUrl] = useState(false);

  useEffect(() => {
    if (open) {
      loadData();
    } else {
      // Discard loaded data on close to keep RAM minimal
      setPresets([]);
      setInstalled([]);
      setSearch("");
      setManualName("");
      setManualPath("");
    }
  }, [open, tab]);

  const loadData = async () => {
    try {
      setLoading(true);
      if (tab === "presets" && presets.length === 0) {
        const data = await api.getPresets();
        setPresets(data);
      } else if (tab === "installed" && installed.length === 0) {
        const data = await api.discoverApps();
        setInstalled(data);
      }
    } catch (err) {
      console.error("Failed to load apps:", err);
    } finally {
      setLoading(false);
    }
  };

  const handleSelectDiscovered = (app: DiscoveredApp) => {
    const entry: AppEntry = {
      id: crypto.randomUUID(),
      name: app.name,
      executable_path: app.executable_path,
      arguments: [],
      working_directory: undefined,
      url: undefined,
      supports_url: app.supports_url,
      launch_order: 0,
      launch_delay_ms: 0,
      duplicate_policy: "always_launch",
    };
    onSelectApp(entry);
    onClose();
  };

  const handleAddManual = (e: React.FormEvent) => {
    e.preventDefault();
    if (!manualName.trim() || !manualPath.trim()) return;

    const entry: AppEntry = {
      id: crypto.randomUUID(),
      name: manualName.trim(),
      executable_path: manualPath.trim(),
      arguments: [],
      working_directory: undefined,
      url: undefined,
      supports_url: manualSupportsUrl,
      launch_order: 0,
      launch_delay_ms: 0,
      duplicate_policy: "always_launch",
    };
    onSelectApp(entry);
    onClose();
  };

  const list = tab === "presets" ? presets : installed;
  const filtered = list.filter((a) =>
    a.name.toLowerCase().includes(search.toLowerCase())
  );

  return (
    <Dialog
      open={open}
      onClose={onClose}
      title="Add Application"
      description="Select an application preset or choose from installed system apps."
    >
      <div className="space-y-4">
        {/* Tab buttons */}
        <div className="flex border-b border-zinc-800/80 pb-2 gap-2 text-xs font-medium">
          <button
            onClick={() => setTab("presets")}
            className={`px-3 py-1.5 rounded-md transition-colors ${
              tab === "presets"
                ? "bg-zinc-800 text-emerald-400 border border-zinc-750"
                : "text-zinc-400 hover:text-zinc-200"
            }`}
          >
            Presets
          </button>
          <button
            onClick={() => setTab("installed")}
            className={`px-3 py-1.5 rounded-md transition-colors ${
              tab === "installed"
                ? "bg-zinc-800 text-emerald-400 border border-zinc-750"
                : "text-zinc-400 hover:text-zinc-200"
            }`}
          >
            Start Menu Apps
          </button>
          <button
            onClick={() => setTab("manual")}
            className={`px-3 py-1.5 rounded-md transition-colors ${
              tab === "manual"
                ? "bg-zinc-800 text-emerald-400 border border-zinc-750"
                : "text-zinc-400 hover:text-zinc-200"
            }`}
          >
            Custom Path
          </button>
        </div>

        {tab !== "manual" ? (
          <div>
            <div className="mb-3">
              <Input
                placeholder="Search applications..."
                value={search}
                onChange={(e) => setSearch(e.target.value)}
              />
            </div>

            <div className="max-h-60 overflow-y-auto space-y-1.5 pr-1">
              {loading ? (
                <div className="py-8 text-center text-xs text-zinc-500">
                  Scanning applications...
                </div>
              ) : filtered.length === 0 ? (
                <div className="py-8 text-center text-xs text-zinc-500">
                  No applications found matching your query.
                </div>
              ) : (
                filtered.map((app, idx) => (
                  <button
                    key={idx}
                    onClick={() => handleSelectDiscovered(app)}
                    className="w-full text-left p-2.5 rounded-md bg-zinc-900/60 hover:bg-zinc-800/80 border border-zinc-800 hover:border-zinc-700 flex items-center justify-between transition-colors group"
                  >
                    <div>
                      <div className="text-xs font-semibold text-zinc-200 group-hover:text-emerald-400 transition-colors">
                        {app.name}
                      </div>
                      <div className="text-[10px] font-mono text-zinc-500 truncate max-w-sm">
                        {app.executable_path}
                      </div>
                    </div>
                    {app.supports_url && (
                      <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-emerald-950/60 text-emerald-400 border border-emerald-800/50">
                        Browser
                      </span>
                    )}
                  </button>
                ))
              )}
            </div>
          </div>
        ) : (
          <form onSubmit={handleAddManual} className="space-y-3">
            <Input
              label="Application Name"
              placeholder="e.g. My Custom Tool"
              value={manualName}
              onChange={(e) => setManualName(e.target.value)}
              required
            />
            <Input
              label="Executable Path"
              placeholder="C:\Tools\app.exe"
              value={manualPath}
              onChange={(e) => setManualPath(e.target.value)}
              required
            />
            <div className="flex items-center gap-2 pt-1">
              <input
                type="checkbox"
                id="supportsUrl"
                checked={manualSupportsUrl}
                onChange={(e) => setManualSupportsUrl(e.target.checked)}
                className="rounded border-zinc-800 text-emerald-500 focus:ring-emerald-500/40 bg-zinc-900"
              />
              <label htmlFor="supportsUrl" className="text-xs text-zinc-400 select-none">
                This application accepts URL / browser arguments
              </label>
            </div>
            <div className="pt-2 flex justify-end gap-2">
              <Button type="button" variant="secondary" onClick={onClose}>
                Cancel
              </Button>
              <Button type="submit" variant="primary">
                Add Application
              </Button>
            </div>
          </form>
        )}
      </div>
    </Dialog>
  );
};

