// Todo Agent Prompts

use crate::modules::TodoItem;
use crate::utils::text::safe_truncate;

/// Generate prompt for todo maintenance
/// 
/// IMPORTANT: Uses FIXED template structure to maximize KV cache hit rate.
pub fn build_todo_maintenance_prompt(
    current_doc: &str,
    current_todos: &[TodoItem],
    user_input: &str,
    recent_changes: &str,
) -> String {
    // Fixed length document preview for KV cache
    let doc_preview = safe_truncate(current_doc, 500);
    
    // FIXED STRUCTURE: Always show section, use [None] placeholder if empty
    let active_todo_count = current_todos.iter().filter(|t| !t.completed).count();
    let todos_text = if current_todos.is_empty() {
        "[None]".to_string()
    } else {
        current_todos.iter()
            .map(|t| {
                if t.completed {
                    let turns_info = if let Some(turns) = t.completed_turns_ago {
                        format!(" - {} turns ago", turns)
                    } else {
                        String::new()
                    };
                    format!("- [{}] {} (completed{})", t.id, t.desc, turns_info)
                } else {
                    format!("- [{}] {} (pending)", t.id, t.desc)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    
    // FIXED STRUCTURE: Always show Recent Changes section
    let recent_changes_text = if recent_changes.trim().is_empty() {
        "[None]"
    } else {
        recent_changes
    };
    
    // FIXED STRUCTURE: Always show status line (for KV cache)
    let status_line = if active_todo_count > 10 {
        format!("STATUS: {} active items - EXCEEDS LIMIT, delete at least {} immediately", 
                active_todo_count, active_todo_count - 8)
    } else if active_todo_count > 8 {
        format!("STATUS: {} active items - approaching limit, consider cleanup", active_todo_count)
    } else {
        format!("STATUS: {} active items - within limit", active_todo_count)
    };
    
    format!(
        r#"You are a Todo Agent that maintains a todo list for document writing.

## Current Document
```
{}
```

## Current Todos
{}

{}

## Recent Changes
{}

## User Input
{}

## Your Task
Analyze the user input and document state, then decide what todo operations to perform:
1. If user input completes a todo, mark it as complete
2. If a todo description needs updating, update it
3. If a todo is no longer relevant, delete it
4. If new tasks emerge from the document, add new todos
5. **CRITICAL**: If document is empty or very short (< 50 chars), DELETE ALL todos
6. **CRITICAL**: If todo list exceeds 10 items, aggressively delete less important ones
7. **IMPORTANT**: Todos marked as "completed" are tasks ALREADY DONE - do NOT add similar tasks again

## Important Rules
- Keep the todo list concise (max 5-10 items, STRICTLY enforce)
- Only track actionable, document-related tasks
- Be conservative: don't add todos unless clearly needed
- Use clear, concise descriptions (max 50 characters)
- When document is cleared/empty, clear all todos
- When list is too long (>10), prioritize and delete low-priority items

## Output Format
Output ONLY a valid JSON object (no markdown, no explanation):
{{
  "operations": [
    {{"action": "complete", "todo_id": "existing-id"}},
    {{"action": "update", "todo_id": "existing-id", "new_desc": "new description"}},
    {{"action": "delete", "todo_id": "existing-id"}},
    {{"action": "add", "desc": "new todo description"}}
  ]
}}

If no operations needed, output: {{"operations": []}}
"#,
        doc_preview,
        todos_text,
        status_line,
        recent_changes_text,
        user_input
    )
}
