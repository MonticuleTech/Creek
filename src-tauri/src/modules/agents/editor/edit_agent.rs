use async_trait::async_trait;
use log::{info, error, warn};
use futures_util::StreamExt;

use crate::modules::pipeline::utils::{emit_and_save, emit_warning_toast};
use crate::modules::pipeline::state_updater::update_state_and_git;
use crate::modules::pipeline::types::MAX_EDIT_RETRIES;
use crate::prompts::document_editing::{
    build_system_message_with_state,
    build_edit_retry_prompt,
    EDIT_AGENT_PROMPT,
};
use crate::services::llm_client::{ChatMessage, LLMClient};
use super::super::{Agent, AgentContext};

pub struct EditAgent;

#[async_trait]
impl Agent for EditAgent {
    fn name(&self) -> &str {
        "EditAgent"
    }

    async fn execute(&self, ctx: &mut AgentContext) -> anyhow::Result<()> {
        info!("[EditAgent] Executing...");

        // 1. Prepare Context
        let state = ctx.state_manager.get_state();
        let todo_list_pairs: Vec<(String, String)> = state.todo_list.iter()
            .filter(|t| !t.completed)
            .map(|t| (t.id.clone(), t.desc.clone()))
            .collect();
        
        // Initial Full Doc for Prompt
        let initial_doc = ctx.doc_service.get_snapshot().content;

        let mut system_msg = build_system_message_with_state(
            EDIT_AGENT_PROMPT,
            &initial_doc,
            &state.focus,
            &state.git_history,
            &todo_list_pairs,
        );
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

        let mut messages = vec![
            ChatMessage { role: "system".to_string(), content: system_msg },
        ];
        messages.extend(ctx.chat_history.clone());
        
        // Emphasize the specific instruction for this step
        let user_content = format!(
            "User's Full Request: \"{}\"\n\n>>> EXECUTION PLAN:\n{}\n\n>>> YOUR CURRENT ASSIGNMENT (Step {}/{}):\n[EDIT] \"{}\"\n\n(Focus ONLY on this assigned task. Ignore other parts of the request if they don't relate to this step.)", 
            ctx.transcript, 
            plan_context,
            ctx.current_step + 1,
            ctx.plan.len(),
            instruction
        );
        messages.push(ChatMessage { role: "user".to_string(), content: user_content });

        // 2. Call LLM loop (with Retry)
        let mut current_response = String::new();
        let mut success = false;
        
        // First attempt logic
        let mut stream = match ctx.llm_coder.stream_completion(messages.clone()).await {
             Ok(s) => s,
             Err(e) => {
                 error!("[EditAgent] LLM failed: {:?}", e);
                 return Ok(());
             }
        };
        while let Some(Ok(chunk)) = stream.next().await {
            current_response.push_str(&chunk);
            print!("{}", chunk);
        }

        // Retry Loop
        for attempt in 1..=MAX_EDIT_RETRIES {
             let clean_response = self.cleanup_tags(&current_response);

             // >>> CRITICAL: WRITE LOCK CHECK <<<
             // Always fetch LATEST snapshot before applying
             // >>> CRITICAL: WRITE LOCK CHECK <<<
             // Always fetch LATEST snapshot before applying
             let _ = ctx.doc_service.get_snapshot();
             
             // Try Apply
             let apply_result = (|| -> Result<bool, String> {
                 let changed = ctx.doc_service.process_stream_chunk(&clean_response)?;
                 // Optional: Validate structure here (e.g. mermaid)
                 Ok(changed)
             })();

             match apply_result {
                 Ok(changed) => {
                     if changed {
                         info!("[EditAgent] Edit applied successfully");
                         emit_and_save(&ctx.doc_service, &ctx.app_handle, ctx.recording_id.as_deref());
                         self.finalize(&clean_response, ctx).await;
                     } else {
                         warn!("[EditAgent] No changes applied (Content identical?)");
                         // Still record conversation
                         self.finalize_no_save(&clean_response, ctx).await;
                     }
                     success = true;
                     break;
                 }
                 Err(e) => {
                     warn!("[EditAgent] Apply failed (Attempt {}): {}", attempt, e);
                     if attempt == MAX_EDIT_RETRIES { break; }
                     
                     // Construct Retry Prompt
                     let retry_prompt = build_edit_retry_prompt(&e);
                     
                     // RE-READ Doc for up-to-date context in retry prompt system message
                     let latest_content = ctx.doc_service.get_snapshot().content;
                     let retry_sys = build_system_message_with_state(
                         EDIT_AGENT_PROMPT,
                         &latest_content, 
                         &state.focus,
                         &state.git_history,
                         &todo_list_pairs
                     );
                     
                     // Rebuild messages
                     let mut retry_messages = messages.clone();
                     retry_messages[0].content = retry_sys; // Update system prompt
                     retry_messages.push(ChatMessage { role: "assistant".to_string(), content: current_response.clone() });
                     retry_messages.push(ChatMessage { role: "user".to_string(), content: retry_prompt });

                     // Call LLM
                     if let Ok(mut s) = ctx.llm_coder.stream_completion(retry_messages).await {
                         let mut new_resp = String::new();
                         while let Some(Ok(chunk)) = s.next().await {
                             new_resp.push_str(&chunk);
                             print!("{}", chunk);
                         }
                         current_response = new_resp;
                     } else {
                         break;
                     }
                 }
             }
        }

        if !success {
            error!("[EditAgent] Failed after retries.");
            emit_warning_toast(&ctx.app_handle, "Failed to apply edits after retries");
        }

        Ok(())
    }
}

impl EditAgent {
    fn enrich_system_message(&self, msg: &mut String, ctx: &AgentContext) {
        msg.push_str("\n### Relevant Context (RAG)\n");
        if ctx.retrieved_context.is_empty() {
            msg.push_str("[None]\n");
        } else {
            msg.push_str(&ctx.retrieved_context);
        }
    }

    fn cleanup_tags(&self, response: &str) -> String {
        response.replace("<<<<< SEARCH", "<<<<<<< SEARCH")
                .replace("<<<< SEARCH", "<<<<<<< SEARCH")
                .replace(">>>>> REPLACE", ">>>>>>> REPLACE")
                .replace(">>>>>> REPLACE", ">>>>>>> REPLACE")
                .replace("\t", "    ")
    }

    async fn finalize(&self, response: &str, ctx: &mut AgentContext) {
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
        self.finalize_no_save(response, ctx).await;
    }

    async fn finalize_no_save(&self, response: &str, ctx: &mut AgentContext) {
        ctx.chat_history.push(ChatMessage { role: "user".to_string(), content: ctx.transcript.clone() });
        ctx.chat_history.push(ChatMessage { role: "assistant".to_string(), content: response.to_string() });
        
        // Ensure history size limit
        let max_history = 6;
        if ctx.chat_history.len() > max_history {
            let remove = ctx.chat_history.len() - max_history;
            ctx.chat_history.drain(0..remove);
        }
    }
}
