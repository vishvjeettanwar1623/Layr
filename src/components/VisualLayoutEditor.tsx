import React, { useState, useEffect, useRef, useCallback } from "react";
import { AppEntry, MonitorWorkArea, WindowPlacement } from "../types/workspace";
import * as api from "../api/workspaces";

interface VisualLayoutEditorProps {
  apps: AppEntry[];
  onChangeApps: (updated: AppEntry[]) => void;
  selectedAppId?: string;
  onSelectApp?: (id: string) => void;
}

interface RectPercent {
  x: number;
  y: number;
  w: number;
  h: number;
}

const DEFAULT_MONITOR: MonitorWorkArea = {
  monitor_index: 0,
  x: 0,
  y: 0,
  width: 1920,
  height: 1080,
  scale_factor: 1.0,
  is_primary: true,
};

function getAppRect(app: AppEntry): RectPercent {
  if (!app.window_config) {
    return { x: 0, y: 0, w: 50, h: 100 };
  }
  const cfg = app.window_config;
  const toPercent = (dim: typeof cfg.x, fallback: number) => {
    if (dim.type === "percent") return dim.value;
    return fallback;
  };

  return {
    x: toPercent(cfg.x, 0),
    y: toPercent(cfg.y, 0),
    w: toPercent(cfg.width, 50),
    h: toPercent(cfg.height, 100),
  };
}

function rectsOverlap(a: RectPercent, b: RectPercent): boolean {
  return (
    a.x < b.x + b.w - 0.5 &&
    a.x + a.w > b.x + 0.5 &&
    a.y < b.y + b.h - 0.5 &&
    a.y + a.h > b.y + 0.5
  );
}

export const VisualLayoutEditor: React.FC<VisualLayoutEditorProps> = ({
  apps,
  onChangeApps,
  selectedAppId,
  onSelectApp,
}) => {
  const [monitors, setMonitors] = useState<MonitorWorkArea[]>([DEFAULT_MONITOR]);
  const [activeMonitorIndex, setActiveMonitorIndex] = useState<number>(0);
  const [activeAppId, setActiveAppId] = useState<string>(
    selectedAppId || (apps.length > 0 ? apps[0].id : "")
  );

  const canvasRef = useRef<HTMLDivElement>(null);
  const [dragging, setDragging] = useState<{
    appId: string;
    startX: number;
    startY: number;
    origRect: RectPercent;
    handle: "move" | "nw" | "ne" | "sw" | "se" | "n" | "s" | "e" | "w";
  } | null>(null);

  useEffect(() => {
    api
      .getMonitors()
      .then((res) => {
        if (res && res.length > 0) {
          setMonitors(res);
        }
      })
      .catch((err) => {
        console.warn("Could not query monitors, using default:", err);
      });
  }, []);

  useEffect(() => {
    if (selectedAppId) {
      setActiveAppId(selectedAppId);
    } else if (!activeAppId && apps.length > 0) {
      setActiveAppId(apps[0].id);
    }
  }, [selectedAppId, apps, activeAppId]);

  const activeMonitor =
    monitors.find((m) => m.monitor_index === activeMonitorIndex) || monitors[0] || DEFAULT_MONITOR;

  // Apps currently assigned to the active monitor
  const monitorApps = apps.filter(
    (a) => (a.window_config?.monitor_index ?? 0) === activeMonitorIndex
  );

  // Compute overlaps and out of bounds
  const overlaps: string[] = [];
  for (let i = 0; i < monitorApps.length; i++) {
    for (let j = i + 1; j < monitorApps.length; j++) {
      const rectA = getAppRect(monitorApps[i]);
      const rectB = getAppRect(monitorApps[j]);
      if (rectsOverlap(rectA, rectB)) {
        overlaps.push(`"${monitorApps[i].name}" and "${monitorApps[j].name}" collide`);
      }
    }
  }

  const outOfBounds: string[] = [];
  for (const app of monitorApps) {
    const r = getAppRect(app);
    if (r.x + r.w > 100.1 || r.y + r.h > 100.1 || r.x < -0.1 || r.y < -0.1) {
      outOfBounds.push(`"${app.name}" extends outside monitor boundaries`);
    }
  }

  const updateAppPlacement = useCallback(
    (appId: string, updates: Partial<RectPercent> & { is_maximized?: boolean; monitor_index?: number }) => {
      const updated = apps.map((app) => {
        if (app.id !== appId) return app;
        const current = getAppRect(app);
        const nextX = Math.round(updates.x !== undefined ? updates.x : current.x);
        const nextY = Math.round(updates.y !== undefined ? updates.y : current.y);
        const nextW = Math.max(10, Math.round(updates.w !== undefined ? updates.w : current.w));
        const nextH = Math.max(10, Math.round(updates.h !== undefined ? updates.h : current.h));

        const newPlacement: WindowPlacement = {
          monitor_index:
            updates.monitor_index !== undefined
              ? updates.monitor_index
              : app.window_config?.monitor_index ?? activeMonitorIndex,
          x: { type: "percent", value: nextX },
          y: { type: "percent", value: nextY },
          width: { type: "percent", value: nextW },
          height: { type: "percent", value: nextH },
          is_maximized: updates.is_maximized ?? app.window_config?.is_maximized ?? false,
        };

        return { ...app, window_config: newPlacement };
      });

      onChangeApps(updated);
    },
    [apps, activeMonitorIndex, onChangeApps]
  );

  // Template layouts
  const applyTemplate = (template: "50_50" | "65_35" | "thirds" | "quadrants" | "maximized") => {
    if (monitorApps.length === 0) return;

    const newApps = [...apps];
    monitorApps.forEach((app, idx) => {
      let rect: RectPercent = { x: 0, y: 0, w: 100, h: 100 };
      let isMax = false;

      if (template === "50_50") {
        if (idx % 2 === 0) {
          rect = { x: 0, y: 0, w: 50, h: 100 };
        } else {
          rect = { x: 50, y: 0, w: 50, h: 100 };
        }
      } else if (template === "65_35") {
        if (idx % 2 === 0) {
          rect = { x: 0, y: 0, w: 65, h: 100 };
        } else {
          rect = { x: 65, y: 0, w: 35, h: 100 };
        }
      } else if (template === "thirds") {
        const slot = idx % 3;
        const w = 33.33;
        rect = { x: slot * w, y: 0, w: slot === 2 ? 33.34 : 33.33, h: 100 };
      } else if (template === "quadrants") {
        const slot = idx % 4;
        const x = slot % 2 === 0 ? 0 : 50;
        const y = slot < 2 ? 0 : 50;
        rect = { x, y, w: 50, h: 50 };
      } else if (template === "maximized") {
        rect = { x: 0, y: 0, w: 100, h: 100 };
        isMax = true;
      }

      const indexInAll = newApps.findIndex((a) => a.id === app.id);
      if (indexInAll !== -1) {
        newApps[indexInAll] = {
          ...newApps[indexInAll],
          window_config: {
            monitor_index: activeMonitorIndex,
            x: { type: "percent", value: Math.round(rect.x) },
            y: { type: "percent", value: Math.round(rect.y) },
            width: { type: "percent", value: Math.round(rect.w) },
            height: { type: "percent", value: Math.round(rect.h) },
            is_maximized: isMax,
          },
        };
      }
    });

    onChangeApps(newApps);
  };

  // Mouse event handlers for dragging & resizing
  const snapValue = (val: number, snapPoints: number[], threshold = 2.5): number => {
    for (const p of snapPoints) {
      if (Math.abs(val - p) <= threshold) {
        return p;
      }
    }
    return val;
  };

  const handleMouseDown = (
    e: React.MouseEvent,
    appId: string,
    handle: "move" | "nw" | "ne" | "sw" | "se" | "n" | "s" | "e" | "w"
  ) => {
    e.stopPropagation();
    setActiveAppId(appId);
    onSelectApp?.(appId);

    const app = apps.find((a) => a.id === appId);
    if (!app) return;

    setDragging({
      appId,
      startX: e.clientX,
      startY: e.clientY,
      origRect: getAppRect(app),
      handle,
    });
  };

  useEffect(() => {
    if (!dragging) return;

    const onMouseMove = (e: MouseEvent) => {
      if (!canvasRef.current) return;
      const rect = canvasRef.current.getBoundingClientRect();
      const deltaXPercent = ((e.clientX - dragging.startX) / rect.width) * 100;
      const deltaYPercent = ((e.clientY - dragging.startY) / rect.height) * 100;

      const orig = dragging.origRect;
      let newRect = { ...orig };

      // Base snap points: screen edges and centerlines
      const snapX = [0, 25, 33.33, 50, 66.66, 75, 100];
      const snapY = [0, 25, 50, 75, 100];

      if (dragging.handle === "move") {
        let targetX = orig.x + deltaXPercent;
        let targetY = orig.y + deltaYPercent;

        targetX = snapValue(targetX, snapX);
        targetY = snapValue(targetY, snapY);

        // Clamp inside [0, 100 - w]
        targetX = Math.max(0, Math.min(100 - orig.w, targetX));
        targetY = Math.max(0, Math.min(100 - orig.h, targetY));

        newRect.x = targetX;
        newRect.y = targetY;
      } else if (dragging.handle === "e") {
        let targetW = orig.w + deltaXPercent;
        targetW = snapValue(orig.x + targetW, snapX) - orig.x;
        newRect.w = Math.max(10, Math.min(100 - orig.x, targetW));
      } else if (dragging.handle === "s") {
        let targetH = orig.h + deltaYPercent;
        targetH = snapValue(orig.y + targetH, snapY) - orig.y;
        newRect.h = Math.max(10, Math.min(100 - orig.y, targetH));
      } else if (dragging.handle === "se") {
        let targetW = orig.w + deltaXPercent;
        let targetH = orig.h + deltaYPercent;
        targetW = snapValue(orig.x + targetW, snapX) - orig.x;
        targetH = snapValue(orig.y + targetH, snapY) - orig.y;
        newRect.w = Math.max(10, Math.min(100 - orig.x, targetW));
        newRect.h = Math.max(10, Math.min(100 - orig.y, targetH));
      }

      updateAppPlacement(dragging.appId, newRect);
    };

    const onMouseUp = () => {
      setDragging(null);
    };

    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
    return () => {
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
    };
  }, [dragging, updateAppPlacement]);

  const selectedApp = apps.find((a) => a.id === activeAppId);
  const selectedRect = selectedApp ? getAppRect(selectedApp) : null;

  return (
    <div className="space-y-4">
      {/* Top bar: Monitor Selector + Quick Templates */}
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-zinc-800 pb-3">
        {/* Monitor Tabs */}
        <div className="flex items-center gap-1.5">
          <span className="text-xs text-zinc-400 mr-1">Display:</span>
          {monitors.map((mon) => (
            <button
              key={mon.monitor_index}
              type="button"
              onClick={() => setActiveMonitorIndex(mon.monitor_index)}
              className={`px-2.5 py-1 text-xs rounded border transition-colors ${
                activeMonitorIndex === mon.monitor_index
                  ? "bg-zinc-800 text-emerald-400 border-emerald-500/50 font-medium"
                  : "bg-zinc-900 text-zinc-400 border-zinc-800 hover:text-zinc-200"
              }`}
            >
              Monitor {mon.monitor_index + 1}
              {mon.is_primary && " (Primary)"}
            </button>
          ))}
        </div>

        {/* Layout Templates */}
        <div className="flex items-center gap-1">
          <span className="text-[11px] text-zinc-500 mr-1">Presets:</span>
          <button
            type="button"
            onClick={() => applyTemplate("50_50")}
            className="px-2 py-1 text-[11px] bg-zinc-900 hover:bg-zinc-800 text-zinc-300 border border-zinc-800 rounded transition-colors"
          >
            50/50
          </button>
          <button
            type="button"
            onClick={() => applyTemplate("65_35")}
            className="px-2 py-1 text-[11px] bg-zinc-900 hover:bg-zinc-800 text-zinc-300 border border-zinc-800 rounded transition-colors"
          >
            65/35 Focus
          </button>
          <button
            type="button"
            onClick={() => applyTemplate("thirds")}
            className="px-2 py-1 text-[11px] bg-zinc-900 hover:bg-zinc-800 text-zinc-300 border border-zinc-800 rounded transition-colors"
          >
            Thirds
          </button>
          <button
            type="button"
            onClick={() => applyTemplate("quadrants")}
            className="px-2 py-1 text-[11px] bg-zinc-900 hover:bg-zinc-800 text-zinc-300 border border-zinc-800 rounded transition-colors"
          >
            2x2 Grid
          </button>
          <button
            type="button"
            onClick={() => applyTemplate("maximized")}
            className="px-2 py-1 text-[11px] bg-zinc-900 hover:bg-zinc-800 text-zinc-300 border border-zinc-800 rounded transition-colors"
          >
            Full
          </button>
        </div>
      </div>

      {/* Warnings bar */}
      {(overlaps.length > 0 || outOfBounds.length > 0) && (
        <div className="p-2.5 rounded border border-amber-900/60 bg-amber-950/20 text-amber-300 text-xs space-y-1">
          {overlaps.map((msg, i) => (
            <div key={`col-${i}`} className="flex items-center gap-1.5">
              <span className="text-amber-400 font-bold">!</span>
              <span>Overlap: {msg}</span>
            </div>
          ))}
          {outOfBounds.map((msg, i) => (
            <div key={`oob-${i}`} className="flex items-center gap-1.5">
              <span className="text-amber-400 font-bold">!</span>
              <span>{msg}</span>
            </div>
          ))}
        </div>
      )}

      {/* Interactive Monitor Canvas */}
      <div className="relative w-full aspect-[16/9] max-h-[320px] bg-zinc-950 border border-zinc-800 rounded-lg p-2 overflow-hidden shadow-inner flex flex-col justify-between">
        {/* Subtle grid background */}
        <div
          ref={canvasRef}
          className="relative w-full h-full bg-zinc-950 rounded border border-zinc-800/80 overflow-hidden"
          style={{
            backgroundImage:
              "linear-gradient(to right, #27272a22 1px, transparent 1px), linear-gradient(to bottom, #27272a22 1px, transparent 1px)",
            backgroundSize: "25% 25%",
          }}
        >
          {monitorApps.map((app, idx) => {
            const rect = getAppRect(app);
            const isSelected = app.id === activeAppId;

            return (
              <div
                key={app.id}
                onMouseDown={(e) => handleMouseDown(e, app.id, "move")}
                style={{
                  left: `${rect.x}%`,
                  top: `${rect.y}%`,
                  width: `${rect.w}%`,
                  height: `${rect.h}%`,
                }}
                className={`absolute select-none cursor-move rounded transition-all duration-75 flex flex-col justify-between p-2 border ${
                  isSelected
                    ? "border-emerald-500 bg-emerald-500/15 shadow-[0_0_12px_rgba(16,185,129,0.2)] z-20"
                    : "border-zinc-700 bg-zinc-900/60 hover:border-zinc-500 z-10"
                }`}
              >
                {/* Header label */}
                <div className="flex items-center justify-between pointer-events-none">
                  <div className="flex items-center gap-1.5 truncate">
                    <span
                      className={`text-[9px] px-1 py-0.5 rounded font-mono font-bold ${
                        isSelected
                          ? "bg-emerald-500/30 text-emerald-300"
                          : "bg-zinc-800 text-zinc-400"
                      }`}
                    >
                      #{idx + 1}
                    </span>
                    <span className="text-xs font-semibold text-zinc-100 truncate">
                      {app.name}
                    </span>
                  </div>
                  {app.window_config?.is_maximized && (
                    <span className="text-[9px] text-emerald-400 font-mono">[MAX]</span>
                  )}
                </div>

                {/* Dimension readout */}
                <div className="text-[10px] font-mono text-zinc-400 pointer-events-none">
                  <span>{rect.w}% × {rect.h}%</span>
                  <span className="text-zinc-600 block text-[9px]">
                    ({Math.round((rect.w / 100) * activeMonitor.width)}px ×{" "}
                    {Math.round((rect.h / 100) * activeMonitor.height)}px)
                  </span>
                </div>

                {/* Resize Handle (bottom-right) */}
                {isSelected && (
                  <div
                    onMouseDown={(e) => handleMouseDown(e, app.id, "se")}
                    className="absolute bottom-0 right-0 w-3.5 h-3.5 cursor-se-resize flex items-center justify-center text-emerald-400 hover:text-emerald-200"
                    title="Resize region"
                  >
                    <svg className="w-2.5 h-2.5" viewBox="0 0 10 10" fill="currentColor">
                      <polygon points="10,0 10,10 0,10" />
                    </svg>
                  </div>
                )}
              </div>
            );
          })}

          {monitorApps.length === 0 && (
            <div className="h-full flex items-center justify-center text-xs text-zinc-600">
              No apps assigned to Monitor {activeMonitorIndex + 1}. Add apps below.
            </div>
          )}
        </div>
      </div>

      {/* Two-Way Sync Form Controls for Selected App */}
      {selectedApp && selectedRect && (
        <div className="p-3 bg-zinc-900/60 border border-zinc-800 rounded-lg space-y-3">
          <div className="flex items-center justify-between">
            <span className="text-xs font-semibold text-zinc-200">
              Coordinates for <span className="text-emerald-400">{selectedApp.name}</span>
            </span>
            <label className="flex items-center gap-2 cursor-pointer text-xs text-zinc-400">
              <input
                type="checkbox"
                checked={selectedApp.window_config?.is_maximized ?? false}
                onChange={(e) =>
                  updateAppPlacement(selectedApp.id, { is_maximized: e.target.checked })
                }
                className="rounded border-zinc-700 bg-zinc-950 text-emerald-500 focus:ring-0"
              />
              Maximize Window
            </label>
          </div>

          <div className="grid grid-cols-4 gap-2.5 text-xs">
            <div>
              <label className="block text-[10px] text-zinc-400 mb-1">X Position (%)</label>
              <input
                type="number"
                min={0}
                max={90}
                value={selectedRect.x}
                onChange={(e) =>
                  updateAppPlacement(selectedApp.id, {
                    x: Math.max(0, parseInt(e.target.value) || 0),
                  })
                }
                className="w-full bg-zinc-950 border border-zinc-800 rounded px-2 py-1 text-zinc-200 font-mono focus:border-emerald-500 focus:outline-none"
              />
            </div>

            <div>
              <label className="block text-[10px] text-zinc-400 mb-1">Y Position (%)</label>
              <input
                type="number"
                min={0}
                max={90}
                value={selectedRect.y}
                onChange={(e) =>
                  updateAppPlacement(selectedApp.id, {
                    y: Math.max(0, parseInt(e.target.value) || 0),
                  })
                }
                className="w-full bg-zinc-950 border border-zinc-800 rounded px-2 py-1 text-zinc-200 font-mono focus:border-emerald-500 focus:outline-none"
              />
            </div>

            <div>
              <label className="block text-[10px] text-zinc-400 mb-1">Width (%)</label>
              <input
                type="number"
                min={10}
                max={100}
                value={selectedRect.w}
                onChange={(e) =>
                  updateAppPlacement(selectedApp.id, {
                    w: Math.max(10, parseInt(e.target.value) || 10),
                  })
                }
                className="w-full bg-zinc-950 border border-zinc-800 rounded px-2 py-1 text-zinc-200 font-mono focus:border-emerald-500 focus:outline-none"
              />
            </div>

            <div>
              <label className="block text-[10px] text-zinc-400 mb-1">Height (%)</label>
              <input
                type="number"
                min={10}
                max={100}
                value={selectedRect.h}
                onChange={(e) =>
                  updateAppPlacement(selectedApp.id, {
                    h: Math.max(10, parseInt(e.target.value) || 10),
                  })
                }
                className="w-full bg-zinc-950 border border-zinc-800 rounded px-2 py-1 text-zinc-200 font-mono focus:border-emerald-500 focus:outline-none"
              />
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default VisualLayoutEditor;

