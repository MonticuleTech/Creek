// Todo Agent Module
//
// Automatically generates and maintains todo list based on document content and user input

use anyhow::Result;
use serde::{Deserialize, Serialize};
use crate::services::llm_client::{LLMClient, ChatMessage};
use crate::modules::TodoItem;
use crate::prompts::todo_agent::build_todo_maintenance_prompt;
use futures_util::StreamExt;
use log::{info, warn, error};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "lowercase")]
pub enum TodoOperation {
    #[serde(rename = "complete")]
    Complete { todo_id: String },
    #[serde(rename = "update")]
    Update { todo_id: String, new_desc: String },
    #[serde(rename = "delete")]
    Delete { todo_id: String },
    #[serde(rename = "add")]
    Add { desc: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoOperations {
    pub operations: Vec<TodoOperation>,
}

pub struct TodoAgent;

impl TodoAgent {
    pub fn new() -> Self {
        Self
    }
    
    /// Maintain todos using LLM
    pub async fn maintain_todos<T: LLMClient>(
        &self,
        llm: &T,
        current_doc: &str,
        current_todos: &[TodoItem],
        user_input: &str,
        recent_changes: &str,
    ) -> Result<Vec<TodoOperation>> {
        // Skip if document is empty and no todos exist
        if current_doc.trim().is_empty() && current_todos.is_empty() {
            return Ok(vec![]);
        }
        
        info!("[Todo Agent] Analyzing todos (current: {})...", current_todos.len());
        
        let prompt = build_todo_maintenance_prompt(
            current_doc,
            current_todos,
            user_input,
            recent_changes,
        );
        
        let messages = vec![
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
            }
        ];
        
        // Call LLM with timeout
        let response = match tokio::time::timeout(
            std::time::Duration::from_secs(15),
            self.collect_llm_response(llm, messages)
        ).await {
            Ok(Ok(resp)) => {
                info!("[Todo Agent] LLM response received");
                resp
            },
            Err(_) => {
                warn!("[Todo Agent] LLM timeout");
                return Ok(vec![]);
            },
            Ok(Err(e)) => {
                error!("[Todo Agent] LLM error: {:?}", e);
                return Ok(vec![]);
            }
        };
        
        // Parse JSON response
        let response = response.trim();
        
        // Try to extract JSON if wrapped in markdown
        let json_str = if response.starts_with("```json") && response.ends_with("```") {
            response.strip_prefix("```json")
                .and_then(|s| s.strip_suffix("```"))
                .unwrap_or(response)
                .trim()
        } else if response.starts_with("```") && response.ends_with("```") {
            response.strip_prefix("```")
                .and_then(|s| s.strip_suffix("```"))
                .unwrap_or(response)
                .trim()
        } else {
            response
        };
        
        match serde_json::from_str::<TodoOperations>(json_str) {
            Ok(ops) => {
                if !ops.operations.is_empty() {
                    info!("[Todo Agent] Parsed {} operations", ops.operations.len());
                }
                Ok(ops.operations)
            },
            Err(e) => {
                error!("[Todo Agent] Failed to parse JSON: {:?}", e);
                error!("Response was: {}", json_str);
                Ok(vec![])
            }
        }
    }
    
    /// Helper to collect full LLM response from stream
    async fn collect_llm_response<T: LLMClient>(
        &self,
        llm: &T,
        messages: Vec<ChatMessage>,
    ) -> Result<String> {
        let mut stream = llm.stream_completion(messages).await
            .map_err(|e| anyhow::anyhow!("LLM error: {:?}", e))?;
        
        let mut response = String::new();
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => response.push_str(&chunk),
                Err(e) => return Err(anyhow::anyhow!("Stream error: {:?}", e)),
            }
        }
        
        Ok(response)
    }
}

impl Default for TodoAgent {
    fn default() -> Self {
        Self::new()
    }
}
