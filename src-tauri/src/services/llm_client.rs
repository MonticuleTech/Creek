use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use futures_util::StreamExt;
use futures_util::future;
use std::pin::Pin;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug)]
pub enum LLMError {
    RequestFailed(String),
    ParseError(String),
}

#[async_trait]
pub trait LLMClient: Send + Sync {
    async fn stream_completion(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<Pin<Box<dyn futures_util::Stream<Item = Result<String, LLMError>> + Send>>, LLMError>;

    async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        _options: Option<serde_json::Value>,
    ) -> Result<String, LLMError> {
        let mut stream = self.stream_completion(messages).await?;
        let mut full_text = String::new();
        while let Some(chunk) = stream.next().await {
            full_text.push_str(&chunk?);
        }
        Ok(full_text)
    }
}

pub struct OpenAILikeClient {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
    enable_thinking: Option<bool>,
}

impl OpenAILikeClient {
    pub fn new(base_url: String, api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url,
            model,
            enable_thinking: None,
        }
    }

    pub fn new_with_thinking(
        base_url: String,
        api_key: String,
        model: String,
        enable_thinking: bool,
    ) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url,
            model,
            enable_thinking: Some(enable_thinking),
        }
    }
}

#[async_trait]
impl LLMClient for OpenAILikeClient {
    async fn stream_completion(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<Pin<Box<dyn futures_util::Stream<Item = Result<String, LLMError>> + Send>>, LLMError> {
        let url = format!("{}/chat/completions", self.base_url);
        let mut body = json!({
            "model": self.model,
            "messages": messages,
            "stream": true,
            "temperature": 1,
        });

        if let Some(enable_thinking) = self.enable_thinking {
            body["extra_body"] = json!({
                "enable_thinking": enable_thinking
            });
        }

        let resp = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| LLMError::RequestFailed(e.to_string()))?;

        if !resp.status().is_success() {
             return Err(LLMError::RequestFailed(format!("HTTP Error: {}", resp.status())));
        }

        let stream = resp.bytes_stream();

        // Robust-ish SSE line buffering:
        // - bytes chunks can split a single SSE "data: {json}" line (very common)
        // - we buffer partial lines across chunks and only parse complete lines
        let stream_mapped = stream.scan(String::new(), |buf, chunk_result| {
            future::ready(match chunk_result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    buf.push_str(&text);

                    let mut delta_content = String::new();

                    // Consume complete lines (ending with '\n'). Keep remainder in `buf`.
                    while let Some(nl_idx) = buf.find('\n') {
                        // Take line (without '\n')
                        let mut line = buf[..nl_idx].to_string();
                        // Drain including '\n'
                        buf.drain(..=nl_idx);

                        // Handle CRLF
                        if line.ends_with('\r') {
                            line.pop();
                        }

                        let line = line.trim_start();
                        // SSE allows "data:" or "data: " prefixes
                        let data = if let Some(rest) = line.strip_prefix("data:") {
                            rest.trim_start()
                        } else {
                            continue;
                        };

                        if data == "[DONE]" {
                            continue;
                        }

                        if let Ok(val) = serde_json::from_str::<Value>(data) {
                            if let Some(content) = val["choices"][0]["delta"]["content"].as_str() {
                                delta_content.push_str(content);
                            }
                        }
                    }

                    Some(Ok(delta_content))
                }
                Err(e) => Some(Err(LLMError::RequestFailed(e.to_string()))),
            })
        });

        Ok(Box::pin(stream_mapped))
    }
}
