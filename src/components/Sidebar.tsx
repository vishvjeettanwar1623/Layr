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
