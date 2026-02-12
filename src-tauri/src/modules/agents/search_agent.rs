use async_trait::async_trait;
use log::info;
use super::{Agent, AgentContext};
use crate::modules::intent_router::ToolIntent;
use crate::services::llm_client::{OpenAILikeClient, ChatMessage};

pub struct SearchAgent;

#[async_trait]
impl Agent for SearchAgent {
    fn name(&self) -> &str {
        "SearchAgent"
    }

    async fn execute(&self, ctx: &mut AgentContext) -> anyhow::Result<()> {
        let result = Self::gather(&ctx.tool_intent, &ctx.llm_flash).await?;
        ctx.search_results = result;
        Ok(())
    }
}

impl SearchAgent {
    pub async fn gather(tool_intent: &ToolIntent, llm_flash: &OpenAILikeClient) -> anyhow::Result<String> {
        match tool_intent {
             ToolIntent::Search(query) => {
                 info!("[SearchAgent] Performing web search for: {}", query);
                 let messages = vec![
                     ChatMessage {
                         role: "system".to_string(),
                         content: "You are a web search assistant. Search for the following query and return relevant, factual information. Be concise and cite sources when possible.".to_string(),
                     },
                     ChatMessage {
                         role: "user".to_string(),
                         content: query.clone(),
                     },
                 ];
                 let result = llm_flash.chat_with_search(messages).await
                     .map_err(|e| anyhow::anyhow!("Search LLM call failed: {:?}", e))?;
                 Ok(result)
             },
             ToolIntent::None => Ok(String::new())
        }
    }
}
