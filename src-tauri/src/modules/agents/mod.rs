use async_trait::async_trait;
use std::sync::Arc;
use std::path::PathBuf;
use tauri::AppHandle;

use crate::modules::document_service::DocumentService;
use crate::modules::{StateManager, GitManager, TodoAgent, RagService, DocIntent, ToolIntent};
use crate::modules::intent_router::PlanStep;
use crate::services::llm_client::{OpenAILikeClient, ChatMessage};

pub mod rag_agent;
pub mod search_agent;
pub mod editor;

/// Context passed between agents in the pipeline
pub struct AgentContext {
    // --- Inputs / Services ---
    pub transcript: String,
    pub doc_service: Arc<DocumentService>,
    pub llm_coder: Arc<OpenAILikeClient>,
    pub llm_flash: Arc<OpenAILikeClient>,
    pub app_handle: AppHandle,
    pub state_manager: Arc<StateManager>,
    pub git_manager: Arc<GitManager>,
    pub todo_agent: Arc<TodoAgent>,
    pub rag_service: Arc<RagService>,
    pub recording_id: Option<String>,
    pub recording_path: Option<PathBuf>,

    // --- Mutable State (The "Blackboard") ---
    pub chat_history: Vec<ChatMessage>,
    
    /// Decision from Router 1
    /// Decision from Router 1
    /// Decision from Router 1
    pub doc_intent: DocIntent,

// ...

    /// Planned sequence of intents
    pub plan: Vec<PlanStep>,
    /// Current step in the plan (0-indexed)
    pub current_step: usize,

    /// Decision from Router 2
    pub need_rag: bool,
    /// Decision from Router 3
    pub tool_intent: ToolIntent,
    
    /// Data populated by RagAgent
    pub retrieved_context: String,
    /// Data populated by SearchAgent
    pub search_results: String,
}

impl AgentContext {
    pub fn new(
        transcript: String,
        doc_service: Arc<DocumentService>,
        llm_coder: Arc<OpenAILikeClient>,
        llm_flash: Arc<OpenAILikeClient>,
        app_handle: AppHandle,
        state_manager: Arc<StateManager>,
        git_manager: Arc<GitManager>,
        todo_agent: Arc<TodoAgent>,
        rag_service: Arc<RagService>,
        recording_id: Option<String>,
        recording_path: Option<PathBuf>,
        chat_history: Vec<ChatMessage>,
        doc_intent: DocIntent,
        need_rag: bool,
        tool_intent: ToolIntent,
    ) -> Self {
        Self {
            transcript,
            doc_service,
            llm_coder,
            llm_flash,
            app_handle,
            state_manager,
            git_manager,
            todo_agent,
            rag_service,
            recording_id,
            recording_path,
            chat_history,
            doc_intent,
            plan: Vec::new(),
            current_step: 0,
            need_rag,
            tool_intent,
            retrieved_context: String::new(),
            search_results: String::new(),
        }
    }
}

/// The base trait that all sub-agents must implement
#[async_trait]
pub trait Agent: Send + Sync {
    /// Name of the agent for logging
    fn name(&self) -> &str;
    
    /// Execute the agent's logic, potentially modifying the context
    async fn execute(&self, ctx: &mut AgentContext) -> anyhow::Result<()>;
}
