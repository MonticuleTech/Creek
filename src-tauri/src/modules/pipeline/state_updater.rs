use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use log::{info, warn, error};
use crate::models::event::TodoUpdate;
use crate::modules::document_service::DocumentService;
use crate::modules::{StateManager, GitManager, TodoAgent, TodoOperation};
use crate::services::llm_client::OpenAILikeClient;

use super::utils::emit_warning_toast;

/// Update state, git, and todos after document change
pub async fn update_state_and_git(
    doc_service: &Arc<DocumentService>,
    state_manager: &Arc<StateManager>,
    git_manager: &Arc<GitManager>,
    todo_agent: &Arc<TodoAgent>,
    llm: &Arc<OpenAILikeClient>,
    app_handle: &AppHandle,
    recording_path: Option<&std::path::Path>,
    user_input: &str,
) {
    let snapshot = doc_service.get_snapshot();
    let content = snapshot.content.clone();
    
    // 1. Update document in StateManager
    state_manager.update_document(content.clone());
    
    // 2. Generate focus description (non-blocking, with timeout)
    let state_mgr = state_manager.clone();
    let llm_clone = llm.clone();
    let content_clone = content.clone();
    tokio::spawn(async move {
        if let Err(e) = state_mgr.generate_and_update_focus(&*llm_clone, &content_clone).await {
            let error_msg = format!("Focus generation failed: {:?}", e);
            warn!("{} {}", "[Warning]", error_msg);
            // Don't emit toast for focus generation failure - it's non-critical
        }
    });
    
    // 2.5. Maintain todos using TodoAgent (non-blocking)
    let state_mgr = state_manager.clone();
    let todo_ag = todo_agent.clone();
    let llm_clone = llm.clone();
    let app_clone = app_handle.clone();
    let content_clone = content.clone();
    let user_input = user_input.to_string();
    
    tokio::spawn(async move {
        let current_todos = state_mgr.get_todos();
        let recent_changes = state_mgr.get_state().focus;
        
        // Check if todo list is too long
        let todo_count = current_todos.iter().filter(|t| !t.completed).count();
        if todo_count > 10 {
            warn!("[Todo Agent] Todo list has {} items (> 10), triggering cleanup", todo_count);
        }
        
        match todo_ag.maintain_todos(&*llm_clone, &content_clone, &current_todos, &user_input, &recent_changes).await {
            Ok(operations) => {
                if !operations.is_empty() {
                    info!("[Todo Agent] {} operations", operations.len());
                    
                    for op in operations {
                        match op {
                            TodoOperation::Complete { todo_id } => {
                                if let Err(e) = state_mgr.complete_todo(&todo_id) {
                                    error!("Failed to complete todo: {:?}", e);
                                } else {
                                    info!("  ✓ Completed: {}", todo_id);
                                }
                            }
                            TodoOperation::Update { todo_id, new_desc } => {
                                if let Err(e) = state_mgr.update_todo(&todo_id, new_desc.clone()) {
                                    error!("Failed to update todo: {:?}", e);
                                } else {
                                    info!("  ↻ Updated: {} -> {}", todo_id, new_desc);
                                }
                            }
                            TodoOperation::Delete { todo_id } => {
                                if let Err(e) = state_mgr.delete_todo(&todo_id) {
                                    error!("Failed to delete todo: {:?}", e);
                                } else {
                                    info!("  ✗ Deleted: {}", todo_id);
                                }
                            }
                            TodoOperation::Add { desc } => {
                                let new_id = uuid::Uuid::new_v4().to_string();
                                state_mgr.add_todo(new_id.clone(), desc.clone());
                                info!("  + Added: {} ({})", desc, new_id);
                            }
                        }
                    }
                    
                    // Remove completed todos after operations
                    state_mgr.remove_completed_todos();
                    
                    // Emit todo update to frontend
                    let todos = state_mgr.get_todos();
                    let _ = app_clone.emit("todo-update", TodoUpdate { todos });
                }
            }
            Err(e) => {
                let error_msg = format!("Todo Agent failed: {:?}", e);
                warn!("[Todo Agent] {}", error_msg);
                emit_warning_toast(&app_clone, &error_msg);
            }
        }
    });
    
    // 3. Git commit (if we have a recording)
    if let Some(recording_path) = recording_path {
        let recording_path = recording_path.to_path_buf();

        
        // Get diff and generate commit message (spawn to not block)
        let git_mgr = git_manager.clone();
        let state_mgr = state_manager.clone();
        let llm_clone = llm.clone();
        let content_clone = content.clone();
        let recording_path_clone = recording_path.clone();
        
        tokio::spawn(async move {
            // Write document file first (so we can get diff)
            // Extract recording_id from path
            let rec_id = recording_path_clone.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("document");
            let doc_path = recording_path_clone.join(format!("{}.md", rec_id));
            if let Err(e) = std::fs::write(&doc_path, &content_clone) {
                error!("[Git] Failed to write document: {:?}", e);
                return;
            }
            
            // Get diff
            let diff = match git_mgr.get_diff(&recording_path_clone) {
                Ok(d) => d,
                Err(e) => {
                    let error_msg = format!("Git diff failed: {:?}", e);
                    warn!("[Git] {}", error_msg);
                    // Don't emit toast for git diff failure - it's internal operation
                    return;
                }
            };
            
            // Generate commit message
            let commit_msg = match git_mgr.generate_commit_message(&*llm_clone, &diff).await {
                Ok(msg) => msg,
                Err(e) => {
                    let error_msg = format!("Commit message generation failed: {:?}", e);
                    warn!("[Git] {}", error_msg);
                    // Don't emit toast - use fallback message
                    "Document updated".to_string()
                }
            };
            
            // Commit (file already written, just need to git add & commit)
            if let Err(e) = git_mgr.commit_existing(&recording_path_clone, &commit_msg) {
                let error_msg = format!("Git commit failed: {:?}", e);
                error!("[Git] {}", error_msg);
                // Don't emit toast for git commit failure - it's internal operation
                return;
            }
            
            // Update git history in StateManager
            state_mgr.add_git_history(commit_msg.clone());
            
            // Persist state
            if let Err(e) = state_mgr.persist_state() {
                let error_msg = format!("State persistence failed: {:?}", e);
                warn!("[State Manager] {}", error_msg);
                // Don't emit toast for state persistence failure - it's internal operation
            }
            
            info!("[Committed] {}", commit_msg);
        });
    }
}
