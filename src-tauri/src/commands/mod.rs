// Commands module

pub mod todo_commands;
pub mod git_commands;
pub mod recording_commands;
pub mod workspace_commands;

use tauri::{AppHandle, Emitter};
use crate::models::event::ToastPayload;

/// Show a toast notification to the frontend
#[tauri::command]
pub fn show_toast(
    app: AppHandle,
    message: String,
    toast_type: Option<String>,
    duration: Option<u32>,
) -> Result<(), String> {
    let payload = ToastPayload {
        message,
        toast_type: toast_type.unwrap_or_else(|| "info".to_string()),
        duration,
    };

    app.emit("show-toast", payload)
        .map_err(|e| format!("Failed to emit toast event: {:?}", e))?;

    Ok(())
}

// Re-export todo commands
pub use todo_commands::{get_todos, add_todo, update_todo, toggle_todo, delete_todo};

// Re-export recording commands
pub use recording_commands::{list_recordings, get_recording, update_recording, delete_recording};

// Re-export workspace commands
pub use workspace_commands::{
    create_workspace, list_workspaces, rename_workspace, 
    delete_workspace, get_current_workspace, set_current_workspace,
    get_workspace_uploads_path
};

