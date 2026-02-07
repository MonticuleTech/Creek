// Current implementation modules
pub mod document_service;
pub mod pipeline;

// New DESIGN modules (placeholders for future implementation)
pub mod intent_router;
pub mod rag;
pub mod state_manager;
pub mod git_manager;
pub mod tool_extension;
pub mod todo_agent;
pub mod workspace_manager;
pub mod agents;

    // Re-exports
    pub use state_manager::{StateManager, DocumentState, TodoItem};
    pub use git_manager::GitManager;
    pub use todo_agent::{TodoAgent, TodoOperation};
    pub use rag::{RagService, QueryAgent, ConversationTurn, SearchResult};
    pub use intent_router::{IntentRouter, DocIntent, ToolIntent};
    pub use workspace_manager::{WorkspaceManager, Workspace, WorkspaceConfig};
