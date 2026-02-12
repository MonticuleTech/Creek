use crate::modules::TodoItem;
use serde::{Deserialize, Serialize};

/// Update from ASR (simulated or real)
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TranscriptUpdate {
    pub text: String,
    pub is_final: bool,
}

/// Document state update
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DocumentUpdate {
    pub content: String,
    pub version: u64,
}

/// Todo list update
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TodoUpdate {
    pub todos: Vec<TodoItem>,
}

/// Recording started event
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct RecordingStarted {
    pub recording_id: String,
}

/// Payload for toast notifications
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ToastPayload {
    pub message: String,
    #[serde(rename = "type")]
    pub toast_type: String, // "info" | "success" | "warning" | "error"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u32>, // milliseconds
}

impl ToastPayload {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            toast_type: "error".to_string(),
            duration: Some(5000), // 5 seconds for errors
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            toast_type: "warning".to_string(),
            duration: Some(4000),
        }
    }

    pub fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            toast_type: "success".to_string(),
            duration: Some(3000),
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            toast_type: "info".to_string(),
            duration: Some(3000),
        }
    }
}

/// Web search results for frontend display
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SearchResultsPayload {
    pub query: String,
    pub content: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AgentStatusPayload {
    pub status: String, // "thinking", "idle"
}
