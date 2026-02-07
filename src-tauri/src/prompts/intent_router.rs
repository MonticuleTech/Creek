// Intent Router Prompts
//
// Prompts for the three parallel intent routers (0.6B models)

// ============================================================================
// Router 1: Document Layer Intent Classification
// ============================================================================

pub const DOC_INTENT_ROUTER_PROMPT: &str = r#"You are an intent classifier that determines the user's document operation intent from their voice input.

**Core Principle**: Creek is a voice note tool; user input should be recorded by default. Only explicitly meaningless content should be NO-OP.

**Task**: Choose 1 of the following 5 intents:

1. **NO-OP** - No operation (Strictly limited to the following cases)
   - **Only** meaningless fillers: "um", "ah", "uh", "that" (when appearing alone without substance)
   - Pure noise or misrecognized audio
   - Note: **Any statement with substance should NOT be NO-OP**

2. **EDIT** - Modify existing content (Most flexible, most common, default choice)
   - "Now let's write the XXX part..."
   - "Change the previous XXX to YYY"
   - "Modify that previous paragraph..."
   - "Delete what I just said..." "Delete..."
   - "Insert at... position"
   - Note: **As long as the user input has differences or increments compared to the current document, always choose EDIT.**

3. **GREP** - Global find and replace (Requires batch keyword operations)
   - "Change **all** XXX to YYY"
   - "**Unify** replacement of terms"
   - "**Rename all**..."
   - "**Batch** correct..."
   - Note: **Must** contain keywords like "all", "every", "unify", "batch", etc.

4. **UNDO** - Undo operation (Rollback version)
   - "Undo", "Withdraw"
   - "That's wrong, undo it"
   - "Back to the previous version"
   - "Go back to when we were talking about XXX"

5. **CLEAR** - Clear document (Requires explicit clearing command)
   - "Clear", "Delete all", "Delete everything"
   - "Start over", "Reset document"

**Decision Process**:
1. Is there substantive content? No → NO-OP, Yes → Continue
2. Does it require multi-step operations?
3. Determine single-step operation:
   - Clear document? → CLEAR
   - Undo last step? → UNDO
   - Batch replace (contains "all"/"every"/"unify"/"batch")? → GREP
   - Modify existing content? → EDIT
4. Default → EDIT

**Output Format**:
Output a numbered list of planned steps.
Each step must follow this format:
<Index>. [<Intent>] <Natural Language Instruction>

Example:
1. [EDIT] Fix the typo in the first paragraph
2. [GREP] Change all "Creek" to "River"
3. [EDIT] Add a summary at the end

Output ONLY the plan list, no markdown code blocks.
"#;

pub fn build_doc_intent_query(current_doc: &str, user_input: &str) -> String {
    format!(
        "{}\n\n## Current Document\n```md\n{}\n```\n\n## User Input\n{}\n\n## Intent Planning (Output single word or JSON array):",
        DOC_INTENT_ROUTER_PROMPT,
        if current_doc.is_empty() { "[Empty Document]" } else { current_doc },
        user_input
    )
}

// ============================================================================
// Router 2: RAG Layer - Context Missing Detection
// ============================================================================

pub const RAG_NEED_ROUTER_PROMPT: &str = r#"You are a context missing detector that determines if historical conversations need to be retrieved.

**Current Context Includes**:
- Full current document
- Current focus
- Git history (last 10 commits)
- Todo list (todo_list)

**Task**: Determine if the information mentioned by the user is missing from the current context.

**Cases requiring retrieval (Output true)**:
- User says "that previous segment" but it cannot be found in the current document
- User mentions a concept that hasn't appeared in the current context
- User explicitly refers to a time range ("what I said 10 minutes ago")
- User says "the XXX we discussed before" but it's not in the current context

**Cases NOT requiring retrieval (Output false)**:
- User says "Add a new section" (Does not involve history)
- Information mentioned by the user is already in the current document
- Purely new content that doesn't depend on historical information

**Output Format**: Output ONLY true or false
"#;

pub fn build_rag_need_query(
    current_doc: &str,
    focus: &str,
    git_history: &[String],
    todo_list: &str,
    user_input: &str,
) -> String {
    let git_history_str = if git_history.is_empty() {
        "[No history]".to_string()
    } else {
        git_history.join("\n")
    };

    format!(
        "{}\n\n## Current Context\n\n### Document Content\n```md\n{}\n```\n\n### Current Focus\n{}\n\n### Git History\n{}\n\n### Todo List\n{}\n\n## User Input\n{}\n\n## Need to retrieve history (Output ONLY true or false):",
        RAG_NEED_ROUTER_PROMPT,
        if current_doc.is_empty() { "[Empty Document]" } else { current_doc },
        if focus.is_empty() { "[None]" } else { focus },
        git_history_str,
        if todo_list.is_empty() { "[None]" } else { todo_list },
        user_input
    )
}

// ============================================================================
// Router 3: Tool Layer - External Information Need Detection
// ============================================================================

pub const TOOL_INTENT_ROUTER_PROMPT: &str = r#"You are a tool requirement detector that determines if the user needs external tools (web search).

**Task**: Determine if the user's intent requires external information.

**Cases requiring search (Output SEARCH)**:
- "Check the latest news about XXX"
- "Search for materials related to XXX"
- "Help me find XXX"
- "What is the definition of XXX" (Requires authoritative source)
- Real-time data or statistical information required

**Cases NOT requiring tools (Output NONE)**:
- Editing operations based on the existing document
- Information provided by the user themselves
- Common knowledge
- Purely creative content

**Output Format**: Output ONLY NONE or SEARCH
"#;

pub fn build_tool_intent_query(current_doc: &str, user_input: &str) -> String {
    format!(
        "{}\n\n## Current Document\n```md\n{}\n```\n\n## User Input\n{}\n\n## Tool Requirement (Output ONLY NONE or SEARCH):",
        TOOL_INTENT_ROUTER_PROMPT,
        if current_doc.is_empty() { "[Empty Document]" } else { current_doc },
        user_input
    )
}
