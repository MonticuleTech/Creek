use async_trait::async_trait;
use log::info;
use tauri::Emitter;

use crate::modules::pipeline::utils::emit_update;
use crate::modules::pipeline::state_updater::update_state_and_git;
use crate::services::llm_client::ChatMessage;
use super::super::{Agent, AgentContext};

pub struct ClearAgent;

#[async_trait]
impl Agent for ClearAgent {
    fn name(&self) -> &str {
        "ClearAgent"
    }

    async fn execute(&self, ctx: &mut AgentContext) -> anyhow::Result<()> {
        info!("[ClearAgent] Clearing document...");
        
        ctx.doc_service.reset(String::new());
        emit_update(&ctx.doc_service, &ctx.app_handle);
        
        let current_todos = ctx.state_manager.get_todos();
        if !current_todos.is_empty() {
             for todo in current_todos {
                let _ = ctx.state_manager.delete_todo(&todo.id);
            }
            let _ = ctx.app_handle.emit("todo-update", crate::models::event::TodoUpdate { todos: vec![] });
        }

        update_state_and_git(
            &ctx.doc_service, 
            &ctx.state_manager, 
            &ctx.git_manager, 
            &ctx.todo_agent, 
            &ctx.llm_flash, 
            &ctx.app_handle, 
            ctx.recording_path.as_deref(), 
            &ctx.transcript
        ).await;

        ctx.chat_history.push(ChatMessage { role: "user".to_string(), content: ctx.transcript.clone() });
        ctx.chat_history.push(ChatMessage { role: "assistant".to_string(), content: "ACTION: CLEAR".to_string() });

        Ok(())
    }
}
