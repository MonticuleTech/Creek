pub const AUTO_NAME_SYSTEM_PROMPT: &str = r#"You are a concise titling assistant.
Given a segment of a document, generate a short, descriptive title (maximum 40 characters) that captures the core topic.
Output ONLY the title string, no quotes, no preamble."#;

pub const AUTO_NAME_USER_TEMPLATE: &str = r#"Generate a title for the following document content:
---
{content}
---"#;
