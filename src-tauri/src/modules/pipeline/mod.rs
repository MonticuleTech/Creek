use std::sync::Arc;
use tokio::sync::mpsc;
use tauri::{AppHandle, Emitter, Manager};
use tokio_util::sync::CancellationToken;
use tokio::time::{Duration, Instant};

pub mod types;
pub mod utils;
pub mod state_updater;
pub mod transcript_processor;
pub mod auto_naming;

pub use types::{PipelineCommand, SpeechAggregator, FLUSH_TIMEOUT_MS};
use crate::utils::paths::{get_app_data_dir, get_state_db_path};
use self::utils::{emit_error_toast, emit_warning_toast, emit_success_toast, emit_update};
use transcript_processor::process_transcript;
use log::{info, error, warn};

use crate::models::event::DocumentUpdate;
use crate::modules::document_service::DocumentService;
use crate::modules::{StateManager, GitManager, TodoAgent, RagService, IntentRouter, WorkspaceManager};
use crate::services::asr_service::AsrService;
use crate::services::llm_client::OpenAILikeClient;

/// Helper to get the recordings directory for the CURRENT workspace.
/// If no workspace is active or error occurs, falls back to global (legacy) or empty path,
/// but mostly tries to enforce workspace isolation.
async fn get_current_workspace_recordings_dir(app_handle: &AppHandle) -> Option<std::path::PathBuf> {
    let workspace_manager = app_handle.state::<Arc<tokio::sync::RwLock<WorkspaceManager>>>();
    let manager = workspace_manager.read().await;
    match manager.get_current_workspace() {
        Ok(Some(workspace)) => Some(workspace.path.join("recordings")),
        _ => None,
    }
}

pub async fn run_pipeline(app_handle: AppHandle, mut cmd_rx: mpsc::Receiver<PipelineCommand>) {
    // Start with a clean, empty canvas (no default title/template)
    let initial_content = String::new();
    
    let api_key = std::env::var("OPENAI_API_KEY")
        .or_else(|_| std::env::var("DASHSCOPE_API_KEY"))
        .unwrap_or_else(|_| {
            warn!("OPENAI_API_KEY (or DASHSCOPE_API_KEY) not found. Pipeline will fail.");
            "sk-placeholder".to_string()
        });
    
    // Ensure app data directory exists
    let app_data_dir = get_app_data_dir();
    std::fs::create_dir_all(&app_data_dir).ok();
    
    // Initialize StateManager, GitManager, and TodoAgent
    let state_manager = Arc::new(
        StateManager::new(&get_state_db_path().to_string_lossy())
            .unwrap_or_else(|e| {
                let error_msg = format!("State Manager initialization failed: {:?}", e);
                warn!("{}", error_msg);
                emit_error_toast(&app_handle, &error_msg);
                // Create in-memory fallback (will not persist)
                StateManager::new(":memory:").expect("Failed to create in-memory StateManager")
            })
    );
    let git_manager = Arc::new(GitManager::new());
    let todo_agent = Arc::new(TodoAgent::new());
    let intent_router = Arc::new(IntentRouter::new(api_key.clone()));
    
    // Services
    let doc_service = Arc::new(DocumentService::new(initial_content.clone()));
    
    // LLM Client:
    let llm_flash = Arc::new(OpenAILikeClient::new(
        "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string(),
        api_key.clone(),
        "qwen-flash".to_string(),
    ));
    
    let llm_coder = Arc::new(OpenAILikeClient::new(
        "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string(),
        api_key.clone(),
        "qwen3-coder-flash".to_string(),
    ));
    
    // Initialize RAG Service
    let rag_db_path = get_app_data_dir().join("rag_db.lance");
    let rag_service = match RagService::new(rag_db_path).await {
        Ok(svc) => Arc::new(svc),
        Err(e) => {
            let error_msg = format!("RAG Service initialization failed: {:?}", e);
            error!("{}", error_msg);
            emit_error_toast(&app_handle, &error_msg);
            panic!("RagService failed to initialize: {:?}", e);
        }
    };
    
    
    // ASR State
    let mut asr_cancellation_token: Option<CancellationToken> = None;
    let mut processing_cancellation_token: Option<CancellationToken> = None;
    let mut is_paused = false;
    let (asr_tx, mut asr_rx) = mpsc::unbounded_channel::<String>();
    let mut asr = AsrService::new(api_key);
    asr.set_callback(asr_tx);
    let mut speech_agg = SpeechAggregator::default();

    // Chat History State
    let chat_history = Arc::new(tokio::sync::RwLock::new(Vec::new()));
    
    // Recording State
    let mut current_recording_id: Option<String> = None;

    // Initial Emit
    let _ = app_handle.emit("document-update", DocumentUpdate {
        content: initial_content,
        version: 0,
    });

    // Recovery: check for last active recording on startup
    // DISABLED: Causing auto-recording bug without frontend sync.
    if let Ok(db) = rusqlite::Connection::open(get_state_db_path()) {
        // Clear any stuck active state to prevent issues
        let _ = db.execute("UPDATE active_recording SET recording_id = NULL WHERE id = 1", []);
    }

    info!("Pipeline Ready. Waiting for commands...");

    let mut flush_deadline: Option<Instant> = None;
    let mut holdback_deadline: Option<Instant> = None;
    let mut pending_transcript = String::new();
    const HOLDBACK_MS: u64 = 450; 

    loop {
        let timeout_fut = async {
            let deadline = match (flush_deadline, holdback_deadline) {
                (Some(f), Some(h)) => Some(f.min(h)),
                (f, h) => f.or(h),
            };

            if let Some(deadline) = deadline {
                tokio::time::sleep_until(deadline).await;
            } else {
                std::future::pending::<()>().await;
            }
        };

        tokio::select! {
            // Handle Commands from Frontend
            Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    PipelineCommand::StartRecording { recording_id } => {
                        flush_deadline = None;
                        is_paused = false;
                        if asr_cancellation_token.is_none() {
                            // Get workspace recordings dir
                            if let Some(recordings_dir) = get_current_workspace_recordings_dir(&app_handle).await {
                                info!("Starting Recording... ID: {}", recording_id);

                                // Clear history from previous session
                                chat_history.write().await.clear();
                                
                                current_recording_id = Some(recording_id.clone());
                                
                                let recording_path = recordings_dir.join(&recording_id);
                                let doc_path = recording_path.join(format!("{}.md", &recording_id));
                                
                                // Load existing content if available
                                if doc_path.exists() {
                                    if let Ok(existing_content) = std::fs::read_to_string(&doc_path) {
                                        info!("Loading existing recording content: {} chars", existing_content.len());
                                        doc_service.reset(existing_content.clone());
                                        state_manager.update_document(existing_content);
                                        emit_update(&doc_service, &app_handle);
                                    }
                                }
                                
                                if let Err(e) = state_manager.set_current_recording(recording_id.clone()) {
                                    let error_msg = format!("Failed to set recording in State Manager: {:?}", e);
                                    warn!("{}", error_msg);
                                    emit_warning_toast(&app_handle, &error_msg);
                                }
                                
                                let state = state_manager.get_state();
                                info!("[Initial State]");
                                info!("  Document: {} chars", state.current_doc.len());
                                info!("  Focus: {}", if state.focus.is_empty() { "(empty)" } else { &state.focus });
                                info!("  Git history: {} entries", state.git_history.len());
                                info!("  Todos: {}", state.todo_list.len());
                                
                                let rag_clone = rag_service.clone();
                                let rec_id_clone = recording_id.clone();
                                let app_clone = app_handle.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = rag_clone.init_recording(&rec_id_clone).await {
                                        let error_msg = format!("Failed to initialize RAG recording: {:?}", e);
                                        warn!("{}", error_msg);
                                        emit_warning_toast(&app_clone, &error_msg);
                                    }
                                });
                                
                                // Init Git in correct path
                                std::fs::create_dir_all(&recording_path).ok();
                                if let Err(e) = git_manager.init_repo(&recording_path) {
                                    let error_msg = format!("Failed to initialize Git repository: {:?}", e);
                                    warn!("{}", error_msg);
                                    emit_error_toast(&app_handle, &error_msg);
                                }
                                
                                let token = CancellationToken::new();
                                asr_cancellation_token = Some(token.clone());
                                let asr_clone = asr.clone();
                                let app_clone_asr = app_handle.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = asr_clone.start_recording(token, app_clone_asr).await {
                                        error!("ASR Error: {}", e);
                                    }
                                });
                            } else {
                                let error_msg = "Cannot start recording: No active workspace found";
                                error!("{}", error_msg);
                                emit_error_toast(&app_handle, error_msg);
                            }
                        }
                    }
                    PipelineCommand::PauseRecording => {
                        info!("Pausing Recording and all processing...");
                        is_paused = true;
                        
                        // Cancel any ongoing processing
                        if let Some(token) = processing_cancellation_token.take() {
                            token.cancel();
                            info!("[Processing Cancelled] All agent tasks aborted");
                        }
                        flush_deadline = None;
                    }
                    PipelineCommand::ResumeRecording => {
                        info!("Resuming Recording...");
                        is_paused = false;
                    }
                    PipelineCommand::StopRecording => {
                        info!("Stopping Recording...");
                        
                        if let Some(rec_id) = &current_recording_id {
                             if let Some(recordings_dir) = get_current_workspace_recordings_dir(&app_handle).await {
                                let recording_path = recordings_dir.join(rec_id);
                                let current_content = doc_service.get_snapshot().content;
                                
                                if let Err(e) = std::fs::create_dir_all(&recording_path) {
                                    let error_msg = format!("Failed to create recording directory: {:?}", e);
                                    error!("[Save Failed] {}", error_msg);
                                    emit_error_toast(&app_handle, &error_msg);
                                } else {
                                    let doc_path = recording_path.join(format!("{}.md", rec_id));
                                    if let Err(e) = std::fs::write(&doc_path, &current_content) {
                                        let error_msg = format!("Failed to save final document: {:?}", e);
                                        error!("[Save Failed] {}", error_msg);
                                        emit_error_toast(&app_handle, &error_msg);
                                    } else {
                                        info!("[Document Saved] {}", doc_path.display());
                                    }
                                }
                            }
                        }
                        
                        let state = state_manager.get_state();
                        info!("[Final State]");
                        info!("  Document: {} chars", state.current_doc.len());
                        info!("  Focus: {}", if state.focus.is_empty() { "(empty)" } else { &state.focus });
                        info!("  Git history: {} entries", state.git_history.len());
                        info!("  Todos: {}", state.todo_list.len());
                        
                        flush_deadline = None;
                        is_paused = false;
                        
                        // Cancel ASR
                        if let Some(token) = asr_cancellation_token.take() {
                            token.cancel();
                            info!("[ASR Cancelled]");
                        }
                        
                        // Cancel any ongoing processing
                        if let Some(token) = processing_cancellation_token.take() {
                            token.cancel();
                            info!("[Processing Cancelled] All agent tasks aborted");
                        }

                        if let Err(e) = state_manager.persist_state() {
                            error!("Failed to persist state on stop: {:?}", e);
                        }
                        
                        if let Err(e) = state_manager.clear_current_recording() {
                            let error_msg = format!("Failed to clear recording in State Manager: {:?}", e);
                            warn!("{}", error_msg);
                            emit_warning_toast(&app_handle, &error_msg);
                        }
                        current_recording_id = None;
                    }
                    PipelineCommand::ResetDocument => {
                        info!("Resetting Document (Hard Reset)...");
                        flush_deadline = None;
                        chat_history.write().await.clear();
                        doc_service.reset(String::new());
                        state_manager.update_document(String::new());
                        emit_update(&doc_service, &app_handle);
                        
                        if let Some(rec_id) = &current_recording_id {
                            warn!("[Hard Reset] Clearing data for: {}", rec_id);
                            
                            // 1. Clear RAG Data (Async)
                            let rag_clone = rag_service.clone();
                            let rec_id_clone = rec_id.clone();
                            tokio::spawn(async move {
                                if let Err(e) = rag_clone.delete_recording(&rec_id_clone).await {
                                    error!("Failed to delete RAG tables during reset: {:?}", e);
                                }
                            });
                            
                            // 2. Clear Persistence (SQL State)
                            if let Err(e) = state_manager.reset_recording_state(rec_id) {
                                error!("Failed to reset recording state: {:?}", e);
                            }

                            // 3. Clear File & Git (Physical Data) - FIX IMPLEMENTED HERE
                            if let Some(recordings_dir) = get_current_workspace_recordings_dir(&app_handle).await {
                                let recording_path = recordings_dir.join(rec_id);
                                let doc_path = recording_path.join(format!("{}.md", rec_id));
                                let git_path = recording_path.join(".git");

                                // A. Truncate file
                                if let Err(e) = std::fs::write(&doc_path, "") {
                                    error!("Failed to clear document file: {:?}", e);
                                }

                                // B. Remove Git Repo
                                if git_path.exists() {
                                    if let Err(e) = std::fs::remove_dir_all(&git_path) {
                                        error!("Failed to remove git repo: {:?}", e);
                                    }
                                }

                                // C. Re-init Git Repo
                                if let Err(e) = git_manager.init_repo(&recording_path) {
                                    error!("Failed to re-init git repo after reset: {:?}", e);
                                }
                            }
                            
                            let _ = app_handle.emit("show-toast", crate::models::event::ToastPayload::success("Document and memory cleared"));
                        }
                    }
                    PipelineCommand::UpdateDocument(new_content) => {
                        info!("[Manual Edit]");
                        
                        // 1. Update StateManager (Memory)
                        state_manager.update_document(new_content.clone());
                        
                        // 2. Update DocumentService (Memory)
                        doc_service.reset(new_content.clone());
                        
                        // 3. Persist to Disk & Git (Source of Truth)
                        if let Some(rec_id) = &current_recording_id {
                             if let Some(recordings_dir) = get_current_workspace_recordings_dir(&app_handle).await {
                                let recording_path = recordings_dir.join(rec_id);
                                
                                // Save to file
                                let doc_path = recording_path.join(format!("{}.md", rec_id));
                                if let Err(e) = std::fs::write(&doc_path, &new_content) {
                                    let error_msg = format!("Failed to save manual edit: {:?}", e);
                                    error!("{}", error_msg);
                                    emit_error_toast(&app_handle, &error_msg);
                                } else {
                                    // Git Commit associated with manual edit
                                    // 1. Calculate diff
                                    let diff = git_manager.get_diff(&recording_path).unwrap_or_default();
                                    
                                    // 2. Generate commit message using LLM
                                    let commit_msg = git_manager.generate_commit_message(&*llm_flash, &diff).await.unwrap_or_else(|_| "Manual edit by user".to_string());
                                    
                                    // 3. Commit
                                    if let Err(e) = git_manager.commit_existing(&recording_path, &commit_msg) {
                                        error!("Failed to commit manual edit: {:?}", e);
                                    } else {
                                         // Refresh state to include new commit
                                        let _ = state_manager.refresh_git_history(&recording_path, &git_manager);
                                    }
                                }
                            }
                        }
                    }
                    PipelineCommand::IngestDocument { filename, content } => {
                        if let Some(rec_id) = &current_recording_id {
                            info!("Ingesting Document: {}", filename);
                            let rag_clone = rag_service.clone();
                            let rec_id_clone = rec_id.clone();
                            let app_clone = app_handle.clone();
                            let filename_clone = filename.clone();
                            tokio::spawn(async move {
                                if let Err(e) = rag_clone.ingest_document(&rec_id_clone, &filename_clone, &content).await {
                                    let error_msg = format!("Document ingestion failed ({}): {:?}", filename_clone, e);
                                    error!("Ingestion failed: {}", error_msg);
                                    emit_error_toast(&app_clone, &error_msg);
                                } else {
                                    emit_success_toast(&app_clone, format!("Document '{}' ingested successfully", filename_clone));
                                }
                            });
                        } else {
                            let error_msg = "Cannot ingest document: No active recording session";
                            warn!("{}", error_msg);
                            emit_warning_toast(&app_handle, error_msg);
                        }
                    }
                    PipelineCommand::RollbackToCommit(commit_hash) => {
                        if let Some(rec_id) = &current_recording_id {
                             if let Some(recordings_dir) = get_current_workspace_recordings_dir(&app_handle).await {
                                info!("Rolling back to commit: {}", commit_hash);
                                let recording_path = recordings_dir.join(rec_id);
                                
                                match git_manager.rollback(&recording_path, &commit_hash) {
                                    Ok(restored_content) => {
                                        doc_service.reset(restored_content.clone());
                                        emit_update(&doc_service, &app_handle);
                                        state_manager.update_document(restored_content);
                                        emit_success_toast(&app_handle, "Rollback successful");
                                        info!("Rollback successful");
                                    }
                                    Err(e) => {
                                        let error_msg = format!("Rollback failed: {:?}", e);
                                        error!("{}", error_msg);
                                        emit_error_toast(&app_handle, &error_msg);
                                    }
                                }
                            }
                        } else {
                            warn!("Cannot rollback: No active recording session.");
                        }
                    }
                    PipelineCommand::UndoLastChange => {
                        if let Some(rec_id) = &current_recording_id {
                             if let Some(recordings_dir) = get_current_workspace_recordings_dir(&app_handle).await {
                                info!("Undoing last change...");
                                let recording_path = recordings_dir.join(rec_id);
                                
                                match git_manager.get_history(&recording_path, 2) {
                                    Ok(history) if history.len() >= 2 => {
                                        if let Ok(repo) = git2::Repository::open(&recording_path) {
                                            let mut revwalk = repo.revwalk().unwrap();
                                            revwalk.push_head().unwrap();
                                            
                                            let commits: Vec<_> = revwalk.take(2).collect();
                                            if commits.len() >= 2 {
                                                if let Ok(oid) = commits[1] {
                                                    let commit_hash = format!("{}", oid);
                                                    match git_manager.rollback(&recording_path, &commit_hash) {
                                                        Ok(restored_content) => {
                                                            doc_service.reset(restored_content.clone());
                                                            emit_update(&doc_service, &app_handle);
                                                            state_manager.update_document(restored_content);
                                                            emit_success_toast(&app_handle, "Undo successful");
                                                            info!("Undo successful");
                                                        }
                                                        Err(e) => {
                                                            let error_msg = format!("Undo failed: {:?}", e);
                                                            error!("{}", error_msg);
                                                            emit_error_toast(&app_handle, &error_msg);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    _ => {
                                        warn!("No previous version to undo to.");
                                    }
                                }
                            }
                        } else {
                            warn!("Cannot undo: No active recording session.");
                        }
                    }
                    PipelineCommand::LoadRecording { recording_id } => {
                        info!("[Load Recording] {}", recording_id);
                        flush_deadline = None;
                        is_paused = false;

                        // Clear history from previous session
                        chat_history.write().await.clear();
                        
                        current_recording_id = Some(recording_id.clone());
                        
                        // Sync StateManager
                        if let Err(e) = state_manager.set_current_recording(recording_id.clone()) {
                            let error_msg = format!("Failed to set recording in State Manager: {:?}", e);
                            warn!("{}", error_msg);
                            emit_warning_toast(&app_handle, &error_msg);
                        }
                        
                        // Load document content from disk WITH workspace isolation
                        let mut content = String::new();
                        if let Some(recordings_dir) = get_current_workspace_recordings_dir(&app_handle).await {
                            let doc_path = recordings_dir.join(&recording_id).join(format!("{}.md", &recording_id));
                            if doc_path.exists() {
                                if let Ok(c) = std::fs::read_to_string(&doc_path) {
                                    info!("Loaded recording content: {} chars", c.len());
                                    content = c;
                                }
                            }
                        } else {
                             warn!("Could not load recording: No active workspace.");
                        }

                        // Always update state even if empty/error to ensure fresh start
                        state_manager.update_document(content.clone());
                        doc_service.reset(content);
                        emit_update(&doc_service, &app_handle);


                        let _ = app_handle.emit("recording-started", serde_json::json!({ "recording_id": recording_id }));
                        
                        let rag_clone = rag_service.clone();
                        let rec_id_clone = recording_id.clone();
                        tokio::spawn(async move {
                            if let Err(e) = rag_clone.init_recording(&rec_id_clone).await {
                                error!("Failed to init RAG for loaded recording: {:?}", e);
                            }
                        });
                    }
                    PipelineCommand::DeleteRecording { recording_id } => {
                        info!("[Delete Recording] {}", recording_id);
                        
                        let rag_clone = rag_service.clone();
                        let rec_id_clone = recording_id.clone();
                        tokio::spawn(async move {
                            if let Err(e) = rag_clone.delete_recording(&rec_id_clone).await {
                                error!("Failed to delete RAG data for {}: {:?}", rec_id_clone, e);
                            }
                        });
                        
                        // If we deleted the current recording, reset state
                        if let Some(curr) = &current_recording_id {
                            if curr == &recording_id {
                                current_recording_id = None;
                                doc_service.reset(String::new());
                                state_manager.update_document(String::new());
                            }
                        }
                    }
                    PipelineCommand::AddTodo(desc) => {
                        info!("[Add Todo] {}", desc);
                        if let Some(_) = &current_recording_id {
                             let todo_id = uuid::Uuid::new_v4().to_string();
                             // Correct method: add_todo takes (id, desc)
                             state_manager.add_todo(todo_id.clone(), desc.clone());
                             emit_success_toast(&app_handle, "Todo added");
                             
                             if let Err(e) = state_manager.persist_state() {
                                 error!("Failed to persist state after adding todo: {:?}", e);
                             }
                        } else {
                             warn!("No active recording to add todo to");
                             emit_warning_toast(&app_handle, "No active recording to add todo to");
                        }
                    }
                    PipelineCommand::UpdateTodo { id, description } => {
                        info!("[Update Todo] {} -> {}", id, description);
                         // Correct method: update_todo (not update_todo_description)
                         if let Err(e) = state_manager.update_todo(&id, description) {
                             let error_msg = format!("Failed to update todo: {:?}", e);
                             emit_error_toast(&app_handle, &error_msg);
                         } else {
                             let _ = state_manager.persist_state();
                             emit_success_toast(&app_handle, "Todo updated");
                         }
                    }
                    PipelineCommand::ToggleTodo(id) => {
                        info!("[Toggle Todo] {}", id);
                        // Correct method: toggle_todo (not toggle_todo_status)
                        if let Err(e) = state_manager.toggle_todo(&id) {
                            let error_msg = format!("Failed to toggle todo: {:?}", e);
                            emit_error_toast(&app_handle, &error_msg);
                        } else {
                            let _ = state_manager.persist_state();
                        }
                    }
                    PipelineCommand::DeleteTodo(id) => {
                        info!("[Delete Todo] {}", id);
                        if let Err(e) = state_manager.delete_todo(&id) {
                            let error_msg = format!("Failed to delete todo: {:?}", e);
                            emit_error_toast(&app_handle, &error_msg);
                        } else {
                            let _ = state_manager.persist_state();
                            emit_success_toast(&app_handle, "Todo deleted");
                        }
                    }
                }
            }

            // Handle Transcripts from ASR
            Some(transcript) = asr_rx.recv() => {
                if let Some(token) = &asr_cancellation_token {
                    if !token.is_cancelled() && !is_paused {
                        // Start/Reset Holdback deadline
                        pending_transcript.push_str(&transcript);
                        holdback_deadline = Some(Instant::now() + Duration::from_millis(HOLDBACK_MS));
                        info!("[ASR Chaining] Added to pending batch. Waiting {}ms...", HOLDBACK_MS);
                    }
                }
            }

            _ = timeout_fut => {
                let now = Instant::now();
                
                // 1. Check Holdback (ASR Batching)
                if let Some(deadline) = holdback_deadline {
                    if now >= deadline {
                        info!("[ASR Holdback] Deadline met. Firing processing for {} chars...", pending_transcript.len());
                        let text = std::mem::take(&mut pending_transcript);
                        holdback_deadline = None;
                        
                        // Combine any internal buffered text from aggregator with the new text
                        // to ensure the entire turn is processed as a WHOLE unit.
                        let mut turn_text = speech_agg.flush();
                        if !text.is_empty() {
                            if !turn_text.is_empty() { turn_text.push(' '); }
                            turn_text.push_str(&text);
                        }

                        if turn_text.is_empty() {
                            continue;
                        }

                        // Resolve recording path
                        let recording_path = if let Some(rec_id) = &current_recording_id {
                            get_current_workspace_recordings_dir(&app_handle).await.map(|d| d.join(rec_id))
                        } else {
                            None
                        };

                        // Create new cancellation token for this processing
                        let cancel_token = CancellationToken::new();
                        processing_cancellation_token = Some(cancel_token.clone());
                        
                        let doc_service_clone = doc_service.clone();
                        let llm_coder_clone = llm_coder.clone();
                        let llm_flash_clone = llm_flash.clone();
                        let app_handle_clone = app_handle.clone();
                        let state_manager_clone = state_manager.clone();
                        let git_manager_clone = git_manager.clone();
                        let todo_agent_clone = todo_agent.clone();
                        let rag_service_clone = rag_service.clone();
                        let intent_router_clone = intent_router.clone();
                        let current_recording_id_clone = current_recording_id.clone();
                        let recording_path_clone = recording_path.clone();
                        let chat_history_clone = chat_history.clone();
                        
                        tokio::spawn(async move {
                            tokio::select! {
                                _ = cancel_token.cancelled() => {
                                    info!("[Processing Aborted] Task cancelled");
                                }
                                _ = process_transcript(
                                    turn_text, 
                                    &doc_service_clone, 
                                    &llm_coder_clone,
                                    &llm_flash_clone,
                                    &app_handle_clone, 
                                    &chat_history_clone,
                                    &state_manager_clone,
                                    &git_manager_clone,
                                    &todo_agent_clone,
                                    &rag_service_clone,
                                    &intent_router_clone,
                                    current_recording_id_clone.as_ref(),
                                    recording_path_clone.as_deref(),
                                ) => {
                                    info!("[Processing Complete]");
                                }
                            }
                        });

                        flush_deadline = None;
                    }
                }

                // 2. Check Aggregator Flush (Timeout)
                if let Some(deadline) = flush_deadline {
                    if now >= deadline {
                        let text = speech_agg.flush();
                        flush_deadline = None;
                        
                        // Resolve recording path
                        let recording_path = if let Some(rec_id) = &current_recording_id {
                            get_current_workspace_recordings_dir(&app_handle).await.map(|d| d.join(rec_id))
                        } else {
                            None
                        };

                        // Create new cancellation token for this processing
                        let cancel_token = CancellationToken::new();
                        processing_cancellation_token = Some(cancel_token.clone());
                        
                        let doc_service_clone = doc_service.clone();
                        let llm_coder_clone = llm_coder.clone();
                        let llm_flash_clone = llm_flash.clone();
                        let app_handle_clone = app_handle.clone();
                        let state_manager_clone = state_manager.clone();
                        let git_manager_clone = git_manager.clone();
                        let todo_agent_clone = todo_agent.clone();
                        let rag_service_clone = rag_service.clone();
                        let intent_router_clone = intent_router.clone();
                        let current_recording_id_clone = current_recording_id.clone();
                        let recording_path_clone = recording_path.clone();
                        let chat_history_clone = chat_history.clone();
                        
                        tokio::spawn(async move {
                            tokio::select! {
                                _ = cancel_token.cancelled() => {
                                    info!("[Processing Aborted] Flush task cancelled");
                                }
                                _ = process_transcript(
                                    text, 
                                    &doc_service_clone, 
                                    &llm_coder_clone,
                                    &llm_flash_clone,
                                    &app_handle_clone, 
                                    &chat_history_clone,
                                    &state_manager_clone,
                                    &git_manager_clone,
                                    &todo_agent_clone,
                                    &rag_service_clone,
                                    &intent_router_clone,
                                    current_recording_id_clone.as_ref(),
                                    recording_path_clone.as_deref(),
                                ) => {
                                    info!("[Flush Processing Complete]");
                                }
                            }
                        });
                    }
                }
            }
        }
    }
}
