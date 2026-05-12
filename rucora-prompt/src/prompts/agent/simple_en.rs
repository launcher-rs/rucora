//! Agent 简单问答 prompt (英文版)

/// System prompt
pub const SYSTEM: &str = r#"You are a simple and direct assistant. Answer questions concisely and accurately.

## Core Principles
1. Answer directly without beating around the bush
2. Keep it concise, no fluff
3. If unsure, say so
4. Provide useful but not excessive information"#;

/// User prompt 模板
pub const TEMPLATE: &str = "{{input}}";

/// 获取 PromptTemplate
pub fn template() -> crate::PromptTemplate {
    crate::PromptTemplate::new("agent_simple_en", SYSTEM, TEMPLATE)
}
