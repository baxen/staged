use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};
use std::io::{BufRead, BufReader};
use std::thread;

use crate::store::Store;

/// Event emitted when action output is produced
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionOutputEvent {
    pub execution_id: String,
    pub chunk: String,
    pub stream: String, // "stdout" or "stderr"
}

/// Event emitted when action status changes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionStatusEvent {
    pub execution_id: String,
    pub branch_id: String,
    pub action_id: String,
    pub action_name: String,
    pub status: ActionStatus,
    pub exit_code: Option<i32>,
    pub started_at: i64,
    pub completed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionStatus {
    Running,
    Completed,
    Failed,
    Stopped,
}

/// Tracks a running action
struct RunningActionState {
    execution_id: String,
    action_id: String,
    action_name: String,
    branch_id: String,
    started_at: i64,
    #[allow(dead_code)]
    child_pid: Option<u32>,
}

/// Manages action execution
pub struct ActionRunner {
    running: Arc<Mutex<HashMap<String, RunningActionState>>>,
}

impl ActionRunner {
    pub fn new() -> Self {
        Self {
            running: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Execute an action in the given worktree directory
    pub fn run_action(
        &self,
        app: AppHandle,
        store: Arc<Store>,
        branch_id: String,
        action_id: String,
        worktree_path: String,
    ) -> Result<String> {
        let execution_id = uuid::Uuid::new_v4().to_string();

        // Get the action from store
        let action = store
            .get_project_action(&action_id)?
            .context("Action not found")?;

        // Determine which shell to use
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());

        // Start the process
        let mut child = Command::new(&shell)
            .arg("-l") // Login shell to inherit PATH and environment
            .arg("-c")
            .arg(&action.command)
            .current_dir(&worktree_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn action process")?;

        let child_pid = child.id();

        // Record the running action
        {
            let mut running = self.running.lock().unwrap();
            running.insert(
                execution_id.clone(),
                RunningActionState {
                    execution_id: execution_id.clone(),
                    action_id: action_id.clone(),
                    action_name: action.name.clone(),
                    branch_id: branch_id.clone(),
                    started_at: crate::store::now_timestamp(),
                    child_pid: Some(child_pid),
                },
            );
        }

        // Emit initial status event
        let _ = app.emit(
            "action_status",
            ActionStatusEvent {
                execution_id: execution_id.clone(),
                branch_id: branch_id.clone(),
                action_id: action_id.clone(),
                action_name: action.name.clone(),
                status: ActionStatus::Running,
                exit_code: None,
                started_at: crate::store::now_timestamp(),
                completed_at: None,
            },
        );

        // Spawn threads to read stdout and stderr
        let exec_id = execution_id.clone();
        let app_clone = app.clone();
        if let Some(stdout) = child.stdout.take() {
            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        let _ = app_clone.emit(
                            "action_output",
                            ActionOutputEvent {
                                execution_id: exec_id.clone(),
                                chunk: format!("{}\n", line),
                                stream: "stdout".to_string(),
                            },
                        );
                    }
                }
            });
        }

        let exec_id = execution_id.clone();
        let app_clone = app.clone();
        if let Some(stderr) = child.stderr.take() {
            thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        let _ = app_clone.emit(
                            "action_output",
                            ActionOutputEvent {
                                execution_id: exec_id.clone(),
                                chunk: format!("{}\n", line),
                                stream: "stderr".to_string(),
                            },
                        );
                    }
                }
            });
        }

        // Spawn thread to wait for completion
        let exec_id = execution_id.clone();
        let running_clone = self.running.clone();
        let app_clone = app.clone();
        let _store_clone = store.clone();
        let branch_id_clone = branch_id.clone();
        let worktree_path_clone = worktree_path.clone();
        let auto_commit = action.auto_commit;
        let action_name = action.name.clone();

        thread::spawn(move || {
            let exit_status = child.wait();
            let exit_code = exit_status.as_ref().ok().and_then(|s| s.code());
            let completed_at = crate::store::now_timestamp();

            // Remove from running actions
            {
                let mut running = running_clone.lock().unwrap();
                running.remove(&exec_id);
            }

            let success = exit_status.as_ref().map(|s| s.success()).unwrap_or(false);

            // Emit completion status
            let _ = app_clone.emit(
                "action_status",
                ActionStatusEvent {
                    execution_id: exec_id.clone(),
                    branch_id: branch_id_clone.clone(),
                    action_id: action_id.clone(),
                    action_name: action_name.clone(),
                    status: if success {
                        ActionStatus::Completed
                    } else {
                        ActionStatus::Failed
                    },
                    exit_code,
                    started_at: crate::store::now_timestamp(), // Will be overridden by frontend
                    completed_at: Some(completed_at),
                },
            );

            // If auto_commit is enabled and action succeeded, commit changes
            if auto_commit && success {
                if let Err(e) = Self::auto_commit_changes(
                    &worktree_path_clone,
                    &action_name,
                ) {
                    eprintln!("Failed to auto-commit changes: {}", e);
                } else {
                    // Emit event to notify frontend of the commit
                    let _ = app_clone.emit(
                        "action_auto_commit",
                        serde_json::json!({
                            "executionId": exec_id,
                            "branchId": branch_id_clone,
                            "actionName": action_name,
                        }),
                    );
                }
            }
        });

        Ok(execution_id)
    }

    /// Auto-commit changes after a successful action
    fn auto_commit_changes(worktree_path: &str, action_name: &str) -> Result<()> {
        // Check if there are any changes
        let status = Command::new("git")
            .arg("diff")
            .arg("--exit-code")
            .current_dir(worktree_path)
            .status()?;

        // If exit code is 0, no changes exist
        if status.success() {
            return Ok(());
        }

        // Stage all changes
        Command::new("git")
            .args(["add", "-A"])
            .current_dir(worktree_path)
            .status()
            .context("Failed to stage changes")?;

        // Commit with action name
        let commit_message = format!("chore: {}", action_name);
        Command::new("git")
            .args(["commit", "-m", &commit_message])
            .current_dir(worktree_path)
            .status()
            .context("Failed to commit changes")?;

        Ok(())
    }

    /// Stop a running action
    pub fn stop_action(&self, execution_id: &str) -> Result<()> {
        let state = {
            let mut running = self.running.lock().unwrap();
            running.remove(execution_id)
        };

        if let Some(state) = state {
            if let Some(pid) = state.child_pid {
                // Kill the process
                #[cfg(unix)]
                {
                    unsafe {
                        libc::kill(pid as i32, libc::SIGTERM);
                    }
                }

                #[cfg(windows)]
                {
                    Command::new("taskkill")
                        .args(["/PID", &pid.to_string(), "/F"])
                        .status()?;
                }
            }
        }

        Ok(())
    }

    /// Get all running actions for a branch
    pub fn get_running_actions(&self, branch_id: &str) -> Vec<ActionStatusEvent> {
        let running = self.running.lock().unwrap();
        running
            .values()
            .filter(|state| state.branch_id == branch_id)
            .map(|state| ActionStatusEvent {
                execution_id: state.execution_id.clone(),
                branch_id: state.branch_id.clone(),
                action_id: state.action_id.clone(),
                action_name: state.action_name.clone(),
                status: ActionStatus::Running,
                exit_code: None,
                started_at: state.started_at,
                completed_at: None,
            })
            .collect()
    }
}
