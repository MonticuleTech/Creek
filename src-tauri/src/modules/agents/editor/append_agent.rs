use async_trait::async_trait;
use log::{info, error};
use futures_util::StreamExt;
use regex::Regex;

use crate::modules::pipeline::utils::emit_and_save;
use crate::modules::pipeline::state_updater::update_state_and_git;
use crate::prompts::document_editing::{
    build_system_message_with_state,
    APPEND_AGENT_PROMPT,
};
use crate::services::llm_client::{ChatMessage, LLMClient};
use super::super::{Agent, AgentContext};

pub struct AppendAgent;

#[async_trait]
impl Agent for AppendAgent {
    fn name(&self) -> &str {
        "AppendAgent"
    }

    async fn execute(&self, ctx: &mut AgentContext) -> anyhow::Result<()> {
        info!("[AppendAgent] Executing...");

        // 1. Prepare Snapshot & Prompts
        let snapshot = ctx.doc_service.get_snapshot();
        let full_doc = snapshot.content;
        
        // Build ToDo list string
        let state = ctx.state_manager.get_state();
        let todo_list_pairs: Vec<(String, String)> = state.todo_list.iter()
            .filter(|t| !t.completed)
            .map(|t| (t.id.clone(), t.desc.clone()))
            .collect();

        let mut system_msg = build_system_message_with_state(
            APPEND_AGENT_PROMPT,
            &full_doc,
            &state.focus,
            &state.git_history,
            &todo_list_pairs,
        );

        // Add RAG & Search Context
        self.enrich_system_message(&mut system_msg, ctx);

        // Get current instruction from plan
        let instruction = ctx.plan.get(ctx.current_step)
            .map(|step| step.instruction.clone())
            .unwrap_or_else(|| "Process the user request".to_string());

        // Build Plan Context String
        let plan_context = ctx.plan.iter().enumerate()
            .map(|(i, step)| format!("{}. [{}] {}", i + 1, step.intent, step.instruction))
            .collect::<Vec<_>>()
            .join("\n");

        // Build messages
        let mut messages = vec![
            ChatMessage { role: "system".to_string(), content: system_msg },
        ];
        messages.extend(ctx.chat_history.clone());
        
        let user_content = format!(
            "User's Full Request: \"{}\"\n\n>>> EXECUTION PLAN:\n{}\n\n>>> YOUR CURRENT ASSIGNMENT (Step {}/{}):\n[APPEND] \"{}\"\n\n(Focus ONLY on this assigned task. Ignore other parts of the request if they don't relate to this step.)", 
            ctx.transcript, 
            plan_context,
            ctx.current_step + 1,
            ctx.plan.len(),
            instruction
        );
        messages.push(ChatMessage { role: "user".to_string(), content: user_content });

        // 2. Call LLM & Stream
        let mut stream = match ctx.llm_coder.stream_completion(messages.clone()).await {
            Ok(s) => s,
            Err(e) => {
                error!("[AppendAgent] LLM failed: {:?}", e);
                return Ok(());
            }
        };

        info!("[AppendAgent] Streaming response...");
        let mut response_buffer = String::new();
        let mut first_chunk = true;

        while let Some(chunk_res) = stream.as_mut().next().await {
            if let Ok(chunk) = chunk_res {
                let processed = chunk.replace("\t", "    ");
                response_buffer.push_str(&processed);
                print!("{}", processed);

                // Stream directly to document
                if first_chunk {
                    ctx.doc_service.ensure_newlines(2);
                    first_chunk = false;
                }
                ctx.doc_service.append_content(&processed);
                emit_and_save(&ctx.doc_service, &ctx.app_handle, ctx.recording_id.as_deref());
            }
        }
        
        // Final cleanup
        let mut response = response_buffer.replace("\t", "    ");
        if let Ok(re) = Regex::new(r"(?m)^  ") {
            response = re.replace_all(&response, "    ").to_string();
        }
        let response = response.trim().to_string();

        // 3. Update State & History
        if !response.is_empty() {
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
            ctx.chat_history.push(ChatMessage { role: "assistant".to_string(), content: response });
            
             // Ensure history size limit
            let max_history = 6;
            if ctx.chat_history.len() > max_history {
                let remove = ctx.chat_history.len() - max_history;
                ctx.chat_history.drain(0..remove);
            }
        }

        Ok(())
    }
}

impl AppendAgent {
    fn enrich_system_message(&self, msg: &mut String, ctx: &AgentContext) {
        msg.push_str("\n### Relevant Context (RAG)\n");
        if ctx.retrieved_context.is_empty() {
            msg.push_str("[None]\n");
        } else {
            msg.push_str(&ctx.retrieved_context);
        }
        
        if !ctx.search_results.is_empty() {
            msg.push_str("\n### Search Results\n");
            msg.push_str(&ctx.search_results);
            msg.push_str("\n");
        }
    }
}
