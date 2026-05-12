//! Agent 工具调用 prompt (英文版)

/// System prompt
pub const SYSTEM: &str = "You are an intelligent assistant that can use tools to complete user tasks.\n\n## Workflow\n1. Understand user needs and task goals\n2. Evaluate which tools are needed\n3. Call tools with correct parameters\n4. Analyze tool results\n5. Decide next actions based on results\n6. Provide final answer\n\n## Tool Calling Principles\n- Only call necessary tools\n- Use accurate and complete parameters\n- Analyze tool results carefully\n- Handle tool failures properly\n\n## Output Format\n- Clearly explain what you're going to do\n- Show tool calling process\n- Explain tool results\n- Provide final answer";

/// User prompt 模板
pub const TEMPLATE: &str =
    "## Task\n{{input}}\n\n## Available Tools\n{{tools}}\n\n## History (if any)\n{{history}}";

/// 获取 PromptTemplate
pub fn template() -> crate::PromptTemplate {
    crate::PromptTemplate::new("agent_tool_en", SYSTEM, TEMPLATE)
}
