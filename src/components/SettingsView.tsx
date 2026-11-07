import React, { useState, useEffect } from "react";
import { AppSettings, Workspace } from "../types/workspace";
import * as api from "../api/workspaces";
import { Toggle } from "./ui/Toggle";
import { Button } from "./ui/Button";

interface SettingsViewProps {
  onWorkspacesImported?: (workspaces: Workspace[]) => void;
}

export const SettingsView: React.FC<SettingsViewProps> = ({ onWorkspacesImported }) => {
  const [settings, setSettings] = useState<AppSettings>({
    autostart: false,
    default_strategy: "sequential",
    default_window_timeout_secs: 6,
    enable_notifications: true,
  });
  const [loading, setLoading] = useState(true);
  const [savedMessage, setSavedMessage] = useState("");
  const [errorMessage, setErrorMessage] = useState("");
  const fileInputRef = React.useRef<HTMLInputElement>(null);

  useEffect(() => {
    api
      .getSettings()
      .then((s) => {
        setSettings(s);
        setLoading(false);
      })
      .catch((err) => {
        console.error("Failed to load settings:", err);
        setLoading(false);
      });
  }, []);

  const handleUpdate = async (updates: Partial<AppSettings>) => {
    const updated = { ...settings, ...updates };
    setSettings(updated);
    setErrorMessage("");
    setSavedMessage("");

    try {
      await api.updateSettings(updated);
      setSavedMessage("Settings saved successfully.");
      setTimeout(() => setSavedMessage(""), 3000);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setErrorMessage(`Failed to save settings: ${msg}`);
    }
  };

  const handleExport = async () => {
    try {
      const json = await api.exportConfig();
      const blob = new Blob([json], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `layr-config-${new Date().toISOString().slice(0, 10)}.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      setSavedMessage("Configuration exported successfully.");
      setTimeout(() => setSavedMessage(""), 3000);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setErrorMessage(`Export failed: ${msg}`);
    }
  };

  const handleImport = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = async (evt) => {
      try {
        const content = evt.target?.result as string;
        const imported = await api.importConfig(content);
        onWorkspacesImported?.(imported);
        setSavedMessage(`Imported ${imported.length} workspaces successfully.`);
        setTimeout(() => setSavedMessage(""), 4000);
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err);
        setErrorMessage(`Import failed: ${msg}`);
      }
    };
    reader.readAsText(file);
    e.target.value = "";
  };

  if (loading) {
    return (
      <div className="flex-1 flex items-center justify-center text-zinc-500 font-mono text-xs">
        Loading preferences...
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto p-8 max-w-3xl space-y-8 select-none">
      {/* Header */}
      <div>
        <h1 className="text-xl font-bold tracking-tight text-zinc-100">Settings</h1>
        <p className="text-xs text-zinc-500 mt-1">
          System startup, default launcher behavior, and configuration backup.
        </p>
      </div>

      {savedMessage && (
        <div className="p-3 bg-emerald-950/40 border border-emerald-800/80 rounded-lg text-emerald-400 text-xs">
          {savedMessage}
        </div>
      )}

      {errorMessage && (
        <div className="p-3 bg-red-950/40 border border-red-800/80 rounded-lg text-red-400 text-xs">
          {errorMessage}
        </div>
      )}

      {/* Startup & System */}
      <section className="space-y-4">
        <h2 className="text-xs font-semibold uppercase tracking-wider text-zinc-400">
          Startup & Notifications
        </h2>

        <div className="p-4 bg-zinc-900/50 border border-zinc-800 rounded-lg space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <div className="text-sm font-medium text-zinc-200">Launch at Windows Login</div>
              <div className="text-xs text-zinc-500 mt-0.5">
                Starts Layr automatically in the background (tray only, zero window overhead).
              </div>
            </div>
            <Toggle
              checked={settings.autostart}
              onChange={(val) => handleUpdate({ autostart: val })}
            />
          </div>

          <div className="border-t border-zinc-800/80 pt-4 flex items-center justify-between">
            <div>
              <div className="text-sm font-medium text-zinc-200">Desktop Notifications</div>
              <div className="text-xs text-zinc-500 mt-0.5">
                Displays native toast notifications when launching workspaces or on warnings.
              </div>
            </div>
            <Toggle
              checked={settings.enable_notifications}
              onChange={(val) => handleUpdate({ enable_notifications: val })}
            />
          </div>
        </div>
      </section>

      {/* Execution Defaults */}
      <section className="space-y-4">
        <h2 className="text-xs font-semibold uppercase tracking-wider text-zinc-400">
          Launcher Engine
        </h2>

        <div className="p-4 bg-zinc-900/50 border border-zinc-800 rounded-lg space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <div className="text-sm font-medium text-zinc-200">Default Execution Strategy</div>
              <div className="text-xs text-zinc-500 mt-0.5">
                Sequential ensures stable window discovery; Parallel dispatches all apps simultaneously.
              </div>
            </div>
            <select
              value={settings.default_strategy}
              onChange={(e) =>
                handleUpdate({ default_strategy: e.target.value as "sequential" | "parallel" })
              }
              className="bg-zinc-950 border border-zinc-800 rounded px-3 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-emerald-500"
            >
              <option value="sequential">Sequential (Ordered)</option>
              <option value="parallel">Parallel (Simultaneous)</option>
            </select>
          </div>

          <div className="border-t border-zinc-800/80 pt-4 flex items-center justify-between">
            <div>
              <div className="text-sm font-medium text-zinc-200">Window Detection Timeout</div>
              <div className="text-xs text-zinc-500 mt-0.5">
                Maximum seconds to wait for a spawned app's main window before releasing.
              </div>
            </div>
            <div className="flex items-center gap-2">
              <input
                type="number"
                min={2}
                max={30}
                value={settings.default_window_timeout_secs}
                onChange={(e) =>
                  handleUpdate({
                    default_window_timeout_secs: Math.max(2, parseInt(e.target.value) || 6),
                  })
                }
                className="w-20 bg-zinc-950 border border-zinc-800 rounded px-2.5 py-1 text-xs text-zinc-200 font-mono text-center focus:outline-none focus:border-emerald-500"
              />
              <span className="text-xs text-zinc-500">sec</span>
            </div>
          </div>
        </div>
      </section>

      {/* Backup and Restore */}
      <section className="space-y-4">
        <h2 className="text-xs font-semibold uppercase tracking-wider text-zinc-400">
          Backup & Portability
        </h2>

        <div className="p-4 bg-zinc-900/50 border border-zinc-800 rounded-lg flex flex-col sm:flex-row items-center justify-between gap-4">
          <div>
            <div className="text-sm font-medium text-zinc-200">Export / Import Configuration</div>
            <div className="text-xs text-zinc-500 mt-0.5">
              Export workspaces, app presets, and positions to a standalone JSON file.
            </div>
          </div>

          <div className="flex items-center gap-2.5 w-full sm:w-auto">
            <Button type="button" variant="secondary" size="sm" onClick={handleExport}>
              Export JSON
            </Button>
            <Button
              type="button"
              variant="primary"
              size="sm"
              onClick={() => fileInputRef.current?.click()}
            >
              Import JSON
            </Button>
            <input
              ref={fileInputRef}
              type="file"
              accept=".json,application/json"
              onChange={handleImport}
              className="hidden"
            />
          </div>
        </div>
      </section>

      {/* System Specifications & Telemetry Guarantee */}
      <section className="p-4 rounded-lg border border-zinc-900 bg-zinc-950/60 text-xs text-zinc-500 space-y-2">
        <div className="flex items-center justify-between text-[11px] font-mono">
          <span>Layr v0.1.0 (Windows 11 x86_64)</span>
          <span className="text-emerald-500/90 font-semibold">Zero Telemetry • Pure Local</span>
        </div>
        <p className="text-[11px] leading-relaxed text-zinc-600">
          Layr uses an ultra-low-overhead Rust runtime with webview destruction on window minimize.
          All coordinates, shortcuts, and configurations are stored in an atomic local JSON ledger.
        </p>
      </section>
    </div>
  );
};
