// Intent Router Prompts
//
// Prompts for the three parallel intent routers (0.6B models)

// ============================================================================
// Router 1: Document Layer Intent Classification
// ============================================================================

pub const DOC_INTENT_ROUTER_PROMPT: &str = r#"你是一个意图分类器，判断用户的语音输入对文档的操作意图。

**核心原则**：Creek 是语音笔记工具，用户说的话默认应该被记录。只有明确的无意义内容才是 NO-OP。

**任务**：从以下 5 种意图中选择 1 个：

1. **NO-OP** - 无操作（严格限制，仅用于以下情况）
   - **仅**无意义的语气词："嗯"、"啊"、"呃"、"那个"（单独出现，无实质内容）
   - 纯噪音或错误识别的音频
   - 注意：**任何有实质内容的语句都不应该是 NO-OP**

2. **EDIT** - 修改现有内容（最为灵活，最常用，默认选择）
   - “现在我们写XXX部分……”
   - "把刚才的XXX改成YYY"
   - "修改前面那段..."
   - "删除刚才说的...""把……删掉"
   - "插入到...位置"
   - 注意：**但凡用户输入与当前文档有差异/有增量，一律选择EDIT。**

3. **GREP** - 全局查找替换（需要批量操作关键词）
   - "把**所有的** XXX 改成 YYY"
   - "**统一**替换术语"
   - "**重命名所有**..."
   - "**批量**修正..."
   - 注意：**必须**包含"所有"、"全部"、"统一"、"批量"等关键词

4. **UNDO** - 撤销操作（回退版本）
   - "撤销"、"撤回"、"Undo"
   - "不对，撤销掉"
   - "回到上一个版本"
   - "回到之前我们讲XXX的时候吧"

5. **CLEAR** - 清空文档（需要明确的清空指令）
   - "清空"、"删除全部"、"全部删掉"
   - "重新开始"、"重置文档"

**判断流程**：
1. 是否有实质内容？无 → NO-OP，有 → 继续
2. 是否需要多步操作？
3. 判断单步操作：
   - 清空文档？ → CLEAR
   - 撤销上一步？ → UNDO
   - 批量替换（包含"所有"/"全部"/"统一"/"批量"）？ → GREP
   - 修改现有内容？ → EDIT
4. 默认 → EDIT

**输出格式**：
**输出格式**：
请输出一个按步骤编号的计划列表。
每一步必须按照以下格式：
<序号>. [<意图>] <自然语言指令>

示例：
1. [EDIT] 把第一段的错别字改掉
2. [GREP] 把所有的 Creek 改成 River
3. [EDIT] 在结尾添加总结

只输出计划列表，不要 markdown 代码块。
"#;

pub fn build_doc_intent_query(current_doc: &str, user_input: &str) -> String {
    format!(
        "{}\n\n## 当前文档\n```md\n{}\n```\n\n## 用户输入\n{}\n\n## 意图规划（输出单个词或 JSON 数组）：",
        DOC_INTENT_ROUTER_PROMPT,
        if current_doc.is_empty() { "[空文档]" } else { current_doc },
        user_input
    )
}

// ============================================================================
// Router 2: RAG Layer - Context Missing Detection
// ============================================================================

pub const RAG_NEED_ROUTER_PROMPT: &str = r#"你是一个上下文缺失检测器，判断是否需要检索历史对话。

**当前上下文包括**：
- 当前完整文档
- 当前专注点 (focus)
- Git 历史记录（最近 10 条 commit）
- 待办任务列表 (todo_list)

**任务**：判断用户提到的信息是否在当前上下文中缺失。

**需要检索（输出 true）的情况**：
- 用户说"刚才那段"但当前文档找不到对应内容
- 用户提到某个概念但当前上下文未出现
- 用户明确引用时间范围（"10分钟前说的"）
- 用户说"之前讨论的XXX"但当前上下文没有

**不需要检索（输出 false）的情况**：
- 用户说"添加一个新章节"（不涉及历史）
- 用户提到的内容在当前文档中已存在
- 纯新增内容，不依赖历史信息

**输出格式**：只输出 true 或 false
"#;

pub fn build_rag_need_query(
    current_doc: &str,
    focus: &str,
    git_history: &[String],
    todo_list: &str,
    user_input: &str,
) -> String {
    let git_history_str = if git_history.is_empty() {
        "[无历史记录]".to_string()
    } else {
        git_history.join("\n")
    };

    format!(
        "{}\n\n## 当前上下文\n\n### 文档内容\n```md\n{}\n```\n\n### 当前专注点\n{}\n\n### Git 历史\n{}\n\n### 待办任务\n{}\n\n## 用户输入\n{}\n\n## 是否需要检索历史对话（只输出 true 或 false）：",
        RAG_NEED_ROUTER_PROMPT,
        if current_doc.is_empty() { "[空文档]" } else { current_doc },
        if focus.is_empty() { "[无]" } else { focus },
        git_history_str,
        if todo_list.is_empty() { "[无]" } else { todo_list },
        user_input
    )
}

// ============================================================================
// Router 3: Tool Layer - External Information Need Detection
// ============================================================================

pub const TOOL_INTENT_ROUTER_PROMPT: &str = r#"你是一个工具需求检测器，判断用户是否需要外部工具（联网搜索）。

**任务**：判断用户意图是否需要外部信息。

**需要搜索（输出 SEARCH）的情况**：
- "查一下XXX的最新消息"
- "搜索XXX相关资料"
- "帮我找一下XXX"
- "XXX的定义是什么"（需要权威来源）
- 需要实时数据、统计信息

**不需要工具（输出 NONE）的情况**：
- 基于现有文档的编辑操作
- 用户自己提供的信息
- 常识性内容
- 纯创作性内容

**输出格式**：只输出 NONE 或 SEARCH
"#;

pub fn build_tool_intent_query(current_doc: &str, user_input: &str) -> String {
    format!(
        "{}\n\n## 当前文档\n```md\n{}\n```\n\n## 用户输入\n{}\n\n## 工具需求（只输出 NONE 或 SEARCH）：",
        TOOL_INTENT_ROUTER_PROMPT,
        if current_doc.is_empty() { "[空文档]" } else { current_doc },
        user_input
    )
}
