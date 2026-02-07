use async_trait::async_trait;
use log::warn;
use super::{Agent, AgentContext};
use crate::modules::intent_router::ToolIntent;

pub struct SearchAgent;

#[async_trait]
impl Agent for SearchAgent {
    fn name(&self) -> &str {
        "SearchAgent"
    }

    async fn execute(&self, ctx: &mut AgentContext) -> anyhow::Result<()> {
        let result = Self::gather(&ctx.tool_intent).await?;
        ctx.search_results = result;
        Ok(())
    }
}

impl SearchAgent {
    pub async fn gather(tool_intent: &ToolIntent) -> anyhow::Result<String> {
        match tool_intent {
             ToolIntent::Search(query) => {
                 // Placeholder for future search implementation
                 warn!("[SearchAgent] Search not yet implemented. Query: {}", query);
                 // We could mock a result here or just leave it empty
                 Ok(format!("(Web Search for '{}' is currently disabled)", query))
             },
             ToolIntent::None => Ok(String::new())
        }
    }
}
