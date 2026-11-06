use crate::modules::process;
use crate::modules::window;
use crate::modules::workspace::{AppEntry, LaunchStrategy, Workspace};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum EngineError {
    #[error("Workspace '{0}' is already running; launch request ignored to prevent re-entrancy")]
    AlreadyRunning(String),
    #[error("Workspace contains no applications")]
    EmptyWorkspace,
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "message", rename_all = "snake_case")]
pub enum AppLaunchStatus {
    Pending,
    Starting,
    WaitingForWindow,
    Positioned,
    Done,
    Skipped,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppExecutionResult {
    pub app_id: String,
    pub app_name: String,
    pub status: AppLaunchStatus,
    pub duration_ms: u64,
    pub hwnd: Option<isize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceExecutionResult {
    pub workspace_id: String,
    pub workspace_name: String,
    pub success: bool,
    pub total_duration_ms: u64,
    pub app_results: Vec<AppExecutionResult>,
}

/// Abstract runner trait to allow mocking in unit tests without real OS spawns.
pub trait EngineRunner: Send + Sync {
    fn execute_app(
        &self,
        app: &AppEntry,
        on_progress: &(dyn Fn(AppLaunchStatus) + Send + Sync),
    ) -> AppExecutionResult;
    fn sleep(&self, duration: Duration);
}

/// Real OS runner for actual app spawning and window placement.
pub struct LiveEngineRunner;

impl EngineRunner for LiveEngineRunner {
    fn execute_app(
        &self,
        app: &AppEntry,
        on_progress: &(dyn Fn(AppLaunchStatus) + Send + Sync),
    ) -> AppExecutionResult {
        let start = Instant::now();
        on_progress(AppLaunchStatus::Starting);

        // 1. Snapshot windows prior to spawn
        let before = window::snapshot_top_level_windows();

        // 2. Spawn process
        let spawn_res = match process::spawn_app(app) {
            Ok(res) => res,
            Err(e) => {
                let err_msg = e.to_string();
                warn!("Failed to launch app '{}': {}", app.name, err_msg);
                on_progress(AppLaunchStatus::Failed(err_msg.clone()));
                return AppExecutionResult {
                    app_id: app.id.clone(),
                    app_name: app.name.clone(),
                    status: AppLaunchStatus::Failed(err_msg),
                    duration_ms: start.elapsed().as_millis() as u64,
                    hwnd: None,
                };
            }
        };

        if spawn_res.skipped {
            info!("App '{}' was skipped by duplicate policy", app.name);
            on_progress(AppLaunchStatus::Skipped);
            return AppExecutionResult {
                app_id: app.id.clone(),
                app_name: app.name.clone(),
                status: AppLaunchStatus::Skipped,
                duration_ms: start.elapsed().as_millis() as u64,
                hwnd: None,
            };
        }

        // 3. Window detection & placement if configured
        let mut final_hwnd: Option<isize> = None;
        let final_status = AppLaunchStatus::Done;

        if let Some(win_cfg) = &app.window_config {
            on_progress(AppLaunchStatus::WaitingForWindow);

            match window::wait_for_new_window(
                &before,
                &app.executable_path,
                None,
                Duration::from_secs(6),
            ) {
                Ok(win) => {
                    final_hwnd = Some(win.hwnd);
                    if let Err(pos_err) = window::position_window_by_placement(win.hwnd, win_cfg) {
                        warn!("Window positioning warning for '{}': {}", app.name, pos_err);
                    } else {
                        on_progress(AppLaunchStatus::Positioned);
                    }
                }
                Err(win_err) => {
                    warn!(
                        "Window wait timed out or failed for '{}': {}",
                        app.name, win_err
                    );
                    // App spawned successfully even if window wasn't detected in time
                }
            }
        }

        on_progress(final_status.clone());
        AppExecutionResult {
            app_id: app.id.clone(),
            app_name: app.name.clone(),
            status: final_status,
            duration_ms: start.elapsed().as_millis() as u64,
            hwnd: final_hwnd,
        }
    }

    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

/// RAII lock guard to ensure workspace ID is removed from active set when execution finishes.
struct ActiveGuard {
    set: Arc<Mutex<HashSet<String>>>,
    id: String,
}

impl Drop for ActiveGuard {
    fn drop(&mut self) {
        if let Ok(mut set) = self.set.lock() {
            set.remove(&self.id);
        }
    }
}

/// The workspace automation execution engine.
#[derive(Clone)]
pub struct ExecutionEngine {
    active_workspaces: Arc<Mutex<HashSet<String>>>,
}

impl Default for ExecutionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionEngine {
    pub fn new() -> Self {
        Self {
            active_workspaces: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn is_running(&self, workspace_id: &str) -> bool {
        self.active_workspaces
            .lock()
            .map(|set| set.contains(workspace_id))
            .unwrap_or(false)
    }

    /// Execute a workspace with the given runner and progress callback.
    pub fn execute_workspace<R, F>(
        &self,
        workspace: &Workspace,
        runner: Arc<R>,
        progress_cb: F,
    ) -> Result<WorkspaceExecutionResult, EngineError>
    where
        R: EngineRunner + 'static,
        F: Fn(&str, &str, AppLaunchStatus) + Send + Sync + 'static,
    {
        // 1. Re-entrancy guard check
        {
            let mut active = self
                .active_workspaces
                .lock()
                .map_err(|e| EngineError::ExecutionFailed(e.to_string()))?;
            if active.contains(&workspace.id) {
                return Err(EngineError::AlreadyRunning(workspace.id.clone()));
            }
            active.insert(workspace.id.clone());
        }

        let _guard = ActiveGuard {
            set: Arc::clone(&self.active_workspaces),
            id: workspace.id.clone(),
        };

        if workspace.apps.is_empty() {
            return Ok(WorkspaceExecutionResult {
                workspace_id: workspace.id.clone(),
                workspace_name: workspace.name.clone(),
                success: true,
                total_duration_ms: 0,
                app_results: Vec::new(),
            });
        }

        let total_start = Instant::now();
        let progress_cb = Arc::new(progress_cb);

        // Sort apps by launch order
        let mut ordered_apps = workspace.apps.clone();
        ordered_apps.sort_by_key(|a| a.launch_order);

        let app_results = match workspace.strategy {
            LaunchStrategy::Sequential => {
                let mut results = Vec::with_capacity(ordered_apps.len());
                for app in ordered_apps {
                    if app.launch_delay_ms > 0 {
                        runner.sleep(Duration::from_millis(app.launch_delay_ms));
                    }
                    let cb_clone = Arc::clone(&progress_cb);
                    let ws_id = workspace.id.clone();
                    let app_id = app.id.clone();
                    let res = runner.execute_app(
                        &app,
                        &move |status| {
                            cb_clone(&ws_id, &app_id, status);
                        },
                    );
                    results.push(res);
                }
                results
            }
            LaunchStrategy::Parallel => {
                let handles: Vec<_> = ordered_apps
                    .into_iter()
                    .map(|app| {
                        let runner = Arc::clone(&runner);
                        let cb_clone = Arc::clone(&progress_cb);
                        let ws_id = workspace.id.clone();
                        let app_id = app.id.clone();
                        std::thread::spawn(move || {
                            if app.launch_delay_ms > 0 {
                                runner.sleep(Duration::from_millis(app.launch_delay_ms));
                            }
                            runner.execute_app(
                                &app,
                                &move |status| {
                                    cb_clone(&ws_id, &app_id, status);
                                },
                            )
                        })
                    })
                    .collect();

                let mut results = Vec::with_capacity(handles.len());
                for h in handles {
                    if let Ok(res) = h.join() {
                        results.push(res);
                    }
                }
                results
            }
        };

        let overall_success = app_results
            .iter()
            .all(|r| !matches!(r.status, AppLaunchStatus::Failed(_)));

        Ok(WorkspaceExecutionResult {
            workspace_id: workspace.id.clone(),
            workspace_name: workspace.name.clone(),
            success: overall_success,
            total_duration_ms: total_start.elapsed().as_millis() as u64,
            app_results,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockRunner {
        sleep_calls: Arc<Mutex<Vec<u64>>>,
        executed_order: Arc<Mutex<Vec<String>>>,
        simulated_duration_ms: u64,
        fail_app_id: Option<String>,
    }

    impl MockRunner {
        fn new() -> Self {
            Self {
                sleep_calls: Arc::new(Mutex::new(Vec::new())),
                executed_order: Arc::new(Mutex::new(Vec::new())),
                simulated_duration_ms: 5,
                fail_app_id: None,
            }
        }
    }

    impl EngineRunner for MockRunner {
        fn execute_app(
            &self,
            app: &AppEntry,
            on_progress: &(dyn Fn(AppLaunchStatus) + Send + Sync),
        ) -> AppExecutionResult {
            self.executed_order.lock().unwrap().push(app.id.clone());
            on_progress(AppLaunchStatus::Starting);

            if let Some(fail_id) = &self.fail_app_id {
                if fail_id == &app.id {
                    let err = "Simulated executable missing".to_string();
                    on_progress(AppLaunchStatus::Failed(err.clone()));
                    return AppExecutionResult {
                        app_id: app.id.clone(),
                        app_name: app.name.clone(),
                        status: AppLaunchStatus::Failed(err),
                        duration_ms: self.simulated_duration_ms,
                        hwnd: None,
                    };
                }
            }

            on_progress(AppLaunchStatus::Done);
            AppExecutionResult {
                app_id: app.id.clone(),
                app_name: app.name.clone(),
                status: AppLaunchStatus::Done,
                duration_ms: self.simulated_duration_ms,
                hwnd: Some(0x1234),
            }
        }

        fn sleep(&self, duration: Duration) {
            self.sleep_calls
                .lock()
                .unwrap()
                .push(duration.as_millis() as u64);
        }
    }

    #[test]
    fn test_sequential_order_and_delays() {
        let engine = ExecutionEngine::new();
        let runner = Arc::new(MockRunner::new());

        let mut app1 = AppEntry::new("App1", "C:\\app1.exe");
        app1.launch_order = 1;
        app1.launch_delay_ms = 50;

        let mut app2 = AppEntry::new("App2", "C:\\app2.exe");
        app2.launch_order = 2;
        app2.launch_delay_ms = 100;

        let mut app0 = AppEntry::new("App0", "C:\\app0.exe");
        app0.launch_order = 0;
        app0.launch_delay_ms = 0;

        let mut ws = Workspace::new("Test Sequential");
        ws.strategy = LaunchStrategy::Sequential;
        ws.apps = vec![app1.clone(), app2.clone(), app0.clone()];

        let progress_events = Arc::new(Mutex::new(Vec::new()));
        let p_clone = Arc::clone(&progress_events);

        let runner_clone = Arc::clone(&runner);
        let result = engine
            .execute_workspace(&ws, runner_clone, move |ws_id, app_id, status| {
                p_clone
                    .lock()
                    .unwrap()
                    .push((ws_id.to_string(), app_id.to_string(), status));
            })
            .expect("workspace should execute");

        assert!(result.success);
        assert_eq!(result.app_results.len(), 3);

        // Verify ordering: App0 (order 0), then App1 (order 1), then App2 (order 2)
        let order = runner.executed_order.lock().unwrap().clone();
        assert_eq!(order, vec![app0.id, app1.id, app2.id]);

        // Verify delays honored: 50ms and 100ms
        let sleeps = runner.sleep_calls.lock().unwrap().clone();
        assert_eq!(sleeps, vec![50, 100]);
    }

    #[test]
    fn test_parallel_execution_and_all_apps_run() {
        let engine = ExecutionEngine::new();
        let runner = Arc::new(MockRunner::new());

        let mut ws = Workspace::new("Test Parallel");
        ws.strategy = LaunchStrategy::Parallel;
        let app1 = AppEntry::new("P1", "C:\\p1.exe");
        let app2 = AppEntry::new("P2", "C:\\p2.exe");
        ws.apps = vec![app1.clone(), app2.clone()];

        let runner_clone = Arc::clone(&runner);
        let result = engine
            .execute_workspace(&ws, runner_clone, |_, _, _| {})
            .expect("parallel launch should succeed");

        assert!(result.success);
        assert_eq!(result.app_results.len(), 2);

        let order = runner.executed_order.lock().unwrap().clone();
        assert_eq!(order.len(), 2);
    }

    #[test]
    fn test_failing_app_does_not_abort_others() {
        let engine = ExecutionEngine::new();
        let mut mock = MockRunner::new();

        let app1 = AppEntry::new("Good 1", "C:\\good1.exe");
        let app2 = AppEntry::new("Bad App", "C:\\bad.exe");
        let app3 = AppEntry::new("Good 2", "C:\\good2.exe");

        mock.fail_app_id = Some(app2.id.clone());
        let runner = Arc::new(mock);

        let mut ws = Workspace::new("Resilience Test");
        ws.strategy = LaunchStrategy::Sequential;
        ws.apps = vec![app1.clone(), app2.clone(), app3.clone()];

        let result = engine
            .execute_workspace(&ws, runner, |_, _, _| {})
            .expect("overall execution should return result object");

        // Overall success should be false because one app failed
        assert!(!result.success);
        assert_eq!(result.app_results.len(), 3);

        // App 1 is Done
        assert_eq!(result.app_results[0].status, AppLaunchStatus::Done);
        // App 2 is Failed
        assert!(matches!(
            result.app_results[1].status,
            AppLaunchStatus::Failed(_)
        ));
        // App 3 is Done (did not abort!)
        assert_eq!(result.app_results[2].status, AppLaunchStatus::Done);
    }

    #[test]
    fn test_reentrancy_guard() {
        let engine = ExecutionEngine::new();

        // Runner that simulates a long-running app
        struct SlowRunner {
            counter: Arc<AtomicUsize>,
        }
        impl EngineRunner for SlowRunner {
            fn execute_app(
                &self,
                app: &AppEntry,
                _: &(dyn Fn(AppLaunchStatus) + Send + Sync),
            ) -> AppExecutionResult {
                self.counter.fetch_add(1, Ordering::SeqCst);
                std::thread::sleep(Duration::from_millis(60));
                AppExecutionResult {
                    app_id: app.id.clone(),
                    app_name: app.name.clone(),
                    status: AppLaunchStatus::Done,
                    duration_ms: 60,
                    hwnd: None,
                }
            }
            fn sleep(&self, _: Duration) {}
        }

        let counter = Arc::new(AtomicUsize::new(0));
        let runner = Arc::new(SlowRunner {
            counter: Arc::clone(&counter),
        });

        let mut ws = Workspace::new("Guard Test");
        ws.apps = vec![AppEntry::new("SlowApp", "C:\\slow.exe")];

        let engine_clone = engine.clone();
        let ws_clone = ws.clone();
        let runner_clone = Arc::clone(&runner);

        // Spawn first execution in a background thread
        let handle = std::thread::spawn(move || {
            engine_clone.execute_workspace(&ws_clone, runner_clone, |_, _, _| {})
        });

        // Small sleep so the first thread acquires the lock and begins execution
        std::thread::sleep(Duration::from_millis(15));

        // Attempting to launch the exact same workspace while it is active must fail with AlreadyRunning
        let second_call = engine.execute_workspace(&ws, Arc::clone(&runner), |_, _, _| {});
        assert_eq!(
            second_call.err(),
            Some(EngineError::AlreadyRunning(ws.id.clone()))
        );

        let first_result = handle.join().unwrap();
        assert!(first_result.is_ok());

        // Now that the first launch completed, the guard should have released the lock
        assert!(!engine.is_running(&ws.id));
        let third_call = engine.execute_workspace(&ws, runner, |_, _, _| {});
        assert!(third_call.is_ok());
    }
}
