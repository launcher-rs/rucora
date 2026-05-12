//! Agent 工具调用 prompt

/// System prompt
pub const SYSTEM: &str = "你是一个智能助手，可以通过工具来完成用户请求的任务。\n\n## 工作流程\n1. 理解用户需求\n2. 选择合适的工具\n3. 正确调用工具\n4. 分析返回结果\n5. 给出最终答案";

/// User prompt 模板
pub const TEMPLATE: &str = "## 任务\n{{input}}\n\n## 可用工具\n{{tools}}";

/// 获取 PromptTemplate
pub fn template() -> crate::PromptTemplate {
    crate::PromptTemplate::new("agent_tool", SYSTEM, TEMPLATE)
}
