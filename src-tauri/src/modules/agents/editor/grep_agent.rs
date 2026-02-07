use async_trait::async_trait;
use log::{info, error, warn};
use futures_util::StreamExt;
use regex::Regex;

use crate::modules::pipeline::utils::{emit_and_save, emit_warning_toast};
use crate::modules::pipeline::state_updater::update_state_and_git;
use crate::prompts::document_editing::{
    build_system_message_with_state,
    GREP_AGENT_PROMPT,
};
use crate::services::llm_client::{ChatMessage, LLMClient};
use super::super::{Agent, AgentContext};

pub struct GrepAgent;

#[async_trait]
impl Agent for GrepAgent {
    fn name(&self) -> &str {
        "GrepAgent"
    }

    async fn execute(&self, ctx: &mut AgentContext) -> anyhow::Result<()> {
        info!("[GrepAgent] Executing...");

        // 1. Prepare Context & Prompt
        let snapshot = ctx.doc_service.get_snapshot();
        let full_doc = snapshot.content;
        let state = ctx.state_manager.get_state();
        
        let system_msg = build_system_message_with_state(
            GREP_AGENT_PROMPT,
            &full_doc,
            &state.focus,
            &state.git_history,
            &[], // No todos needed for grep usually
        );

        let mut messages = vec![ChatMessage { role: "system".to_string(), content: system_msg }];
        
        // Get instruction
        let instruction = ctx.plan.get(ctx.current_step)
            .map(|step| step.instruction.clone())
            .unwrap_or_else(|| "Process the user request".to_string());
            
        // Build Plan Context String
        let plan_context = ctx.plan.iter().enumerate()
            .map(|(i, step)| format!("{}. [{}] {}", i + 1, step.intent, step.instruction))
            .collect::<Vec<_>>()
            .join("\n");

        let user_content = format!(
            "User's Full Request: \"{}\"\n\n>>> EXECUTION PLAN:\n{}\n\n>>> YOUR CURRENT ASSIGNMENT (Step {}/{}):\n[GREP] \"{}\"\n\n(Extract proper FIND/REPLACE values for THIS task. Ignore other parts of the request if they don't relate to this step.)", 
            ctx.transcript, 
            plan_context,
            ctx.current_step + 1,
            ctx.plan.len(),
            instruction
        );
        messages.push(ChatMessage { role: "user".to_string(), content: user_content });

        // 2. Call LLM
        let mut stream = match ctx.llm_coder.stream_completion(messages).await {
            Ok(s) => s,
            Err(e) => {
                error!("[GrepAgent] LLM failed: {:?}", e);
                return Ok(());
            }
        };

        let mut response = String::new();
        while let Some(Ok(chunk)) = stream.next().await {
            response.push_str(&chunk);
            print!("{}", chunk);
        }

        // 3. Parse and Apply
        // WRITE LOCK: Re-fetch latest doc
        let latest_doc = ctx.doc_service.get_snapshot().content;

        let mut find = String::new();
        let mut replace = String::new();
        for line in response.lines() {
            if line.starts_with("FIND:") {
                find = line.trim_start_matches("FIND:").trim().to_string();
            } else if line.starts_with("REPLACE:") {
                replace = line.trim_start_matches("REPLACE:").trim().to_string();
            }
        }

        if !find.is_empty() {
             let new_doc = match Regex::new(&find) {
                Ok(re) => re.replace_all(&latest_doc, replace.as_str()).to_string(),
                Err(_) => latest_doc.replace(&find, &replace),
            };

            if new_doc != latest_doc {
                ctx.doc_service.reset(new_doc);
                emit_and_save(&ctx.doc_service, &ctx.app_handle, ctx.recording_id.as_deref());
                
                // Update State
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

                // Update History
                ctx.chat_history.push(ChatMessage { role: "user".to_string(), content: ctx.transcript.clone() });
                ctx.chat_history.push(ChatMessage { role: "assistant".to_string(), content: response });
                 let max_history = 6;
                if ctx.chat_history.len() > max_history {
                    let remove = ctx.chat_history.len() - max_history;
                    ctx.chat_history.drain(0..remove);
                }
            } else {
                 emit_warning_toast(&ctx.app_handle, &format!("Grep: Pattern not found '{}'", find));
            }
        } else {
            warn!("[GrepAgent] No FIND pattern found in response");
        }

        Ok(())
    }
}
