use std::sync::Arc;
use log::info;
use crate::services::llm_client::{OpenAILikeClient, LLMClient};
use crate::prompts::auto_naming::{AUTO_NAME_SYSTEM_PROMPT, AUTO_NAME_USER_TEMPLATE};

pub async fn generate_recording_name(
    content: &str,
    llm: &Arc<OpenAILikeClient>,
) -> Result<String, String> {
    info!("[Auto-Naming] Generating name based on content...");
    
    // Take first 1000 chars for context
    let snippet = if content.len() > 1000 {
        content.chars().take(1000).collect::<String>()
    } else {
        content.to_string()
    };

    let user_prompt = AUTO_NAME_USER_TEMPLATE.replace("{content}", &snippet);
    
    let messages = vec![
        crate::services::llm_client::ChatMessage {
            role: "system".to_string(),
            content: AUTO_NAME_SYSTEM_PROMPT.to_string(),
        },
        crate::services::llm_client::ChatMessage {
            role: "user".to_string(),
            content: user_prompt,
        },
    ];

    let title = llm.chat(messages, None).await
        .map_err(|e| format!("LLM naming failed: {:?}", e))?;
    
    let title = title.trim()
        .trim_matches('"')
        .trim_matches('#') // Remove markdown header if LLM hallucinated
        .trim()
        .to_string();
    
    if title.is_empty() {
        return Err("Generated title is empty".to_string());
    }

    info!("[Auto-Naming] Generated Title: {}", title);
    Ok(title)
}
