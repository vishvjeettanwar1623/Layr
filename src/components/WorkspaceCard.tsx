import { useState } from "react";
import { Workspace } from "../types/workspace";
import { Card } from "./ui/Card";
import { Toggle } from "./ui/Toggle";
import { Dropdown } from "./ui/Dropdown";
import { Button } from "./ui/Button";

interface WorkspaceCardProps {
  workspace: Workspace;
  onEdit: (workspace: Workspace) => void;
  onDuplicate: (id: string) => void;
  onToggle: (id: string, enabled: boolean) => void;
  onDelete: (id: string) => void;
  onLaunch: (id: string) => Promise<void>;
}

export const WorkspaceCard: React.FC<WorkspaceCardProps> = ({
  workspace,
  onEdit,
  onDuplicate,
  onToggle,
  onDelete,
  onLaunch,
}) => {
  const [launching, setLaunching] = useState(false);
  const [launchStatus, setLaunchStatus] = useState<string | null>(null);

  const handleLaunch = async () => {
    if (!workspace.enabled || launching) return;
    try {
      setLaunching(true);
      setLaunchStatus("Launching...");
      await onLaunch(workspace.id);
      setLaunchStatus("Launched!");
      setTimeout(() => setLaunchStatus(null), 3000);
    } catch (err: unknown) {
      setLaunchStatus(typeof err === "string" ? err : "Failed to launch");
      setTimeout(() => setLaunchStatus(null), 4000);
    } finally {
      setLaunching(false);
    }
  };

  return (
    <Card className="flex flex-col justify-between group relative min-h-[12rem]">
      {/* Top row: Icon, Name, More actions */}
      <div>
        <div className="flex items-start justify-between">
          <div className="flex items-center gap-2.5">
            <div className="w-8 h-8 rounded bg-zinc-800/80 border border-zinc-700/60 flex items-center justify-center text-sm font-semibold text-emerald-400">
              {workspace.icon.slice(0, 2).toUpperCase() || "WS"}
            </div>
            <div>
              <h3 className="text-sm font-semibold text-zinc-100 group-hover:text-emerald-300 transition-colors">
                {workspace.name}
              </h3>
              <p className="text-[11px] text-zinc-500 line-clamp-1">
                {workspace.description || "No description"}
              </p>
            </div>
          </div>

          <Dropdown
            trigger={
              <button
                className="text-zinc-500 hover:text-zinc-300 p-1 rounded hover:bg-zinc-800 transition-colors"
                aria-label="Workspace options"
              >
                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z" />
                </svg>
              </button>
            }
            items={[
              { label: "Edit Workspace", onClick: () => onEdit(workspace) },
              { label: "Duplicate", onClick: () => onDuplicate(workspace.id) },
              {
                label: "Delete",
                onClick: () => onDelete(workspace.id),
                danger: true,
              },
            ]}
          />
        </div>

        {/* Mid row: Apps summary & Shortcut */}
        <div className="mt-4 flex items-center gap-2">
          {workspace.shortcut ? (
            <kbd className="px-2 py-0.5 rounded bg-zinc-800 border border-zinc-700 font-mono text-[11px] text-emerald-400 font-medium">
              {workspace.shortcut}
            </kbd>
          ) : (
            <span className="text-[11px] text-zinc-600 font-mono">No shortcut</span>
          )}

          <span className="text-xs text-zinc-600">•</span>

          <span className="text-xs text-zinc-400">
            {workspace.apps.length} {workspace.apps.length === 1 ? "app" : "apps"}
          </span>
        </div>

        {launchStatus && (
          <div className="mt-2 text-[11px] font-mono text-emerald-400 transition-opacity">
            {launchStatus}
          </div>
        )}
      </div>

      {/* Bottom row: Enabled Toggle + Quick Launch button */}
      <div className="pt-3 border-t border-zinc-800/60 flex items-center justify-between">
        <Toggle
          checked={workspace.enabled}
          onChange={(checked) => onToggle(workspace.id, checked)}
          label={workspace.enabled ? "Active" : "Disabled"}
        />

        <div className="flex items-center gap-2">
          <Button
            size="sm"
            variant="primary"
            disabled={!workspace.enabled || launching || workspace.apps.length === 0}
            onClick={handleLaunch}
            className="text-[11px] py-1 px-2.5 gap-1"
          >
            <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2.5} d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" />
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            <span>Launch</span>
          </Button>
        </div>
      </div>
    </Card>
  );
};
