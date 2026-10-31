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
