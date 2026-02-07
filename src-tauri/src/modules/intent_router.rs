// Intent Router Module
// 
// DESIGN: Three independent lightweight models (0.6B) running in parallel to determine user intent
// 
// Router 1: Document Layer Routing
//   - Input: ASR text + current full context (full text + all state)
//   - Output: DocIntent enum (NO-OP/APPEND/EDIT/GREP/REPLACE/CLEAR)
//   - Implementation: Qwen 0.6B model for 0-shot classification
//   - Target latency: < 100ms
//
// Router 2: RAG Layer Routing
//   - Output: bool - whether to retrieve historical conversations
//   - Logic: Determine if information mentioned by user is missing in current context
//          (full text + focus + git_history + todo_list)
//
// Router 3: Tool Layer Routing
//   - Output: ToolIntent enum (NONE/SEARCH)
//   - Logic: Whether user intent requires external information (web search)

use crate::services::llm_client::{LLMClient, OpenAILikeClient, ChatMessage};
use crate::prompts::intent_router::build_doc_intent_query;
use futures_util::StreamExt;
use std::sync::Arc;

/// Document intent types that Router 1 can classify
/// Only 5 types: NO-OP / APPEND / EDIT / GREP / CLEAR
#[derive(Debug, Clone, PartialEq)]
pub enum DocIntent {
    NoOp,
    Append,
    Edit,
    Grep,
    Undo,
    Clear,
}

impl DocIntent {
    /// Parse model output string to DocIntent
    pub fn from_str(s: &str) -> Option<Self> {
        let normalized = s.trim().to_uppercase();
        match normalized.as_str() {
            "NO-OP" | "NOOP" | "NO_OP" => Some(DocIntent::NoOp),
            "APPEND" => Some(DocIntent::Append),
            "EDIT" => Some(DocIntent::Edit),
            "GREP" => Some(DocIntent::Grep),
            "NOTH" | "UNDO" => Some(DocIntent::Undo),
            "CLEAR" => Some(DocIntent::Clear),
            _ => None,
        }
    }
}

/// Tool intent types for Router 3
#[derive(Debug, Clone, PartialEq)]
pub enum ToolIntent {
    None,
    Search(String),
}

/// Intent Router - uses lightweight 0.6B model for fast classification
pub struct IntentRouter {
    llm_client: Arc<OpenAILikeClient>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct PlanStep {
    pub intent: String,
    pub instruction: String, // Explicit natural language instruction
}

impl PlanStep {
    pub fn to_doc_intent(&self) -> Option<DocIntent> {
        DocIntent::from_str(&self.intent)
    }
}

impl IntentRouter {
    /// Create a new IntentRouter with qwen-flash model
    pub fn new(api_key: String) -> Self {
        let client = Arc::new(OpenAILikeClient::new(
            "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string(),
            api_key,
            "qwen-flash".to_string(),
        ));
        
        Self {
            llm_client: client,
        }
    }

    /// Router 1: Plan document intents
    /// Returns: Vec<PlanStep>
    pub async fn plan_doc_intents(
        &self,
        current_doc: &str,
        user_input: &str,
    ) -> Result<Vec<PlanStep>, String> {
        // Build classification query
        let query = build_doc_intent_query(current_doc, user_input);
        
        let messages = vec![
            ChatMessage {
                role: "user".to_string(),
                content: query,
            }
        ];

        // Call LLM and collect full response
        let mut stream = self.llm_client.stream_completion(messages).await
            .map_err(|e| format!("LLM request failed: {:?}", e))?;

        let mut response = String::new();
        while let Some(chunk_res) = stream.as_mut().next().await {
            if let Ok(chunk) = chunk_res {
                response.push_str(&chunk);
            }
        }

        let response = response.trim();
        
        // Parsing Logic: Expecting Numbered List
        // Format: "1. [INTENT] Instruction"
        
        let mut steps = Vec::new();
        let re = regex::Regex::new(r"(?m)^\s*(\d+)\.\s*\[?([A-Za-z-_]+)\]?\s*(.*)$").map_err(|e| e.to_string())?;

        for line in response.lines() {
            let line = line.trim();
            if line.is_empty() { continue; }
            
            if let Some(caps) = re.captures(line) {
                let intent = caps.get(2).map_or("", |m| m.as_str()).to_string();
                let instruction = caps.get(3).map_or("", |m| m.as_str()).to_string();
                
                steps.push(PlanStep {
                    intent,
                    instruction
                });
            }
        }
        
        if !steps.is_empty() {
             return Ok(steps);
        }

        // Fallback for simple single-word responses (backward compatibility / fallback)
        if let Some(_intent) = DocIntent::from_str(response) {
            return Ok(vec![PlanStep {
                intent: response.to_string(),
                instruction: "Execute user request based on transcript".to_string()
            }]);
        }
        
        // Fallback: If no structured steps found but there is text, maybe treat entire text as one EDIT?
        // Risky, better to error if totally unrecognized.
        Err(format!("Failed to parse intent plan from: '{}'", response))
    }

    /// Router 2: Check if RAG retrieval is needed
    /// Returns: true if historical context is missing and needs to be retrieved
    pub async fn check_rag_need(
        &self,
        current_doc: &str,
        focus: &str,
        git_history: &[String],
        todo_list: &str,
        user_input: &str,
    ) -> Result<bool, String> {
        use crate::prompts::intent_router::build_rag_need_query;
        
        // Build RAG need detection query
        let query = build_rag_need_query(current_doc, focus, git_history, todo_list, user_input);
        
        let messages = vec![
            ChatMessage {
                role: "user".to_string(),
                content: query,
            }
        ];

        // Call LLM and collect full response
        let mut stream = self.llm_client.stream_completion(messages).await
            .map_err(|e| format!("LLM request failed: {:?}", e))?;

        let mut response = String::new();
        while let Some(chunk_res) = stream.as_mut().next().await {
            if let Ok(chunk) = chunk_res {
                response.push_str(&chunk);
            }
        }

        let response = response.trim().to_lowercase();
        
        // Parse response to bool
        match response.as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(format!("Failed to parse RAG need from response: '{}'", response))
        }
    }

    /// Router 3: Classify tool intent
    /// Returns: ToolIntent enum (NONE or SEARCH)
    pub async fn classify_tool_intent(
        &self,
        current_doc: &str,
        user_input: &str,
    ) -> Result<ToolIntent, String> {
        use crate::prompts::intent_router::build_tool_intent_query;
        
        // Build tool intent query
        let query = build_tool_intent_query(current_doc, user_input);
        
        let messages = vec![
            ChatMessage {
                role: "user".to_string(),
                content: query,
            }
        ];

        // Call LLM and collect full response
        let mut stream = self.llm_client.stream_completion(messages).await
            .map_err(|e| format!("LLM request failed: {:?}", e))?;

        let mut response = String::new();
        while let Some(chunk_res) = stream.as_mut().next().await {
            if let Ok(chunk) = chunk_res {
                response.push_str(&chunk);
            }
        }

        let response = response.trim().to_uppercase();
        
        // Parse response to ToolIntent
        match response.as_str() {
            "NONE" => Ok(ToolIntent::None),
            "SEARCH" => {
                // Extract search query from user input
                Ok(ToolIntent::Search(user_input.to_string()))
            },
            _ => Err(format!("Failed to parse tool intent from response: '{}'", response))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doc_intent_parsing() {
        assert_eq!(DocIntent::from_str("NO-OP"), Some(DocIntent::NoOp));
        assert_eq!(DocIntent::from_str("NOOP"), Some(DocIntent::NoOp));
        assert_eq!(DocIntent::from_str("no-op"), Some(DocIntent::NoOp));
        assert_eq!(DocIntent::from_str("APPEND"), Some(DocIntent::Append));
        assert_eq!(DocIntent::from_str("append"), Some(DocIntent::Append));
        assert_eq!(DocIntent::from_str("EDIT"), Some(DocIntent::Edit));
        assert_eq!(DocIntent::from_str("GREP"), Some(DocIntent::Grep));
        assert_eq!(DocIntent::from_str("CLEAR"), Some(DocIntent::Clear));
        assert_eq!(DocIntent::from_str("UNDO"), Some(DocIntent::Undo));
        assert_eq!(DocIntent::from_str("INVALID"), None);
    }

    #[test]
    fn test_tool_intent() {
        assert_eq!(ToolIntent::None, ToolIntent::None);
        let search = ToolIntent::Search("test query".to_string());
        if let ToolIntent::Search(query) = search {
            assert_eq!(query, "test query");
        } else {
            panic!("Expected Search variant");
        }
    }
}
