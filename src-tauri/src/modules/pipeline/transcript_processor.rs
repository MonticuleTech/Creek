use std::sync::Arc;
use std::path::Path;
use tauri::{AppHandle, Emitter};
use log::{info, warn, error};

use crate::modules::document_service::DocumentService;
use crate::modules::{StateManager, GitManager, TodoAgent, RagService, ConversationTurn, IntentRouter, DocIntent};
use crate::services::llm_client::{OpenAILikeClient, ChatMessage};
// Import New Agent System
use crate::modules::agents::{Agent, AgentContext};
use crate::modules::agents::rag_agent::RagAgent;
use crate::modules::agents::search_agent::SearchAgent;
use crate::modules::agents::editor::append_agent::AppendAgent;
use crate::modules::agents::editor::edit_agent::EditAgent;
use crate::modules::agents::editor::grep_agent::GrepAgent;
use crate::modules::agents::editor::undo_agent::UndoAgent;
use crate::modules::agents::editor::clear_agent::ClearAgent;
use crate::commands::recording_commands::{get_recording_metadata, save_recording_metadata};
use super::auto_naming::generate_recording_name;

use super::utils::{emit_update, emit_warning_toast};
use super::types::MAX_HISTORY;

pub async fn process_transcript(
    transcript: String,
    doc_service: &Arc<DocumentService>,
    llm_coder: &Arc<OpenAILikeClient>,
    llm_flash: &Arc<OpenAILikeClient>,
    app_handle: &AppHandle,
    history: &Arc<tokio::sync::RwLock<Vec<ChatMessage>>>,
    state_manager: &Arc<StateManager>,
    git_manager: &Arc<GitManager>,
    todo_agent: &Arc<TodoAgent>,
    rag_service: &Arc<RagService>,
    intent_router: &Arc<IntentRouter>,
    recording_id: Option<&String>,
    recording_path: Option<&Path>,
) {
    info!("==================================================");
    info!("[ASR Input] {}", transcript);
    let _ = app_handle.emit("transcript-update", &transcript);

    // Notify frontend: Thinking started
    let _ = app_handle.emit("agent-status", crate::models::event::AgentStatusPayload { status: "thinking".to_string() });
    
    // 0. Store turn in RAG (async/fire-and-forget to not block pipeline)
    if let Some(rec_id) = recording_id {
        let rag_clone = rag_service.clone();
        let rec_id_clone = rec_id.clone();
        let transcript_clone = transcript.clone();
        let app_clone = app_handle.clone();
        tokio::spawn(async move {
            let turn = ConversationTurn::new(transcript_clone);
            if let Err(e) = rag_clone.store_turn(&rec_id_clone, &turn).await {
                let error_msg = format!("Failed to store conversation in RAG: {:?}", e);
                warn!("{}", error_msg);
                emit_warning_toast(&app_clone, &error_msg);
            }
        });
    }

    let snapshot = doc_service.get_snapshot();
    let mut full_doc = snapshot.content;
    
    // Auto-Normalize: Convert Tabs to 4 spaces
    if full_doc.contains("\t") {
        info!("[Auto-Normalizing tabs to spaces]");
        full_doc = full_doc.replace("\t", "    ");
        doc_service.reset(full_doc.clone());
        emit_update(&doc_service, app_handle);
    }
    
    // ===================================================================
    // 1. Intent Router: Parallel execution of 3 routers
    // ===================================================================
    info!("[Intent Routing...]");
    
    let state = state_manager.get_state();
    let todos_str = state.todo_list.iter()
        .map(|t| format!("- {}", t.desc))
        .collect::<Vec<_>>()
        .join("\n");
    
    // Router 1: Document intent planning
    let router1 = intent_router.clone();
    let doc_clone = full_doc.clone();
    let transcript_clone = transcript.clone();
    let router1_task = tokio::spawn(async move {
        router1.plan_doc_intents(&doc_clone, &transcript_clone).await
    });
    
    // Router 2: RAG need
    let router2 = intent_router.clone();
    let doc_clone = full_doc.clone();
    let focus_clone = state.focus.clone();
    let git_history_clone = state.git_history.clone();
    let todos_clone = todos_str.clone();
    let transcript_clone = transcript.clone();
    let router2_task = tokio::spawn(async move {
        router2.check_rag_need(&doc_clone, &focus_clone, &git_history_clone, &todos_clone, &transcript_clone).await
    });
    
    // Router 3: Tool intent
    let router3 = intent_router.clone();
    let doc_clone = full_doc.clone();
    let transcript_clone = transcript.clone();
    let router3_task = tokio::spawn(async move {
        router3.classify_tool_intent(&doc_clone, &transcript_clone).await
    });
    
    // Wait for all routers to complete
    let (router1_result, router2_result, router3_result) = tokio::join!(
        router1_task,
        router2_task,
        router3_task
    );
    
    let mut plan = match router1_result {
        Ok(Ok(plan)) => {
            info!("[Router 1: Planned Plan] {:?}", plan);
            plan
        },
        Ok(Err(e)) => {
            error!("[Router 1 Failed] {}", e);
            vec![crate::modules::intent_router::PlanStep { 
                intent: "APPEND".to_string(), 
                instruction: "Process transcript".to_string() 
            }]
        },
        Err(e) => {
            error!("[Router 1 Task Failed] {:?}", e);
            vec![crate::modules::intent_router::PlanStep { 
                intent: "APPEND".to_string(), 
                instruction: "Process transcript".to_string() 
            }]
        }
    };
    
    // Force APPEND for empty documents
    // "As long as the document is empty, the next or the first action will always be append. This is irrespective of intent router."
    if full_doc.trim().is_empty() {
        info!("[Empty Document] Forcing APPEND intent (User Rule)");
        plan = vec![crate::modules::intent_router::PlanStep { 
            intent: "APPEND".to_string(), 
            instruction: "Initial document content".to_string() 
        }];
    }
    
    let need_rag = match router2_result {
        Ok(Ok(need)) => {
            info!("[Router 2: RAG Need] {}", need);
            need
        },
        _ => false // Default safe fallback
    };
    
    let tool_intent = match router3_result {
        Ok(Ok(intent)) => {
             info!("[Router 3: Tool Intent] {:?}", intent);
             intent
        },
        _ => crate::modules::intent_router::ToolIntent::None
    };

    // ===================================================================
    // 2. Parallel Information Gathering (Stage 1)
    // ===================================================================
    
    let rag_task = {
        let rec_id = recording_id.cloned();
        let transcript = transcript.clone();
        let doc_content = full_doc.clone();
        let llm_flash = llm_flash.clone();
        let rag_service = rag_service.clone();
        let app_handle = app_handle.clone();
        tokio::spawn(async move {
            RagAgent::gather(
                need_rag,
                rec_id.as_deref(),
                &transcript,
                &doc_content,
                &llm_flash,
                &rag_service,
                &app_handle
            ).await
        })
    };

    let search_task = {
        let tool_intent = tool_intent.clone();
        let llm_flash = Arc::clone(&llm_flash);
        tokio::spawn(async move {
            SearchAgent::gather(&tool_intent, &llm_flash).await
        })
    };

    let (rag_result, search_result) = tokio::join!(rag_task, search_task);
    
    let retrieved_context = rag_result.unwrap_or_else(|_| Ok(String::new())).unwrap_or_default();
    let search_results = search_result.unwrap_or_else(|_| Ok(String::new())).unwrap_or_default();

    if !search_results.is_empty() {
        let query = match &tool_intent {
            crate::modules::intent_router::ToolIntent::Search(q) => q.clone(),
            _ => String::new(),
        };
        let _ = app_handle.emit(
            "search-results",
            crate::models::event::SearchResultsPayload {
                query,
                content: search_results.clone(),
            },
        );
    }

    // ===================================================================
    // 3. Initialize Agent Context (Blackboard)
    // ===================================================================
    let initial_intent = plan.first()
        .and_then(|p| p.to_doc_intent())
        .unwrap_or(DocIntent::NoOp);

    let mut ctx = AgentContext::new(
        transcript.clone(),
        doc_service.clone(),
        llm_coder.clone(),
        llm_flash.clone(),
        app_handle.clone(),
        state_manager.clone(),
        git_manager.clone(),
        todo_agent.clone(),
        rag_service.clone(),
        recording_id.cloned(),
        recording_path.map(|p| p.to_path_buf()),
        history.read().await.clone(),
        initial_intent,
        need_rag,
        tool_intent,
    );
    
    ctx.plan = plan.clone();
    ctx.retrieved_context = retrieved_context;
    ctx.search_results = search_results;
    
    // ...

    for (step, plan_step) in plan.iter().enumerate() {
        info!("[Step {}/{}] Executing Intent: {} | Instruction: {}", 
              step + 1, plan.len(), plan_step.intent, plan_step.instruction);
        
        ctx.current_step = step;
        let doc_intent = plan_step.to_doc_intent().unwrap_or(DocIntent::NoOp);
        ctx.doc_intent = doc_intent.clone();
        
        let execution_result = match doc_intent {
             DocIntent::Append => AppendAgent.execute(&mut ctx).await,
             // ...
            DocIntent::Edit => EditAgent.execute(&mut ctx).await,
            DocIntent::Grep => GrepAgent.execute(&mut ctx).await,
            DocIntent::Undo => UndoAgent.execute(&mut ctx).await,
            DocIntent::Clear => ClearAgent.execute(&mut ctx).await,
            DocIntent::NoOp => {
                 info!("[Action: NO-OP] Skipping document update");
                 // Only add history/emit idle if distinct from previous steps or is single step
                 if plan.len() == 1 {
                     ctx.chat_history.push(ChatMessage { role: "user".to_string(), content: ctx.transcript.clone() });
                     ctx.chat_history.push(ChatMessage { role: "assistant".to_string(), content: "ACTION: NOOP".to_string() });
                     let _ = app_handle.emit("agent-status", crate::models::event::AgentStatusPayload { status: "idle".to_string() });
                 }
                 Ok(())
            },
        };

        if let Err(e) = execution_result {
            error!("[Execution Error at Step {}] {}", step, e);
            emit_warning_toast(app_handle, &format!("Agent Error: {}", e));
            // Decide if we should continue subsequent steps? 
            // For safety, maybe break? Or continue if independent?
            // "Edit then Append" -> if Edit fails, Append might be confused. Break.
            break; 
        }
    }

    // ===================================================================
    // 4. Finalize
    // ===================================================================
    
    // Sync back Chat History
    {
        let mut history_write = history.write().await;
        *history_write = ctx.chat_history;
        
        // Ensure history size limit
        if history_write.len() > MAX_HISTORY {
            let remove_count = history_write.len() - MAX_HISTORY;
            history_write.drain(0..remove_count);
        }
    }

    // ===================================================================
    // 5. Auto-Naming (Optional)
    // ===================================================================
    if let (Some(rec_id), Some(rec_path)) = (recording_id, recording_path) {
        let content = doc_service.get_snapshot().content;
        if content.len() > 150 {
            if let Some(meta) = get_recording_metadata(rec_path) {
                let is_default = meta.name == "New Recording" 
                    || meta.name.starts_with("New Recording (") 
                    || meta.name == *rec_id;
                
                if is_default {
                    info!("[Auto-Naming] Triggering for: {}. Content length: {}", rec_id, content.len());
                    let rec_id_clone = rec_id.clone();
                    let content_clone = content.clone();
                    let llm_clone = llm_flash.clone();
                    let rec_path_clone = rec_path.to_path_buf();
                    let app_clone = app_handle.clone();
                    
                    tokio::spawn(async move {
                        match generate_recording_name(&content_clone, &llm_clone).await {
                            Ok(new_name) => {
                                if let Err(e) = save_recording_metadata(&rec_path_clone, &new_name) {
                                    error!("[Auto-Naming Failed] Failed to save metadata: {:?}", e);
                                } else {
                                    info!("[Auto-Naming Success] Renamed to: {}", new_name);
                                    // Notify frontend with specific renaming event for immediate refresh
                                    let _ = app_clone.emit("recording-renamed", serde_json::json!({
                                        "id": rec_id_clone,
                                        "new_name": new_name
                                    }));
                                    let _ = app_clone.emit("recordings-updated", ());
                                }
                            }
                            Err(e) => {
                                error!("[Auto-Naming Failed] {:?}", e);
                            }
                        }
                    });
                }
            } else {
                 warn!("[Auto-Naming Skip] No metadata found for: {:?}", rec_path);
            }
        }
    }

    info!("==================================================");
}
