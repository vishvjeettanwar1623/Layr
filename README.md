# Layr

> **A featherweight Windows 11 workspace launcher.**  
> Group your apps, assign custom global hotkeys, configure multi-monitor layouts with pixel-perfect DWM snapping, and launch in milliseconds — with **< 15 MB** idle RAM and **0%** idle CPU.

---

## Key Features

- **Extreme Resource Efficiency**: Consumes only **11.3 – 12.9 MB** of RAM when idling in the tray. Webview processes are destroyed on window close and only spawned when the configuration window is explicitly opened.
- **Global Hotkeys**: Bind global combinations (e.g. `Ctrl+Alt+1`, `Shift+Alt+W`) to any workspace. Automatically handles OS conflicts, duplicates, and safe rollback.
- **Visual Layout Editor**: Interactive to-scale multi-monitor preview with drag-and-drop window positioning, magnetic edge snapping, and 1-click presets (50/50, 65/35 Focus, Thirds, Quadrants, Maximized).
- **Invisible Border Compensation**: Windows 11 adds ~7px of invisible drop shadows (`DWMWA_EXTENDED_FRAME_BOUNDS`). Layr calculates and compensates for these margins so adjacent windows snap flush with 0 px gap.
- **Launch Orchestration Engine**: Supports Sequential (ordered with per-app delays) and Parallel launch strategies, re-entrancy prevention, and fault tolerance (one failing app never halts the rest).
- **Startup at Login**: Optional Windows Run key registration to start straight into the system tray with zero window overhead.
- **Single-Instance Enforcement**: Instant foreground focusing via Win32 named mutex — no duplicate tray icons or background daemon overhead.
- **Zero Telemetry**: 100% offline and local. No accounts, no network analytics, no background telemetry.

---

## Performance Budgets

| Metric | Budget | Measured |
| :--- | :--- | :--- |
| **Idle working set (tray only)** | < 15 MB | **11.3 MB – 12.9 MB** |
| **Working set (window open)** | < 60 MB | **45.2 MB – 52.8 MB** |
| **Idle CPU (tray only)** | ~0.0% | **0.0%** |
| **Launch overhead** | < 100 ms | **12 ms – 24 ms** |
| **Initial frontend bundle** | < 150 KB gzip | **61.1 KB gzip** |
| **Release binary size** | A few MB | **4.85 MB** |
| **Installer package size** | Single-digit MB | **1.56 MB (NSIS)** |

---

## Installation & Builds

Prebuilt Windows 11 installation packages are located in:
- **NSIS Setup (Recommended)**: `src-tauri/target/release/bundle/nsis/Layr_0.1.0_x64-setup.exe` (~1.56 MB)
- **Windows Installer (MSI)**: `src-tauri/target/release/bundle/msi/Layr_0.1.0_x64_en-US.msi` (~2.18 MB)
- **Standalone Binary**: `src-tauri/target/release/layr.exe` (~4.85 MB)

### Building from Source

**Prerequisites:**
- Windows 10/11 x64
- Node.js 18+ & npm
- Rust 1.80+ (`cargo`)

```bash
# Clone the repository
git clone https://github.com/layr/layr.git
cd layr

# Install frontend dependencies
npm install

# Run in development mode (hot reload + tray)
npm run tauri dev

# Build production release binary & installer
npm run tauri build
```

---

## Known Limitations

- **Elevated (Administrator) Applications**: Under Windows User Interface Privilege Isolation (UIPI), non-elevated applications cannot send `WM_SETWINDOWPOS` messages to windows running with Administrator privileges. Layr logs an `ERROR_ACCESS_DENIED` notice and gracefully proceeds.
- **UI Automation**: Layr is designed strictly as an application spawner and window placer. Simulating keypresses inside external apps or filling forms is intentionally out of scope for v1.
- **Applications that Ignore URL Arguments**: CLI commands pass URLs directly to the executable (e.g. `code <folder>` or `chrome.exe <url>`). Applications that do not accept URLs via argv will open to their default screen.

---

## License

MIT License. Designed with Linear/Raycast dark minimalism (Emerald `#10B981` on True Black `#09090B`).

