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
