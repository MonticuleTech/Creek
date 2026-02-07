// Git Commands - Tauri commands for version control

use tauri::{State, AppHandle, Emitter};
use crate::state::AppState;
use crate::modules::pipeline::PipelineCommand;
use crate::models::event::ToastPayload;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub hash: String,
    pub message: String,
}

/// Get git commit history for current recording
#[tauri::command]
pub async fn get_git_history(app_handle: AppHandle) -> Result<Vec<CommitInfo>, String> { // Now async
    use crate::utils::paths::{get_state_db_path, get_recordings_dir};
    use rusqlite::Connection;
    use crate::modules::{GitManager, WorkspaceManager};
    use std::sync::Arc;
    use tauri::Manager;

    let db_path = get_state_db_path();
    let conn = Connection::open(db_path)
        .map_err(|e| format!("Failed to open DB: {}", e))?;
    
    // 1. Get active recording ID from DB
    let active_id: Option<String> = conn.query_row(
        "SELECT recording_id FROM active_recording WHERE id = 1",
        [],
        |row| row.get(0),
    ).ok();
    
    if let Some(rec_id) = active_id {
        // 2. Resolve Path (Workspace vs Global)
        let recording_path = {
            let workspace_manager = app_handle.state::<Arc<tokio::sync::RwLock<WorkspaceManager>>>();
            let manager = workspace_manager.read().await;
            if let Ok(Some(workspace)) = manager.get_current_workspace() {
                 workspace.path.join("recordings").join(&rec_id)
            } else {
                 get_recordings_dir().join(&rec_id) // Fallback to global
            }
        };

        let git_manager = GitManager::new();
        // Fetch last 50 commits
        match git_manager.get_history_with_hashes(&recording_path, 50) {
            Ok(history) => {
                let infos = history.into_iter().map(|(hash, message)| CommitInfo {
                    hash,
                    message,
                }).collect();
                Ok(infos)
            }
            Err(_e) => {
                // If repo doesn't exist yet (new recording), return empty
                // Don't error out, just return empty list to avoid frightening users
                println!("No git repo found at {:?} (new recording?)", recording_path);
                Ok(vec![])
            }
        }
    } else {
        Ok(vec![])
    }
}

/// Rollback to a specific commit
#[tauri::command]
pub fn rollback_to_commit(
    state: State<'_, AppState>, 
    app: AppHandle,
    commit_hash: String
) -> Result<(), String> {
    state.pipeline_tx
        .try_send(PipelineCommand::RollbackToCommit(commit_hash))
        .map_err(|e| {
            let error_msg = format!("Failed to send rollback command: {:?}", e);
            let _ = app.emit("show-toast", ToastPayload::error(&error_msg));
            error_msg
        })
}

/// Rollback to previous commit (shorthand for "undo")
#[tauri::command]
pub fn undo_last_change(state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    state.pipeline_tx
        .try_send(PipelineCommand::UndoLastChange)
        .map_err(|e| {
            let error_msg = format!("Failed to send undo command: {:?}", e);
            let _ = app.emit("show-toast", ToastPayload::error(&error_msg));
            error_msg
        })
}
