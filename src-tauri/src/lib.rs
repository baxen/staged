pub mod git;
mod refresh;
pub mod review;
mod watcher;

use git::{CommitResult, DiscardRange, FileDiff, GitStatus};
use refresh::RefreshController;
use review::{Comment, DiffId, Edit, NewComment, NewEdit, Review, ReviewStore};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State};

// =============================================================================
// Tauri Commands
// =============================================================================

#[tauri::command]
fn get_git_status(path: Option<String>) -> Result<GitStatus, String> {
    git::get_status(path.as_deref()).map_err(|e| e.message)
}

#[tauri::command]
fn open_repository(path: String) -> Result<GitStatus, String> {
    git::get_status(Some(&path)).map_err(|e| e.message)
}

#[tauri::command]
fn get_file_diff(
    repo_path: Option<String>,
    file_path: String,
    staged: bool,
) -> Result<FileDiff, String> {
    git::get_file_diff(repo_path.as_deref(), &file_path, staged).map_err(|e| e.message)
}

#[tauri::command]
fn get_untracked_file_diff(
    repo_path: Option<String>,
    file_path: String,
) -> Result<FileDiff, String> {
    git::get_untracked_file_diff(repo_path.as_deref(), &file_path).map_err(|e| e.message)
}

#[tauri::command]
fn stage_file(repo_path: Option<String>, file_path: String) -> Result<(), String> {
    git::stage_file(repo_path.as_deref(), &file_path).map_err(|e| e.message)
}

#[tauri::command]
fn unstage_file(repo_path: Option<String>, file_path: String) -> Result<(), String> {
    git::unstage_file(repo_path.as_deref(), &file_path).map_err(|e| e.message)
}

#[tauri::command]
fn discard_file(repo_path: Option<String>, file_path: String) -> Result<(), String> {
    git::discard_file(repo_path.as_deref(), &file_path).map_err(|e| e.message)
}

#[tauri::command]
fn stage_all(repo_path: Option<String>) -> Result<(), String> {
    git::stage_all(repo_path.as_deref()).map_err(|e| e.message)
}

#[tauri::command]
fn unstage_all(repo_path: Option<String>) -> Result<(), String> {
    git::unstage_all(repo_path.as_deref()).map_err(|e| e.message)
}

#[tauri::command]
fn get_last_commit_message(repo_path: Option<String>) -> Result<Option<String>, String> {
    git::get_last_commit_message(repo_path.as_deref()).map_err(|e| e.message)
}

#[tauri::command]
fn create_commit(repo_path: Option<String>, message: String) -> Result<CommitResult, String> {
    git::create_commit(repo_path.as_deref(), &message).map_err(|e| e.message)
}

#[tauri::command]
fn amend_commit(repo_path: Option<String>, message: String) -> Result<CommitResult, String> {
    git::amend_commit(repo_path.as_deref(), &message).map_err(|e| e.message)
}

/// Discard specific lines from a file.
///
/// Takes line ranges from the UI's source_lines data and reverts those
/// specific changes, allowing fine-grained control over what to discard.
#[tauri::command]
fn discard_lines(
    repo_path: Option<String>,
    file_path: String,
    old_start: Option<u32>,
    old_end: Option<u32>,
    new_start: Option<u32>,
    new_end: Option<u32>,
    staged: bool,
) -> Result<(), String> {
    let range = DiscardRange {
        old_start,
        old_end,
        new_start,
        new_end,
    };
    git::discard_lines(repo_path.as_deref(), &file_path, range, staged).map_err(|e| e.message)
}

// =============================================================================
// Review Commands
// =============================================================================

#[tauri::command]
fn get_review(base: String, head: String) -> Result<Review, String> {
    let store = review::get_store().map_err(|e| e.message)?;
    let id = DiffId::new(base, head);
    store.get_or_create(&id).map_err(|e| e.message)
}

#[tauri::command]
fn add_comment(base: String, head: String, comment: NewComment) -> Result<Comment, String> {
    let store = review::get_store().map_err(|e| e.message)?;
    let id = DiffId::new(base, head);
    let comment = Comment::new(comment.file_path, comment.range_index, comment.text);
    store.add_comment(&id, &comment).map_err(|e| e.message)?;
    Ok(comment)
}

#[tauri::command]
fn delete_comment(base: String, head: String, comment_id: String) -> Result<(), String> {
    let store = review::get_store().map_err(|e| e.message)?;
    let id = DiffId::new(base, head);
    let uuid =
        uuid::Uuid::parse_str(&comment_id).map_err(|e| format!("Invalid comment ID: {}", e))?;
    store.delete_comment(&id, uuid).map_err(|e| e.message)
}

#[tauri::command]
fn mark_reviewed(base: String, head: String, file_path: String) -> Result<(), String> {
    let store = review::get_store().map_err(|e| e.message)?;
    let id = DiffId::new(base, head);
    store.mark_reviewed(&id, &file_path).map_err(|e| e.message)
}

#[tauri::command]
fn unmark_reviewed(base: String, head: String, file_path: String) -> Result<(), String> {
    let store = review::get_store().map_err(|e| e.message)?;
    let id = DiffId::new(base, head);
    store
        .unmark_reviewed(&id, &file_path)
        .map_err(|e| e.message)
}

#[tauri::command]
fn record_edit(base: String, head: String, edit: NewEdit) -> Result<Edit, String> {
    let store = review::get_store().map_err(|e| e.message)?;
    let id = DiffId::new(base, head);
    let edit = Edit::new(edit.file_path, edit.diff);
    store.add_edit(&id, &edit).map_err(|e| e.message)?;
    Ok(edit)
}

#[tauri::command]
fn export_review_markdown(base: String, head: String) -> Result<String, String> {
    let store = review::get_store().map_err(|e| e.message)?;
    let id = DiffId::new(base, head);
    let review = store.get_or_create(&id).map_err(|e| e.message)?;
    Ok(review::export_markdown(&review))
}

#[tauri::command]
fn clear_review(base: String, head: String) -> Result<(), String> {
    let store = review::get_store().map_err(|e| e.message)?;
    let id = DiffId::new(base, head);
    store.delete(&id).map_err(|e| e.message)
}

// =============================================================================
// Watcher Commands
// =============================================================================

/// State container for the refresh controller.
/// Wrapped in Option because it's created during setup with the AppHandle.
struct RefreshControllerState(Mutex<Option<RefreshController>>);

#[tauri::command]
async fn start_watching(
    repo_path: String,
    state: State<'_, RefreshControllerState>,
) -> Result<(), String> {
    let controller = state.0.lock().unwrap();
    if let Some(ref ctrl) = *controller {
        ctrl.start(PathBuf::from(repo_path))
    } else {
        Err("Refresh controller not initialized".to_string())
    }
}

#[tauri::command]
fn stop_watching(state: State<RefreshControllerState>) -> Result<(), String> {
    let controller = state.0.lock().unwrap();
    if let Some(ref ctrl) = *controller {
        ctrl.stop();
        Ok(())
    } else {
        Err("Refresh controller not initialized".to_string())
    }
}

#[tauri::command]
fn force_refresh(state: State<RefreshControllerState>) -> Result<GitStatus, String> {
    let controller = state.0.lock().unwrap();
    if let Some(ref ctrl) = *controller {
        ctrl.force_refresh()
    } else {
        Err("Refresh controller not initialized".to_string())
    }
}

// =============================================================================
// Tauri App Setup
// =============================================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(RefreshControllerState(Mutex::new(None)))
        .setup(|app| {
            // Initialize the refresh controller with the app handle
            let controller = RefreshController::new(app.handle().clone());
            let state: State<RefreshControllerState> = app.state();
            *state.0.lock().unwrap() = Some(controller);

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Git commands
            get_git_status,
            open_repository,
            get_file_diff,
            get_untracked_file_diff,
            stage_file,
            unstage_file,
            discard_file,
            stage_all,
            unstage_all,
            discard_lines,
            get_last_commit_message,
            create_commit,
            amend_commit,
            // Review commands
            get_review,
            add_comment,
            delete_comment,
            mark_reviewed,
            unmark_reviewed,
            record_edit,
            export_review_markdown,
            clear_review,
            // Watcher commands
            start_watching,
            stop_watching,
            force_refresh
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
