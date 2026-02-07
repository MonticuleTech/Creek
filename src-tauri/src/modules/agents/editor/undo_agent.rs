use async_trait::async_trait;
use log::{info, warn};
use futures_util::StreamExt;

use crate::modules::pipeline::utils::{emit_update, emit_success_toast, emit_error_toast};
use crate::modules::pipeline::state_updater::update_state_and_git;
use crate::services::llm_client::{ChatMessage, LLMClient};
use super::super::{Agent, AgentContext};

pub struct UndoAgent;

#[async_trait]
impl Agent for UndoAgent {
    fn name(&self) -> &str {
        "UndoAgent"
    }

    async fn execute(&self, ctx: &mut AgentContext) -> anyhow::Result<()> {
        info!("[UndoAgent] Executing...");
        
        let rec_path = match &ctx.recording_path {
            Some(p) => p,
            None => { 
                warn!("Cannot undo: No recording path"); 
                return Ok(()); 
            }
        };

        match ctx.git_manager.get_history_with_hashes(rec_path, 10) {
            Ok(commits) if commits.len() >= 2 => {
                 // Determine target hash via LLM (Flash)
                  let history_text: String = commits.iter().enumerate()
                        .map(|(i, (hash, msg))| format!("{}. [{}] {}", i, hash.chars().take(7).collect::<String>(), msg.trim()))
                        .collect::<Vec<_>>()
                        .join("\n");
                    
                    let instruction = ctx.plan.get(ctx.current_step)
                        .map(|step| step.instruction.clone())
                        .unwrap_or_else(|| ctx.transcript.clone());

                    let prompt = format!(
                        "User wants to undo/rollback.\n\nGit History (Latest first):\n{}\n\nSpecific Instruction: \"{}\"\n(Original Input: \"{}\")\n\nTask: Return the COMMIT HASH that we should reset the document to.\nRules:\n- If user says 'undo', 'back', 'cancel', return the hash of commit #1 (the one before current HEAD).\n- If user identifies a specific version (e.g. 'go back to before X'), return that commit's hash.\n- Output ONLY the hash string.",
                        history_text, instruction, ctx.transcript
                    );
                    
                    
                    let messages = vec![ChatMessage { role: "user".to_string(), content: prompt }];
                    
                    let target_hash = match ctx.llm_flash.stream_completion(messages).await {
                        Ok(mut stream) => {
                            let mut s = String::new();
                            while let Some(Ok(chunk)) = stream.next().await { s.push_str(&chunk); }
                            s.trim().to_string()
                        },
                        Err(_) => commits[1].0.clone() // Fallback
                    };
                     let clean_hash = target_hash.split_whitespace().next().unwrap_or(&commits[1].0).to_string();
                     info!("[Undo Target] {}", clean_hash);

                     if let Ok(restored) = ctx.git_manager.rollback(rec_path, &clean_hash) {
                         ctx.doc_service.reset(restored.clone());
                         emit_update(&ctx.doc_service, &ctx.app_handle);
                         ctx.state_manager.update_document(restored);
                         emit_success_toast(&ctx.app_handle, "Rollback successful");
                         
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
                        ctx.chat_history.push(ChatMessage { role: "assistant".to_string(), content: format!("ACTION: UNDO (to {})", clean_hash) });
                     } else {
                         emit_error_toast(&ctx.app_handle, "Rollback failed");
                     }
            }
            _ => { warn!("Not enough history to undo"); }
        }
        Ok(())
    }
}
