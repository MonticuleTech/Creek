// RAG Module Prompts
use crate::utils::text::safe_truncate;

/// Generate prompt for query generation
pub fn build_query_generation_prompt(user_input: &str, current_doc: &str) -> String {
    let doc_preview = safe_truncate(current_doc, 300);
    
    format!(
        r#"You are a Query Agent in a RAG system.
CRITICAL CONTEXT: An upstream Intent Router has ALREADY determined that the user's request cannot be fulfilled with the Current Document Context alone. The information is missing.

## Current Document Context
```
{}
```

## User Input
{}

## Your Task
Generate a precise search query to find the MISSING information in the historical database (past conversations or ingested documents).
The user is likely referring to something mentioned previously that is not currently visible.

## Rules
1. Analyze what specific information is missing from the context to satisfy the user input.
2. Resolve pronouns (it, that, he, she) to their potential historical referents.
3. If the user refers to a past event ("what we discussed about X"), query for the content of that discussion.
4. The query should be keywords or a short phrase optimized for semantic search.

## Examples
- User: "Modify that previous part" (Context has no recent edits) → Query: "content of the most recent edit"
- User: "The feature mentioned 10 minutes ago" → Query: "description of feature requirements"
- User: "Add to the list mentioned before" (Context has no list) → Query: "content of the previously mentioned list"
- User: "Continue writing the introduction" (Context is empty or unrelated) → Query: "project introduction background goals"

## Output
Output ONLY the search query (no explanation, no preamble):
"#,
        doc_preview,
        user_input
    )
}
