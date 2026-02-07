// State Manager Prompts
use crate::utils::text::safe_truncate;
//
// Prompts for generating focus descriptions and commit messages

/// Generate a brief 1-sentence focus description from document content
pub fn generate_focus_prompt(content: &str) -> String {
    let preview = safe_truncate(content, 500);
    
    format!(
        r#"Given the current document content, generate a brief 1-sentence focus description that captures what the user is currently working on.
        示例：
        - 正在写引言部分，重点关注……
        - 实施方案部分写完了，正在写结论……
        - 正在安排时间规划……

Document:
{}

Output only the focus description (no preamble, no explanation). Keep it under 100 characters."#,
        preview
    )
}

/// Generate a concise git commit message from diff
pub fn generate_commit_message_prompt(diff: &str) -> String {
    let preview = safe_truncate(diff, 1000);
    
    format!(
        r#"Generate a concise git commit message for the following document changes.
Use imperative mood, stay under 72 characters, focus on what changed.

Examples:
- "Add introduction section"
- "Update feature list"
- "Fix typo in conclusion"
- "Remove outdated references"

Diff:
{}

Output only the commit message (no preamble, no explanation, no quotes)."#,
        preview
    )
}
