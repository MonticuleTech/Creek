use serde::Serialize;

pub const FLUSH_TIMEOUT_MS: u64 = 2000;
pub const MAX_HISTORY: usize = 6; // 3 turns (User+Assistant pairs)
pub const MAX_EDIT_RETRIES: usize = 3;

// ASR chunking (character count)
pub const MIN_SPEECH_CHARS: usize = 40;
pub const MAX_SPEECH_CHARS: usize = 500;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct LlmStreamEvent {
    pub phase: String, // "start" | "chunk" | "end"
    pub content: String,
    pub action_type: String, // "thinking" | "noop" | "append" | "edit" | "grep" | "unknown"
}

pub fn char_len(s: &str) -> usize {
    s.chars().count()
}

pub fn split_long_speech(s: &str) -> Vec<String> {
    let t = s.trim();
    if t.is_empty() {
        return vec![];
    }
    if char_len(t) <= MAX_SPEECH_CHARS {
        return vec![t.to_string()];
    }

    // Split by sentence boundaries first
    let mut out = Vec::new();
    let mut buf = String::new();

    for ch in t.chars() {
        buf.push(ch);
        if matches!(ch, '。' | '！' | '？' | '.' | '!' | '?' | ';' | '；' | '\n') || char_len(&buf) >= MAX_SPEECH_CHARS {
            let chunk = buf.trim().to_string();
            if !chunk.is_empty() {
                out.push(chunk);
            }
            buf.clear();
        }
    }
    let tail = buf.trim().to_string();
    if !tail.is_empty() {
        out.push(tail);
    }
    out
}

#[derive(Default)]
pub struct SpeechAggregator {
    pub buf: String,
}

impl SpeechAggregator {
    pub fn push(&mut self, chunk: &str) -> Vec<String> {
        let chunk = chunk.trim();
        if chunk.is_empty() {
            return vec![];
        }

        // If it's very long, flush buffer first then split the long chunk.
        if char_len(chunk) > MAX_SPEECH_CHARS {
            let mut out = Vec::new();
            if char_len(&self.buf) >= MIN_SPEECH_CHARS {
                out.push(self.flush());
            }
            out.extend(split_long_speech(chunk));
            return out;
        }

        // Merge short chunks
        if !self.buf.is_empty() {
            self.buf.push(' ');
        }
        self.buf.push_str(chunk);

        // Flush when buffer is meaningful AND ends with sentence boundary, OR if it gets too long.
        let ends_with_punct = self.buf.ends_with('。')
            || self.buf.ends_with('！')
            || self.buf.ends_with('？')
            || self.buf.ends_with('.')
            || self.buf.ends_with('!')
            || self.buf.ends_with('?');

        let should_flush = (char_len(&self.buf) >= MIN_SPEECH_CHARS && ends_with_punct)
            || char_len(&self.buf) >= MAX_SPEECH_CHARS;

        if should_flush {
            let text = self.flush();
            // split again if it becomes too long
            return split_long_speech(&text);
        }

        vec![]
    }

    pub fn flush(&mut self) -> String {
        let t = self.buf.trim().to_string();
        self.buf.clear();
        t
    }
}

#[derive(Debug)]
pub enum PipelineCommand {
    StartRecording { recording_id: String },
    PauseRecording,
    ResumeRecording,
    StopRecording,
    ResetDocument,
    UpdateDocument(String),
    IngestDocument { filename: String, content: String },
    RollbackToCommit(String),
    UndoLastChange,
    LoadRecording { recording_id: String },
    DeleteRecording { recording_id: String },
    // Todo Commands
    AddTodo(String), // description
    UpdateTodo { id: String, description: String },
    ToggleTodo(String), // id
    DeleteTodo(String), // id
}
