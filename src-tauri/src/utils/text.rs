/// Safely truncate a string to a maximum number of bytes without breaking UTF-8 boundaries.
pub fn safe_truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        s
    } else {
        let mut end = max_bytes;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        &s[..end]
    }
}

/// Strip common LLM output wrappers like markdown fences and brackets.
pub fn clean_concise_output(s: &str) -> String {
    let s = s.trim();
    
    // Strip markdown fences if they wrap the whole thing
    let cleaned = if s.starts_with("```") {
        let lines: Vec<&str> = s.lines().collect();
        if lines.len() >= 2 && lines.last().map(|l| l.trim()).unwrap_or("") == "```" {
            lines[1..lines.len()-1].join("\n").trim().to_string()
        } else {
            s.to_string()
        }
    } else {
        s.to_string()
    };

    cleaned
}

