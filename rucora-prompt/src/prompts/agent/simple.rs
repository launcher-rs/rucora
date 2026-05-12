//! Agent 简单问答 prompt
//!
//! # 使用方式
//! ```rust
//! use rucora_prompt::prompts::agent::simple;
//!
//! // 直接使用静态变量
//! println!("{}", simple::SYSTEM);
//! println!("{}", simple::TEMPLATE);
//!
//! // 使用 template() 函数
//! let tmpl = simple::template();
//! ```

/// System prompt
pub const SYSTEM: &str = r#"你是一个简单直接的助手。请简洁准确地回答用户问题。

## 核心原则
1. 直接回答问题，不绕弯子
2. 保持简洁，不说废话
3. 如不确定，明确说明
4. 提供有用但不过度的信息"#;

/// User prompt 模板
pub const TEMPLATE: &str = "{{input}}";

/// 获取 PromptTemplate
pub fn template() -> crate::PromptTemplate {
    crate::PromptTemplate::new("agent_simple", SYSTEM, TEMPLATE)
}
