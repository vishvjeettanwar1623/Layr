import React from "react";

export type NavTab = "workspaces" | "applications" | "settings";

interface SidebarProps {
  activeTab: NavTab;
  onSelectTab: (tab: NavTab) => void;
  workspaceCount: number;
}

export const Sidebar: React.FC<SidebarProps> = ({
  activeTab,
  onSelectTab,
  workspaceCount,
}) => {
  return (
    <aside className="w-56 bg-zinc-950 border-r border-zinc-800/80 flex flex-col justify-between shrink-0 h-screen select-none">
      <div>
        {/* App Title / Header */}
        <div className="h-14 flex items-center gap-2.5 px-4 border-b border-zinc-800/80">
          <div className="w-6 h-6 rounded bg-emerald-600/20 border border-emerald-500/40 flex items-center justify-center">
            <span className="text-emerald-400 font-bold text-xs">L</span>
          </div>
          <span className="font-semibold text-sm tracking-tight text-zinc-100">
            Layr
          </span>
          <span className="ml-auto text-[10px] uppercase font-mono px-1.5 py-0.5 rounded bg-zinc-850 text-zinc-400 border border-zinc-750">
            v0.1
          </span>
        </div>

        {/* Navigation list */}
        <nav className="p-2 space-y-1">
          <button
            onClick={() => onSelectTab("workspaces")}
            className={`w-full flex items-center justify-between px-3 py-2 rounded-md text-xs font-medium transition-colors ${
              activeTab === "workspaces"
                ? "bg-zinc-850 text-emerald-400 border border-zinc-750"
                : "text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900"
            }`}
          >
            <div className="flex items-center gap-2.5">
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.8} d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
              </svg>
              <span>Workspaces</span>
            </div>
            <span className="text-[11px] font-mono px-1.5 py-0.2 rounded bg-zinc-900 text-zinc-400 border border-zinc-800">
              {workspaceCount}
            </span>
          </button>

          <button
            onClick={() => onSelectTab("applications")}
            className={`w-full flex items-center gap-2.5 px-3 py-2 rounded-md text-xs font-medium transition-colors ${
              activeTab === "applications"
                ? "bg-zinc-850 text-emerald-400 border border-zinc-750"
                : "text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900"
            }`}
          >
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.8} d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z" />
            </svg>
            <span>Applications</span>
          </button>

          <button
            onClick={() => onSelectTab("settings")}
            className={`w-full flex items-center gap-2.5 px-3 py-2 rounded-md text-xs font-medium transition-colors ${
              activeTab === "settings"
                ? "bg-zinc-850 text-emerald-400 border border-zinc-750"
                : "text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900"
            }`}
          >
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.8} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.8} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
            <span>Settings</span>
          </button>
        </nav>
      </div>

      {/* Footer / System status */}
      <div className="p-3 border-t border-zinc-850">
        <div className="flex items-center justify-between text-[11px] text-zinc-500">
          <span className="flex items-center gap-1.5">
            <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse" />
            Tray active
          </span>
          <span className="font-mono text-[10px]">Ctrl+Alt+9</span>
        </div>
      </div>
    </aside>
  );
};

