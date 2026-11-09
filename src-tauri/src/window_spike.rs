// Window detection spike: snapshot windows, launch an app, detect the new window,
// restore it, and move it to the left half of the primary monitor.

use log::{info, warn};
use std::collections::HashSet;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::process::Command;
use std::time::{Duration, Instant};
use tauri::AppHandle;
use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTOPRIMARY,
};
use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
use windows::Win32::UI::WindowsAndMessaging::*;

/// Data collected about a window.
#[allow(dead_code)]
#[derive(Debug, Clone)]
struct WindowInfo {
    hwnd: isize,
    title: String,
    exe_name: String,
    pid: u32,
}

/// Get the process ID for a window handle.
fn get_window_pid(hwnd: HWND) -> u32 {
    let mut pid: u32 = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }
    pid
}

/// Get the executable name for a process ID.
fn get_exe_name(pid: u32) -> String {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid);
        if let Ok(handle) = handle {
            let mut buf = [0u16; 512];
            let len = GetModuleFileNameExW(Some(handle), None, &mut buf);
            if len > 0 {
                let path = OsString::from_wide(&buf[..len as usize]);
                if let Some(name) = std::path::Path::new(&path).file_name() {
                    return name.to_string_lossy().to_lowercase();
                }
            }
        }
    }
    String::new()
}

/// Get the window title.
fn get_window_title(hwnd: HWND) -> String {
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

/// Check if a window is a visible, non-tool, top-level window.
fn is_real_window(hwnd: HWND) -> bool {
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() {
            return false;
        }
        let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;

        // Skip tool windows
        if (ex_style & WS_EX_TOOLWINDOW.0) != 0 {
            return false;
        }

        // Must have a title bar or be an app window
        let is_app_window = (ex_style & WS_EX_APPWINDOW.0) != 0;
        let has_caption = (style & WS_CAPTION.0) == WS_CAPTION.0;

        // Must be a top-level window (no owner for our purposes)
        let owner = GetWindow(hwnd, GW_OWNER);
        let has_owner = owner.is_ok() && owner.unwrap() != HWND::default();

        (is_app_window || has_caption) && !has_owner
    }
}

/// Snapshot all visible top-level window handles.
fn snapshot_windows() -> HashSet<isize> {
    let mut handles = HashSet::new();
    unsafe {
        let _ = EnumWindows(
            Some(enum_callback),
            LPARAM(&mut handles as *mut HashSet<isize> as isize),
        );
    }
    handles
}

unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    if is_real_window(hwnd) {
        let handles = &mut *(lparam.0 as *mut HashSet<isize>);
        handles.insert(hwnd.0 as isize);
    }
    BOOL(1)
}

/// Get the work area of the primary monitor.
fn get_primary_work_area() -> RECT {
    unsafe {
        let hwnd = GetDesktopWindow();
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let _ = GetMonitorInfoW(monitor, &mut info);
        info.rcWork
    }
}

/// Move and resize a window to a given rect, restoring it first if minimized/maximized.
fn position_window(hwnd: HWND, rect: &RECT) {
    unsafe {
        // Restore if minimized or maximized
        if IsIconic(hwnd).as_bool() || IsZoomed(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
            std::thread::sleep(Duration::from_millis(50));
        }

        let _ = SetWindowPos(
            hwnd,
            None,
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );

        info!(
            "Positioned window {:?} to ({}, {}, {}x{})",
            hwnd,
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top
        );
    }
}

/// Detect a new window by exe name, waiting up to `timeout`.
fn detect_new_window(
    before: &HashSet<isize>,
    target_exe: &str,
    timeout: Duration,
) -> Option<WindowInfo> {
    let start = Instant::now();
    let target = target_exe.to_lowercase();

    while start.elapsed() < timeout {
        std::thread::sleep(Duration::from_millis(80));
        let current = snapshot_windows();

        for &h in current.difference(before) {
            let hwnd = HWND(h as *mut _);
            let pid = get_window_pid(hwnd);
            let exe = get_exe_name(pid);

            if exe.contains(&target) {
                let title = get_window_title(hwnd);
                info!(
                    "Detected new window: hwnd={}, pid={}, exe={}, title={}",
                    h, pid, exe, title
                );
                return Some(WindowInfo {
                    hwnd: h,
                    title,
                    exe_name: exe,
                    pid,
                });
            }
        }
    }

    warn!("Timeout waiting for window from {}", target_exe);
    None
}

/// Run the complete window detection spike.
/// Launches notepad.exe, detects its window, positions it to left half of primary monitor.
pub fn run_spike(app: &AppHandle) {
    info!("=== Window Detection Spike Starting ===");

    // Step 1: snapshot before
    let before = snapshot_windows();
    info!("Snapshot: {} windows before launch", before.len());

    // Step 2: launch notepad (simple, always available)
    info!("Launching notepad.exe...");
    match Command::new("notepad.exe").spawn() {
        Ok(child) => info!("Spawned notepad, PID={}", child.id()),
        Err(e) => {
            warn!("Failed to spawn notepad: {}", e);
            return;
        }
    }

    // Step 3: detect new notepad window
    let timeout = Duration::from_secs(5);
    if let Some(win_info) = detect_new_window(&before, "notepad", timeout) {
        info!("Found notepad window: {:?}", win_info);

        // Step 4: position to left half of primary monitor
        let work = get_primary_work_area();
        let half_width = (work.right - work.left) / 2;
        let target_rect = RECT {
            left: work.left,
            top: work.top,
            right: work.left + half_width,
            bottom: work.bottom,
        };
        position_window(HWND(win_info.hwnd as *mut _), &target_rect);
        info!("Notepad positioned to left half of primary monitor");

        // Step 5: now test with a launcher-process app (Edge or Chrome)
        info!("--- Testing launcher-process app ---");
        let before2 = snapshot_windows();

        // Use msedge with a dedicated user-data-dir so it never hits ProcessSingleton locks
        let edge_std = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe";
        let chrome_std = r"C:\Program Files\Google\Chrome\Application\chrome.exe";
        let temp_dir = std::env::temp_dir().join("layr_browser_spike");
        let user_data_arg = format!("--user-data-dir={}", temp_dir.to_string_lossy());

        let (launcher_exe, launcher_args, launcher_name) = if std::path::Path::new(edge_std).exists() {
            (
                edge_std.to_string(),
                vec![
                    user_data_arg,
                    "--no-first-run".into(),
                    "about:blank".into(),
                ],
                "msedge",
            )
        } else if std::path::Path::new(chrome_std).exists() {
            (
                chrome_std.to_string(),
                vec![
                    user_data_arg,
                    "--no-first-run".into(),
                    "about:blank".into(),
                ],
                "chrome",
            )
        } else {
            info!("No browser found for launcher-process test, skipping");
            send_notification(app, "Spike: Notepad OK, no browser found");
            return;
        };

        info!("Launching {} with args {:?}...", launcher_exe, launcher_args);
        match Command::new(&launcher_exe).args(&launcher_args).spawn() {
            Ok(child) => info!("Spawned {}, PID={}", launcher_exe, child.id()),
            Err(e) => {
                warn!("Failed to spawn {}: {}", launcher_exe, e);
                send_notification(
                    app,
                    &format!("Spike: Notepad OK, {} failed to launch", launcher_exe),
                );
                return;
            }
        }

        if let Some(win) = detect_new_window(&before2, launcher_name, Duration::from_secs(12)) {
            info!("Found {} window: {:?}", launcher_name, win);

            // Position to right half
            let right_rect = RECT {
                left: work.left + half_width,
                top: work.top,
                right: work.right,
                bottom: work.bottom,
            };
            position_window(HWND(win.hwnd as *mut _), &right_rect);
            info!("{} positioned to right half of primary monitor", launcher_name);

            send_notification(
                app,
                &format!(
                    "Spike complete! Notepad (left) + {} (right). Method: exe-name match via EnumWindows diff.",
                    launcher_name
                ),
            );
        } else {
            info!("Launcher window detection timed out");
            send_notification(
                app,
                &format!("Spike: Notepad OK, {} window not detected (timeout)", launcher_name),
            );
        }
    } else {
        warn!("Notepad window not detected!");
        send_notification(app, "Spike FAILED: Notepad window not detected");
    }

    info!("=== Window Detection Spike Complete ===");
}

/// Send a system notification via Tauri's tray.
fn send_notification(app: &AppHandle, message: &str) {
    info!("[Notification] {}", message);
    // Use the tray tooltip as a simple feedback mechanism
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_tooltip(Some(message));
    }
}
