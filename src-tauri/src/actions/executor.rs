//! Action execution with streaming output.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;

use once_cell::sync::Lazy;
use parking_lot::Mutex;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::oneshot;

/// Escape a string for safe use in shell commands.
fn shell_escape(s: &str) -> String {
    // Wrap in single quotes, escaping any single quotes within
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Global registry of running actions.
pub static RUNNING_ACTIONS: Lazy<Mutex<HashMap<String, ActionExecutionHandle>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Handle to a running action, used to stop it.
pub struct ActionExecutionHandle {
    /// Channel to signal the task to stop.
    stop_tx: Option<oneshot::Sender<()>>,
}

impl ActionExecutionHandle {
    /// Request the action to stop.
    pub fn stop(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
    }
}

/// Event payload for action output.
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionOutputEvent {
    pub execution_id: String,
    pub chunk: String,
}

/// Event payload for action completion.
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionCompleteEvent {
    pub execution_id: String,
    pub exit_code: Option<i32>,
    pub success: bool,
}

/// Run an action and stream output via Tauri events.
///
/// Returns the execution_id immediately. Output is streamed via `action-output` events.
/// Completion is signaled via `action-complete` event.
pub fn run_action(
    app: AppHandle,
    execution_id: String,
    command: String,
    working_dir: PathBuf,
) -> Result<String, String> {
    let exec_id = execution_id.clone();

    // Create stop channel
    let (stop_tx, stop_rx) = oneshot::channel();

    // Store handle
    {
        let mut actions = RUNNING_ACTIONS.lock();
        actions.insert(
            execution_id.clone(),
            ActionExecutionHandle {
                stop_tx: Some(stop_tx),
            },
        );
    }

    // Spawn the execution task
    let exec_id_clone = execution_id.clone();
    tauri::async_runtime::spawn(async move {
        let result = run_action_inner(&app, &exec_id_clone, &command, &working_dir, stop_rx).await;

        // Clean up handle
        {
            let mut actions = RUNNING_ACTIONS.lock();
            actions.remove(&exec_id_clone);
        }

        // Emit completion event
        let success = result.as_ref().map(|code| *code == Some(0)).unwrap_or(false);
        let exit_code = result.unwrap_or(None);

        let _ = app.emit(
            "action-complete",
            ActionCompleteEvent {
                execution_id: exec_id_clone,
                exit_code,
                success,
            },
        );
    });

    Ok(exec_id)
}

/// Inner async function to run the action.
async fn run_action_inner(
    app: &AppHandle,
    execution_id: &str,
    command: &str,
    working_dir: &PathBuf,
    mut stop_rx: oneshot::Receiver<()>,
) -> Result<Option<i32>, String> {
    // Use interactive login shell to get user's environment
    // cd to the directory first to trigger shell hooks (hermit, nvm, etc.)
    let working_dir_str = working_dir.to_string_lossy();
    // Debug: show shell info before running command
    // Explicitly activate hermit if present in target directory
    let full_command = format!(
        "echo '=== Shell Debug ===' && \
         echo \"Shell: $SHELL\" && \
         echo \"HERMIT_ENV before: $HERMIT_ENV\" && \
         cd {} && \
         echo \"PWD after cd: $PWD\" && \
         if [ -f ./bin/hermit ]; then echo 'Activating hermit...'; eval \"$(./bin/hermit env)\"; fi && \
         echo \"HERMIT_ENV after: $HERMIT_ENV\" && \
         echo \"which node: $(which node 2>/dev/null || echo 'not found')\" && \
         echo \"which pnpm: $(which pnpm 2>/dev/null || echo 'not found')\" && \
         echo '==================' && \
         {}",
        shell_escape(&working_dir_str),
        command
    );

    let mut child = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", &full_command])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
    } else {
        // Use user's preferred shell with interactive login flags
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        Command::new(&shell)
            .args(["-i", "-l", "-c", &full_command])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
    }
    .map_err(|e| format!("Failed to spawn command: {}", e))?;

    let stdout = child.stdout.take().expect("stdout was piped");
    let stderr = child.stderr.take().expect("stderr was piped");

    let app_clone = app.clone();
    let exec_id = execution_id.to_string();

    // Stream stdout
    let exec_id_stdout = exec_id.clone();
    let app_stdout = app_clone.clone();
    let stdout_task = tauri::async_runtime::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = app_stdout.emit(
                "action-output",
                ActionOutputEvent {
                    execution_id: exec_id_stdout.clone(),
                    chunk: format!("{}\n", line),
                },
            );
        }
    });

    // Stream stderr
    let exec_id_stderr = exec_id.clone();
    let app_stderr = app_clone.clone();
    let stderr_task = tauri::async_runtime::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = app_stderr.emit(
                "action-output",
                ActionOutputEvent {
                    execution_id: exec_id_stderr.clone(),
                    chunk: format!("{}\n", line),
                },
            );
        }
    });

    // Wait for completion or stop signal
    tokio::select! {
        status = child.wait() => {
            // Wait for output tasks to finish
            let _ = stdout_task.await;
            let _ = stderr_task.await;

            match status {
                Ok(status) => Ok(status.code()),
                Err(e) => Err(format!("Process error: {}", e)),
            }
        }
        _ = &mut stop_rx => {
            // Stop requested - kill the process
            let _ = child.kill().await;
            let _ = stdout_task.await;
            let _ = stderr_task.await;
            Ok(None) // No exit code when killed
        }
    }
}

/// Stop a running action.
pub fn stop_action(execution_id: &str) -> Result<(), String> {
    let mut actions = RUNNING_ACTIONS.lock();
    if let Some(mut handle) = actions.remove(execution_id) {
        handle.stop();
        Ok(())
    } else {
        Err(format!("Action {} not found or already completed", execution_id))
    }
}
