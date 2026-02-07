use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use crate::models::event::{DocumentUpdate, ToastPayload};
use crate::modules::document_service::DocumentService;

pub use crate::utils::paths::{get_app_data_dir, get_recordings_dir, get_state_db_path};

pub fn emit_update(doc_service: &Arc<DocumentService>, app_handle: &AppHandle) {
    let new_snap = doc_service.get_snapshot();
    let _ = app_handle.emit("document-update", DocumentUpdate {
        content: new_snap.content,
        version: new_snap.version,
    });
    // Strict Logic: The moment valid content is emitted, thinking stops.
    let _ = app_handle.emit("agent-status", crate::models::event::AgentStatusPayload { status: "idle".to_string() });
}

pub fn emit_and_save(doc_service: &Arc<DocumentService>, app_handle: &AppHandle, recording_id: Option<&str>) {
    let new_snap = doc_service.get_snapshot();
    
    // Emit to frontend
    let _ = app_handle.emit("document-update", DocumentUpdate {
        content: new_snap.content.clone(),
        version: new_snap.version,
    });

    // Strict Logic: Content updated -> Stop thinking
    let _ = app_handle.emit("agent-status", crate::models::event::AgentStatusPayload { status: "idle".to_string() });
    
    // Save to disk immediately if we have a recording_id
    if let Some(rec_id) = recording_id {
        let recording_path = get_recordings_dir().join(rec_id);
        let doc_path = recording_path.join(format!("{}.md", rec_id));
        
        if let Err(e) = std::fs::create_dir_all(&recording_path) {
            eprintln!("[Auto-save] Failed to create dir: {:?}", e);
        } else if let Err(e) = std::fs::write(&doc_path, &new_snap.content) {
            eprintln!("[Auto-save] Failed to write file: {:?}", e);
        }
    }
}

/// Helper function to emit error toast to frontend
pub fn emit_error_toast(app_handle: &AppHandle, message: impl Into<String>) {
    let payload = ToastPayload::error(message);
    if let Err(e) = app_handle.emit("show-toast", payload) {
        eprintln!("Failed to emit error toast: {:?}", e);
    }
}

/// Helper function to emit warning toast to frontend
pub fn emit_warning_toast(app_handle: &AppHandle, message: impl Into<String>) {
    let payload = ToastPayload::warning(message);
    if let Err(e) = app_handle.emit("show-toast", payload) {
        eprintln!("Failed to emit warning toast: {:?}", e);
    }
}

/// Helper function to emit success toast to frontend
pub fn emit_success_toast(app_handle: &AppHandle, message: impl Into<String>) {
    let payload = ToastPayload::success(message);
    if let Err(e) = app_handle.emit("show-toast", payload) {
        eprintln!("Failed to emit success toast: {:?}", e);
    }
}
