pub mod layout_math;

use layout_math::{calculate_window_rect, DpiScale, InvisibleBorderPadding, MonitorWorkArea, WindowPlacement};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::Path;
use std::time::{Duration, Instant};
use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, MonitorFromWindow, HDC, HMONITOR, MONITORINFO,
    MONITOR_DEFAULTTOPRIMARY,
};
use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
use windows::Win32::UI::WindowsAndMessaging::*;

/// Error occurring during window detection or placement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum WindowError {
    #[error("Window detection timed out after {0} ms")]
    Timeout(u64),
    #[error("Cannot position elevated window due to Windows UIPI (run Layr as Administrator to manage elevated apps): {0}")]
    ElevatedAccessDenied(String),
    #[error("Window handle invalid or closed during positioning")]
    InvalidHandle,
}

/// Information about a detected top-level window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectedWindow {
    pub hwnd: isize,
    pub title: String,
    pub exe_name: String,
    pub pid: u32,
}

/// Snapshot all visible top-level application windows.
pub fn snapshot_top_level_windows() -> HashSet<isize> {
    let mut handles = HashSet::new();
    unsafe {
        let _ = EnumWindows(
            Some(enum_window_callback),
            LPARAM(&mut handles as *mut HashSet<isize> as isize),
        );
    }
    handles
}

unsafe extern "system" fn enum_window_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    if is_top_level_app_window(hwnd) {
        let handles = &mut *(lparam.0 as *mut HashSet<isize>);
        handles.insert(hwnd.0 as isize);
    }
    BOOL(1)
}

/// Checks if a window is a visible top-level application window.
pub fn is_top_level_app_window(hwnd: HWND) -> bool {
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() {
            return false;
        }

        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;

        // Skip tool windows
        if (ex_style & WS_EX_TOOLWINDOW.0) != 0 {
            return false;
        }

        let is_app_window = (ex_style & WS_EX_APPWINDOW.0) != 0;
        let has_caption = (style & WS_CAPTION.0) == WS_CAPTION.0;

        let owner = GetWindow(hwnd, GW_OWNER);
        let has_owner = owner.is_ok() && owner.unwrap() != HWND::default();

        (is_app_window || has_caption) && !has_owner
    }
}

pub fn get_window_pid(hwnd: HWND) -> u32 {
    let mut pid: u32 = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }
    pid
}

pub fn get_window_exe_name(pid: u32) -> String {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid);
        if let Ok(handle) = handle {
            let mut buf = [0u16; 512];
            let len = GetModuleFileNameExW(Some(handle), None, &mut buf);
            if len > 0 {
                let path = OsString::from_wide(&buf[..len as usize]);
                if let Some(name) = Path::new(&path).file_name() {
                    return name.to_string_lossy().to_lowercase();
                }
            }
        }
    }
    String::new()
}

pub fn get_window_title(hwnd: HWND) -> String {
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        if len == 0 {
            return String::new();
        }
        let mut buf = vec![0u16; (len + 1) as usize];
        let actual = GetWindowTextW(hwnd, &mut buf);
        if actual > 0 {
            OsString::from_wide(&buf[..actual as usize])
                .to_string_lossy()
                .into_owned()
        } else {
            String::new()
        }
    }
}

/// Wait for a new window matching the executable name and optional title hint.
pub fn wait_for_new_window(
    before_snapshot: &HashSet<isize>,
    target_exe: &str,
    title_hint: Option<&str>,
    timeout: Duration,
) -> Result<DetectedWindow, WindowError> {
    let start = Instant::now();
    let target_clean = Path::new(target_exe)
        .file_name()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| target_exe.to_lowercase());

    while start.elapsed() < timeout {
        std::thread::sleep(Duration::from_millis(60));
        let current = snapshot_top_level_windows();

        for &h in current.difference(before_snapshot) {
            let hwnd = HWND(h as *mut _);
            let pid = get_window_pid(hwnd);
            let exe = get_window_exe_name(pid);

            if exe.contains(&target_clean) || target_clean.contains(&exe) {
                let title = get_window_title(hwnd);
                if let Some(hint) = title_hint {
                    if !title.to_lowercase().contains(&hint.to_lowercase()) {
                        continue;
                    }
                }

                info!(
                    "Matched new window: hwnd=0x{:x}, pid={}, exe='{}', title='{}'",
                    h, pid, exe, title
                );
                return Ok(DetectedWindow {
                    hwnd: h,
                    title,
                    exe_name: exe,
                    pid,
                });
            }
        }
    }

    warn!("Detection timeout waiting for window from '{}'", target_exe);
    Err(WindowError::Timeout(timeout.as_millis() as u64))
}

/// Find an existing visible window for an executable (used for `reuse_existing_window` policy).
pub fn find_existing_window(target_exe: &str) -> Option<DetectedWindow> {
    let current = snapshot_top_level_windows();
    let target_clean = Path::new(target_exe)
        .file_name()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| target_exe.to_lowercase());

    for h in current {
        let hwnd = HWND(h as *mut _);
        let pid = get_window_pid(hwnd);
        let exe = get_window_exe_name(pid);

        if exe.contains(&target_clean) || target_clean.contains(&exe) {
            return Some(DetectedWindow {
                hwnd: h,
                title: get_window_title(hwnd),
                exe_name: exe,
                pid,
            });
        }
    }
    None
}

/// Enumerate all connected monitor work areas, with primary monitor first.
pub fn get_monitor_work_areas() -> Vec<MonitorWorkArea> {
    let mut monitors = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(enum_monitor_callback),
            LPARAM(&mut monitors as *mut Vec<MonitorWorkArea> as isize),
        );
    }

    if monitors.is_empty() {
        // Fallback to desktop primary
        unsafe {
            let hwnd = GetDesktopWindow();
            let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY);
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            let _ = GetMonitorInfoW(mon, &mut info);
            let rc = info.rcWork;
            monitors.push(MonitorWorkArea {
                x: rc.left,
                y: rc.top,
                width: rc.right - rc.left,
                height: rc.bottom - rc.top,
                is_primary: true,
            });
        }
    }

    monitors
}

unsafe extern "system" fn enum_monitor_callback(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _lprect: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let monitors = &mut *(lparam.0 as *mut Vec<MonitorWorkArea>);
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };

    if GetMonitorInfoW(hmonitor, &mut info).as_bool() {
        let is_primary = (info.dwFlags & 1) != 0; // MONITORINFOF_PRIMARY = 1
        let rc = info.rcWork;
        let area = MonitorWorkArea {
            x: rc.left,
            y: rc.top,
            width: rc.right - rc.left,
            height: rc.bottom - rc.top,
            is_primary,
        };

        if is_primary {
            monitors.insert(0, area);
        } else {
            monitors.push(area);
        }
    }

    BOOL(1)
}

/// Computes invisible drop shadow padding using DWM.
pub fn get_invisible_border_padding(hwnd: HWND) -> InvisibleBorderPadding {
    unsafe {
        let mut window_rect = RECT::default();
        let mut frame_rect = RECT::default();

        let _ = GetWindowRect(hwnd, &mut window_rect);
        let res = DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut frame_rect as *mut _ as *mut _,
            std::mem::size_of::<RECT>() as u32,
        );

        if res.is_ok() {
            InvisibleBorderPadding {
                left: (frame_rect.left - window_rect.left).max(0),
                right: (window_rect.right - frame_rect.right).max(0),
                top: (frame_rect.top - window_rect.top).max(0),
                bottom: (window_rect.bottom - frame_rect.bottom).max(0),
            }
        } else {
            InvisibleBorderPadding::default()
        }
    }
}

/// Positions a window according to WindowPlacement configuration,
/// handling restore from minimized/maximized, invisible borders, and DPI.
pub fn position_window_by_placement(
    hwnd_val: isize,
    placement: &WindowPlacement,
) -> Result<(), WindowError> {
    let hwnd = HWND(hwnd_val as *mut _);

    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            return Err(WindowError::InvalidHandle);
        }

        // 1. Restore window if minimized or maximized
        if IsIconic(hwnd).as_bool() || IsZoomed(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
            std::thread::sleep(Duration::from_millis(50));
        }

        // 2. Select monitor (or fall back to primary if requested index is missing)
        let monitors = get_monitor_work_areas();
        let work_area = match placement.monitor_index {
            Some(idx) if idx < monitors.len() => &monitors[idx],
            _ => monitors.first().expect("Primary monitor must exist"),
        };

        // 3. Query invisible border compensation from DWM
        let padding = get_invisible_border_padding(hwnd);
        let dpi_scale = DpiScale::default();

        // 4. Calculate target rect
        let rect = calculate_window_rect(work_area, placement, dpi_scale, &padding);

        // 5. Handle Maximized or Fullscreen
        if placement.is_maximized {
            let _ = ShowWindow(hwnd, SW_MAXIMIZE);
            return Ok(());
        }

        // 6. SetWindowPos
        if let Err(err) = SetWindowPos(
            hwnd,
            None,
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            SWP_NOZORDER | SWP_NOACTIVATE,
        ) {
            if err.code().0 as u32 == 5 {
                // ERROR_ACCESS_DENIED = 5 (UIPI restriction)
                return Err(WindowError::ElevatedAccessDenied(format!(
                    "Access denied to HWND 0x{:x}",
                    hwnd_val
                )));
            }
        }

        info!(
            "Positioned window 0x{:x} to ({}, {}, {}x{})",
            hwnd_val, rect.x, rect.y, rect.width, rect.height
        );
        Ok(())
    }
}
