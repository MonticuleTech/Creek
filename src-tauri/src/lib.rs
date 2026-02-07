pub mod state;
pub mod modules;
pub mod models;
pub mod services;
pub mod utils;
pub mod prompts;
pub mod commands;

use tauri::{State, AppHandle, Emitter, Manager};
use state::AppState;
use modules::pipeline::{run_pipeline, PipelineCommand};
use modules::workspace_manager::WorkspaceManager;
use models::event::ToastPayload;
use tokio::sync::mpsc;
use std::sync::Arc;
use tokio::sync::RwLock;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn start_recording(state: State<'_, AppState>, app: AppHandle, recording_id: String) -> Result<(), String> {
    state.pipeline_tx
        .try_send(PipelineCommand::StartRecording { recording_id })
        .map_err(|e| {
            let error_msg = format!("Failed to start recording: {:?}", e);
            let _ = app.emit("show-toast", ToastPayload::error(&error_msg));
            error_msg
        })
}

#[tauri::command]
fn load_recording(state: State<'_, AppState>, app: AppHandle, recording_id: String) -> Result<(), String> {
    state.pipeline_tx
        .try_send(PipelineCommand::LoadRecording { recording_id })
        .map_err(|e| {
            let error_msg = format!("Failed to load recording: {:?}", e);
            let _ = app.emit("show-toast", ToastPayload::error(&error_msg));
            error_msg
        })
}

#[tauri::command]
fn pause_recording(state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    state.pipeline_tx
        .try_send(PipelineCommand::PauseRecording)
        .map_err(|e| {
            let error_msg = format!("Failed to pause recording: {:?}", e);
            let _ = app.emit("show-toast", ToastPayload::error(&error_msg));
            error_msg
        })
}

#[tauri::command]
fn resume_recording(state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    state.pipeline_tx
        .try_send(PipelineCommand::ResumeRecording)
        .map_err(|e| {
            let error_msg = format!("Failed to resume recording: {:?}", e);
            let _ = app.emit("show-toast", ToastPayload::error(&error_msg));
            error_msg
        })
}

#[tauri::command]
fn stop_recording(state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    state.pipeline_tx
        .try_send(PipelineCommand::StopRecording)
        .map_err(|e| {
            let error_msg = format!("Failed to stop recording: {:?}", e);
            let _ = app.emit("show-toast", ToastPayload::error(&error_msg));
            error_msg
        })
}

#[tauri::command]
fn reset_document(state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    state.pipeline_tx
        .try_send(PipelineCommand::ResetDocument)
        .map_err(|e| {
            let error_msg = format!("Failed to reset document: {:?}", e);
            let _ = app.emit("show-toast", ToastPayload::error(&error_msg));
            error_msg
        })
}

#[tauri::command]
fn update_document(state: State<'_, AppState>, app: AppHandle, content: String) -> Result<(), String> {
    state.pipeline_tx
        .try_send(PipelineCommand::UpdateDocument(content))
        .map_err(|e| {
            let error_msg = format!("Failed to update document: {:?}", e);
            let _ = app.emit("show-toast", ToastPayload::error(&error_msg));
            error_msg
        })
}

#[tauri::command]
fn ingest_document(state: State<'_, AppState>, app: AppHandle, filename: String, content: String) -> Result<(), String> {
    state.pipeline_tx
        .try_send(PipelineCommand::IngestDocument { filename, content })
        .map_err(|e| {
            let error_msg = format!("Failed to ingest document: {:?}", e);
            let _ = app.emit("show-toast", ToastPayload::error(&error_msg));
            error_msg
        })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let (tx, rx) = mpsc::channel(32);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .level_for("tao", log::LevelFilter::Error)
                .level_for("tokio_tungstenite", log::LevelFilter::Error)
                .level_for("tungstenite", log::LevelFilter::Error)
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir { file_name: None }),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Webview),
                ])
                .build()
        )
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState { pipeline_tx: tx })
        .setup(|app| {
            let handle = app.handle().clone();
            
            // Initialize workspace manager
            let workspace_manager = WorkspaceManager::new(handle.clone());
            workspace_manager.initialize_default_workspace()
                .expect("Failed to initialize default workspace");
            app.manage(Arc::new(RwLock::new(workspace_manager)));
            
            tauri::async_runtime::spawn(async move {
                // Wait a bit for frontend to load
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                run_pipeline(handle, rx).await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_recording,
            pause_recording,
            resume_recording,
            stop_recording, 
            reset_document,
            update_document,
            ingest_document,
            commands::show_toast,
            commands::todo_commands::get_todos,
            commands::todo_commands::add_todo,
            commands::todo_commands::update_todo,
            commands::todo_commands::toggle_todo,
            commands::todo_commands::delete_todo,
            commands::git_commands::get_git_history,
            commands::git_commands::rollback_to_commit,
            commands::git_commands::undo_last_change,
            commands::recording_commands::list_recordings,
            commands::recording_commands::get_recording,
            commands::recording_commands::update_recording,
            commands::recording_commands::delete_recording,
            commands::recording_commands::create_recording,
            commands::recording_commands::rename_recording,
            commands::workspace_commands::create_workspace,
            commands::workspace_commands::list_workspaces,
            commands::workspace_commands::rename_workspace,
            commands::workspace_commands::delete_workspace,
            commands::workspace_commands::get_current_workspace,
            commands::workspace_commands::set_current_workspace,
            load_recording,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
