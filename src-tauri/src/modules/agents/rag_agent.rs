use async_trait::async_trait;
use log::{info, warn, error};
use tokio::time::Duration;

use crate::modules::pipeline::utils::emit_warning_toast;
use crate::modules::QueryAgent;
use super::{Agent, AgentContext};

pub struct RagAgent;

#[async_trait]
impl Agent for RagAgent {
    fn name(&self) -> &str {
        "RagAgent"
    }

    async fn execute(&self, ctx: &mut AgentContext) -> anyhow::Result<()> {
        let result = Self::gather(
            ctx.need_rag,
            ctx.recording_id.as_deref(),
            &ctx.transcript,
            &ctx.doc_service.get_snapshot().content,
            &ctx.llm_flash,
            &ctx.rag_service,
            &ctx.app_handle
        ).await?;
        
        ctx.retrieved_context = result;
        Ok(())
    }
}

impl RagAgent {
    pub async fn gather(
        need_rag: bool,
        recording_id: Option<&str>,
        transcript: &str,
        doc_content: &str,
        llm_flash: &crate::services::llm_client::OpenAILikeClient,
        rag_service: &crate::modules::RagService,
        app_handle: &tauri::AppHandle,
    ) -> anyhow::Result<String> {
        if !need_rag {
            info!("[RagAgent] Skipped (Router 2: context sufficient)");
            return Ok(String::new());
        }

        let rec_id = match recording_id {
            Some(id) => id,
            None => {
                warn!("[RagAgent] Cannot retrieve: No active recording session");
                return Ok(String::new());
            }
        };

        info!("[RagAgent] Context missing detected, retrieving...");
        let query_agent = QueryAgent::new();

        // 1. Generate Query
        let query_result = tokio::time::timeout(
            Duration::from_millis(3000), 
            query_agent.generate_query(llm_flash, transcript, doc_content)
        ).await;

        let query = match query_result {
            Ok(Ok(q)) => q,
            Ok(Err(e)) => {
                let error_msg = format!("RAG query generation failed: {:?}", e);
                error!("[RagAgent] {}", error_msg);
                emit_warning_toast(app_handle, &error_msg);
                return Ok(String::new()); // Fail gracefully
            }
            Err(_) => {
                warn!("[RagAgent] Query generation timeout");
                emit_warning_toast(app_handle, "RAG query generation timeout");
                return Ok(String::new());
            }
        };

        info!("[RagAgent Query] {}", query);

        // 2. Retrieve Documents
        let retrieve_result = tokio::time::timeout(
            Duration::from_millis(1000), 
            rag_service.retrieve_unified(rec_id, &query, 5)
        ).await;

        match retrieve_result {
            Ok(Ok(results)) => {
                if !results.is_empty() {
                    info!("[RagAgent Retrieved] {} items", results.len());
                    let mut retrieved_text = String::new();
                    for (i, item) in results.iter().enumerate() {
                        let source_display = if item.source == "conversation" {
                            "History".to_string()
                        } else {
                            format!("Doc: {}", item.source)
                        };
                        retrieved_text.push_str(&format!("{}. [{}] {}\n", i + 1, source_display, item.content));
                    }
                    return Ok(retrieved_text);
                } else {
                    warn!("[RagAgent] No relevant context found (similarity < 0.7)");
                }
            },
            Ok(Err(e)) => {
                let error_msg = format!("RAG retrieval failed: {:?}", e);
                error!("[RagAgent] {}", error_msg);
                emit_warning_toast(app_handle, &error_msg);
            },
            Err(_) => {
                warn!("[RagAgent] Retrieve timeout");
                emit_warning_toast(app_handle, "RAG retrieval timeout");
            },
        }

        Ok(String::new())
    }
}
