use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct PendingPatch {
    pub search_block: String,
    pub replace_block: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchResult {
    pub start_index: usize,
    pub end_index: usize,
    pub confidence: f32,
}

#[derive(Debug)]
pub enum PatchError {
    NotFound,
    ApplyFailed(String),
}

impl fmt::Display for PatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PatchError::NotFound => write!(f, "Search block not found in document"),
            PatchError::ApplyFailed(msg) => write!(f, "Failed to apply patch: {}", msg),
        }
    }
}

impl std::error::Error for PatchError {}

// --- DiffParser ---

#[derive(Debug, Clone, PartialEq)]
enum ParserState {
    Idle,
    InSearch,
    InReplace,
}

pub struct DiffParser {
    state: ParserState,
    current_search: String,
    current_replace: String,
}

impl DiffParser {
    pub fn new() -> Self {
        Self {
            state: ParserState::Idle,
            current_search: String::new(),
            current_replace: String::new(),
        }
    }

    /// Processes a chunk of text and returns a list of completed patches.
    /// Note: This is a simplified line-based state machine for demonstration.
    /// In a real stream, we might need to handle partial lines.
    /// Here we assume the input `chunk` contains complete lines or we buffer properly.
    /// For this simplified version, we'll process line by line.
    pub fn process_line(&mut self, line: &str) -> Option<PendingPatch> {
        let trimmed = line.trim();

        match self.state {
            ParserState::Idle => {
                // Accept minor marker variations to avoid "no-op" when the model slightly
                // deviates (e.g. different number of < characters).
                if is_search_marker_line(trimmed) {
                    self.state = ParserState::InSearch;
                    self.current_search.clear();
                    self.current_replace.clear();
                }
            }
            ParserState::InSearch => {
                // Separator line between SEARCH and REPLACE is commonly rendered as
                // "=======" but models sometimes emit different counts; treat any
                // "mostly equals" line as separator.
                if is_separator_line(trimmed) {
                    self.state = ParserState::InReplace;
                } else {
                    self.current_search.push_str(line);
                    // Only add newline if the line doesn't already end with one
                    // and it's likely part of a multi-line block.
                    // Actually, push_str takes the slice as-is. 
                    // If 'line' comes from a file iterator that strips newlines (like `lines()`), we need to add it.
                    // If it comes from `split_inclusive`, it has it.
                    // The bug was unconditionally adding `\n`.
                    if !line.ends_with('\n') {
                        self.current_search.push('\n');
                    }
                }
            }
            ParserState::InReplace => {
                if is_replace_marker_line(trimmed) {
                    self.state = ParserState::Idle;
                    let patch = PendingPatch {
                        search_block: self.current_search.trim_end().to_string(),
                        replace_block: self.current_replace.trim_end().to_string(),
                    };
                    return Some(patch);
                } else {
                    self.current_replace.push_str(line);
                    if !line.ends_with('\n') {
                        self.current_replace.push('\n');
                    }
                }
            }
        }
        None
    }

    /// Parse all complete SEARCH/REPLACE blocks from a full text buffer.
    /// Incomplete trailing blocks are ignored.
    pub fn parse_all(text: &str) -> Vec<PendingPatch> {
        // Normalize newlines so the parser can operate line-by-line even if the model
        // (or upstream streaming parser) emitted CR-only or CRLF newlines.
        let text = text.replace("\r\n", "\n").replace('\r', "\n");

        let mut parser = DiffParser::new();
        let mut patches = Vec::new();
        for line in text.split_inclusive('\n') {
            if let Some(p) = parser.process_line(line) {
                // Discard patches that try to inject protocol markers into the document
                // (we only accept markers as delimiters, not as document content).
                let replace_has_markers = p.replace_block.contains("<<<<<<< SEARCH")
                    || p.replace_block.contains(">>>>>>> REPLACE")
                    || p.replace_block.contains("=======");

                if !replace_has_markers {
                    patches.push(p);
                }
            }
        }
        patches
    }
}

fn is_separator_line(s: &str) -> bool {
    let t = s.trim();
    if t.len() < 3 {
        return false;
    }
    // allow "=" with optional surrounding whitespace (trimmed above)
    t.chars().all(|c| c == '=')
}

fn is_search_marker_line(s: &str) -> bool {
    let t = s.trim_start();
    // Must mention SEARCH and start with a run of '<' to avoid false positives.
    t.contains("SEARCH") && t.chars().take_while(|c| *c == '<').count() >= 3
}

fn is_replace_marker_line(s: &str) -> bool {
    let t = s.trim_start();
    t.contains("REPLACE") && t.chars().take_while(|c| *c == '>').count() >= 3
}

// --- FuzzyMatcher ---

pub struct FuzzyMatcher;

impl FuzzyMatcher {
    /// Level 1: Exact Match
    pub fn locate_exact(doc: &str, search_text: &str) -> Option<MatchResult> {
        doc.find(search_text).map(|start| MatchResult {
            start_index: start,
            end_index: start + search_text.len(),
            confidence: 1.0,
        })
    }

    /// Level 2: Line-based Normalized Match (ignore indentation/trim)
    pub fn locate_normalized(doc: &str, search_text: &str) -> Option<MatchResult> {
        let doc_lines: Vec<&str> = doc.lines().collect();
        let search_lines: Vec<&str> = search_text.lines().collect();

        if search_lines.is_empty() {
            return None;
        }

        // Pre-normalize search lines (trim whitespace)
        let normalized_search: Vec<String> = search_lines.iter()
            .map(|l| l.trim().to_string())
            .collect();

        // Scan document lines
        for i in 0..doc_lines.len() {
            if i + search_lines.len() > doc_lines.len() {
                break;
            }

            // Check if window matches
            let mut match_found = true;
            for j in 0..search_lines.len() {
                if doc_lines[i + j].trim() != normalized_search[j] {
                    match_found = false;
                    break;
                }
            }

            if match_found {
                // Found logic match! Now find byte offsets.
                // Reconstruct the matching block range in the original doc
                // We need to account for line endings (lines() strips them)
                
                // Calculate start byte index
                // Find the byte offset of the start of doc_lines[i]
                // We know doc_lines are just slices of doc.
                let ptr_diff = doc_lines[i].as_ptr() as usize - doc.as_ptr() as usize;
                let start_idx = ptr_diff;

                // Calculate end byte index (end of the last matching line)
                let last_line = doc_lines[i + search_lines.len() - 1];
                let end_idx = (last_line.as_ptr() as usize - doc.as_ptr() as usize) + last_line.len();

                return Some(MatchResult {
                    start_index: start_idx,
                    end_index: end_idx,
                    confidence: 0.9,
                });
            }
        }
        None
    }
}


// --- Apply Patch ---

pub fn apply_patch(doc: &str, patch: PendingPatch) -> Result<String, PatchError> {
    // 1. Try Exact Match
    if let Some(m) = FuzzyMatcher::locate_exact(doc, &patch.search_block) {
        let mut new_doc = String::with_capacity(doc.len() + patch.replace_block.len());
        new_doc.push_str(&doc[..m.start_index]);
        new_doc.push_str(&patch.replace_block);
        new_doc.push_str(&doc[m.end_index..]);
        return Ok(new_doc);
    }

    // 2. Try Normalized Match
    if let Some(m) = FuzzyMatcher::locate_normalized(doc, &patch.search_block) {
        let mut new_doc = String::with_capacity(doc.len() + patch.replace_block.len());
        new_doc.push_str(&doc[..m.start_index]);
        new_doc.push_str(&patch.replace_block);
        new_doc.push_str(&doc[m.end_index..]);
        return Ok(new_doc);
    }

    Err(PatchError::NotFound)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_simple() {
        let mut parser = DiffParser::new();
        let input = vec![
            "Some text before",
            "<<<<<<< SEARCH",
            "foo",
            "bar",
            "=======",
            "baz",
            "qux",
            ">>>>>>> REPLACE",
            "Some text after"
        ];
        
        let mut patches = Vec::new();
        for line in input {
            if let Some(p) = parser.process_line(line) {
                patches.push(p);
            }
        }

        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].search_block, "foo\nbar");
        assert_eq!(patches[0].replace_block, "baz\nqux");
    }

    #[test]
    fn test_matcher_exact() {
        let doc = "Hello world\nThis is a test\nGoodbye";
        let search = "This is a test";
        let m = FuzzyMatcher::locate_exact(doc, search).unwrap();
        assert_eq!(&doc[m.start_index..m.end_index], search);
    }

    #[test]
    fn test_matcher_normalized() {
        let doc = "Hello   world\n\tThis is a test\nGoodbye";
        let search = "This is a test"; // No whitespace/different whitespace
        let m = FuzzyMatcher::locate_normalized(doc, search).unwrap();
        // The extracted text should cover the relevant part in original doc
        let extracted = &doc[m.start_index..m.end_index];
        // It might include the leading/trailing whitespace depending on mapping, 
        // but let's check if it "contains" the non-whitespace parts.
        // Actually our normalize map logic maps to the first non-whitespace char index.
        // So "  This" -> map points to 'T'.
        
        // Let's verify carefully.
        // doc: "Hello   world\n\tThis is a test\nGoodbye"
        // norm: "HelloworldThisisatestGoodbye"
        // search: "This is a test" -> "Thisisatest"
        
        // Match should start at 'T' of "This" and end after 't' of "test".
        
        assert!(extracted.contains("This"));
        assert!(extracted.contains("test"));
        
        // Check apply_patch with normalized
        let patch = PendingPatch {
            search_block: "This is a test".to_string(),
            replace_block: "That was a test".to_string(),
        };
        let new_doc = apply_patch(doc, patch).unwrap();
        assert!(new_doc.contains("That was a test"));
        assert!(!new_doc.contains("This is a test"));
        assert!(new_doc.contains("Hello   world")); // Preserves other parts
    }
    
    #[test]
    fn test_apply_patch_not_found() {
        let doc = "Hello world";
        let patch = PendingPatch {
            search_block: "Not here".to_string(),
            replace_block: "New".to_string(),
        };
        let res = apply_patch(doc, patch);
        assert!(matches!(res, Err(PatchError::NotFound)));
    }
}
