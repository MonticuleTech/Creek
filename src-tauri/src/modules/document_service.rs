use std::sync::{Arc, Mutex};
use crate::utils::diff::{DiffParser, apply_patch, PendingPatch};

/// Represents the current state of the document
#[derive(Debug, Clone)]
pub struct DocumentState {
    pub content: String,
    pub version: u64,
}

impl DocumentState {
    pub fn new(content: String) -> Self {
        Self {
            content,
            version: 0,
        }
    }
}

/// Service for managing document state and applying incremental edits
pub struct DocumentService {
    state: Arc<Mutex<DocumentState>>,
    // Keep one parser for incremental streaming use cases, but we will not rely on its state
    // across responses in the current pipeline (we parse whole response per call).
    #[allow(dead_code)]
    parser: Mutex<DiffParser>,
}

impl DocumentService {
    pub fn new(initial_content: String) -> Self {
        Self {
            state: Arc::new(Mutex::new(DocumentState::new(initial_content))),
            parser: Mutex::new(DiffParser::new()),
        }
    }

    /// Get a snapshot of the current document state
    pub fn get_snapshot(&self) -> DocumentState {
        self.state.lock().unwrap().clone()
    }

    /// Reset the document with new content
    pub fn reset(&self, new_content: String) {
        let mut state = self.state.lock().unwrap();
        state.content = new_content;
        state.version += 1;
    }

    /// Directly append content to the document state (for streaming APPEND action)
    pub fn append_content(&self, chunk: &str) {
        let mut state = self.state.lock().unwrap();
        state.content.push_str(chunk);
        state.version += 1;
    }

    /// Ensure the document ends with at least `count` newlines
    pub fn ensure_newlines(&self, count: usize) {
        let mut state = self.state.lock().unwrap();
        if state.content.is_empty() {
            return;
        }

        let existing_newlines = state.content.chars().rev().take_while(|c| *c == '\n').count();
        if existing_newlines < count {
            state.content.push_str(&"\n".repeat(count - existing_newlines));
            state.version += 1;
        }
    }

    /// Process a chunk of text from LLM containing diff patches
    /// Returns Ok(true) if state was updated, Ok(false) if no updates
    pub fn process_stream_chunk(&self, chunk: &str) -> Result<bool, String> {
        // Parse full response into complete patches to avoid parser state leakage between calls
        // and reduce "marker spam" corruption.
        let patches = DiffParser::parse_all(chunk);
        if patches.is_empty() {
            return Ok(false);
        }

        let mut updated = false;
        for patch in patches {
            self.apply_patch_to_state(patch)?;
            updated = true;
        }
        Ok(updated)
    }

    fn apply_patch_to_state(&self, patch: PendingPatch) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        match apply_patch(&state.content, patch) {
            Ok(new_content) => {
                state.content = new_content;
                state.version += 1;
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        }
    }
}
