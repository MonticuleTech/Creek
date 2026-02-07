// Todo Management Commands

use tauri::{AppHandle, Emitter, State};
use crate::modules::TodoItem;
use crate::models::event::ToastPayload;
use crate::state::AppState;
use crate::modules::pipeline::PipelineCommand;
use crate::utils::paths::get_state_db_path;
use rusqlite::Connection;

/// Get current todo list directly from DB (READ Path)
#[tauri::command]
pub fn get_todos(_app: AppHandle) -> Result<Vec<TodoItem>, String> {
    let db_path = get_state_db_path();
    let conn = Connection::open(db_path)
        .map_err(|e| format!("Failed to open DB: {}", e))?;
    
    // 1. Get active recording ID
    let active_id: Option<String> = conn.query_row(
        "SELECT recording_id FROM active_recording WHERE id = 1",
        [],
        |row| row.get(0),
    ).ok(); // Ignore error if no active recording
    
    // 2. If valid ID, fetch todos from document_states
    if let Some(rec_id) = active_id {
        let todo_json: Option<String> = conn.query_row(
            "SELECT todo_list FROM document_states WHERE recording_id = ?1",
            [&rec_id],
            |row| row.get(0),
        ).ok();
        
        if let Some(json) = todo_json {
             let todos: Vec<TodoItem> = serde_json::from_str(&json)
                 .unwrap_or_default();
             return Ok(todos);
        }
    }
    
    // Default: empty list
    Ok(vec![])
}

/// Add a new todo (user manual operation)
#[tauri::command]
pub fn add_todo(desc: String, state: State<'_, AppState>, app: AppHandle) -> Result<String, String> {
    // Generate ID for frontend optimistically? 
    // Actually the pipeline generates it again in my current implementation.
    // I should probably generate it here and pass it, OR let pipeline do it and we don't return it immediately.
    // But frontend usually likes to have the ID.
    // However, PipelineCommand::AddTodo(desc) handles logic.
    // Let's stick to fire-and-forget for now, frontend refreshes on update.
    
    state.pipeline_tx
        .try_send(PipelineCommand::AddTodo(desc))
        .map_err(|e| {
            let error_msg = format!("Failed to send add_todo command: {:?}", e);
            let _ = app.emit("show-toast", ToastPayload::error(&error_msg));
            error_msg
        })?;

    Ok("sent".to_string())
}

/// Update todo description (user manual operation)
#[tauri::command]
pub fn update_todo(id: String, desc: String, state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    state.pipeline_tx
        .try_send(PipelineCommand::UpdateTodo { id, description: desc })
        .map_err(|e| {
            let error_msg = format!("Failed to send update_todo command: {:?}", e);
            let _ = app.emit("show-toast", ToastPayload::error(&error_msg));
            error_msg
        })
}

/// Toggle todo completion status (user manual operation)
#[tauri::command]
pub fn toggle_todo(id: String, state: State<'_, AppState>, app: AppHandle) -> Result<bool, String> {
    state.pipeline_tx
        .try_send(PipelineCommand::ToggleTodo(id))
        .map_err(|e| {
            let error_msg = format!("Failed to send toggle_todo command: {:?}", e);
            let _ = app.emit("show-toast", ToastPayload::error(&error_msg));
            error_msg
        })?;
    Ok(true) 
}

/// Delete a todo (user manual operation)
#[tauri::command]
pub fn delete_todo(id: String, state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    state.pipeline_tx
        .try_send(PipelineCommand::DeleteTodo(id))
        .map_err(|e| {
            let error_msg = format!("Failed to send delete_todo command: {:?}", e);
            let _ = app.emit("show-toast", ToastPayload::error(&error_msg));
            error_msg
        })
}
