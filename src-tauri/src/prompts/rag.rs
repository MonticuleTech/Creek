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
- User: "把刚才那段改一下" (Context has no recent edits) → Query: "最近一次编辑的内容"
- User: "10分钟前说的那个功能" → Query: "功能需求描述"
- User: "添加到前面说的列表里" (Context has no list) → Query: "之前提到的列表内容"
- User: "继续写介绍部分" (Context is empty or unrelated) → Query: "项目介绍 背景 目标"

## Output
Output ONLY the search query (no explanation, no preamble):
"#,
        doc_preview,
        user_input
    )
}
