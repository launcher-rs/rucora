//! rucora-prompt
//!
//! Prompt 模板库，提供内置 prompt 和自定义加载能力。
//!
//! # 快速开始
//!
//! ```rust,ignore
//! use rucora_prompt::prompt;
//! use std::collections::HashMap;
//!
//! // 使用内置 prompt
//! let tmpl = prompt("tool");
//! let mut vars = HashMap::new();
//! vars.insert("input".to_string(), "hello".to_string());
//! let output = tmpl.render(&vars);
//!
//! // 使用自定义文件
//! let tmpl = prompt("config/my_prompt.toml");
//! ```

pub mod built_in;
#[cfg(feature = "fs")]
pub mod loader;
pub mod template;

pub use built_in::BuiltInPrompt;
pub use template::PromptTemplate;

/// 按名称获取 prompt
///
/// 优先查找内置 prompt，然后尝试加载文件。
///
/// # Arguments
/// - `name`: 内置名称 (如 "tool", "search") 或文件路径
///
/// # Example
/// ```rust
/// let tmpl = rucora_prompt::prompt("tool");
/// let tmpl = rucora_prompt::prompt("search");
/// let tmpl = rucora_prompt::prompt("custom.toml");
/// ```
pub fn prompt(name: &str) -> PromptTemplate {
    // 1. 尝试内置
    if let Some(p) = BuiltInPrompt::from_name(name) {
        return p.template();
    }

    // 2. 尝试文件 (如果启用了 fs feature)
    #[cfg(feature = "fs")]
    if let Some(p) = loader::load(name) {
        return p;
    }

    // 3. 默认
    BuiltInPrompt::default().template()
}

/// 按名称获取并渲染
pub fn prompt_render(name: &str, vars: &std::collections::HashMap<String, String>) -> String {
    prompt(name).render(vars)
}

/// 获取渲染后的消息
pub fn messages(
    name: &str,
    vars: &std::collections::HashMap<String, String>,
) -> Vec<template::Message> {
    prompt(name).messages(vars)
}
