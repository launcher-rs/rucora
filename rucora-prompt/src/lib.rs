//! rucora-prompt
//!
//! Prompt 模板库，提供内置 prompt 和自定义加载能力。
//! 支持多语言（中文/英文），按类别组织，易于扩展。
//!
//! # 快速开始
//!
//! ```rust,ignore
//! use rucora_prompt::{prompt, prompt_with_lang};
//! use std::collections::HashMap;
//!
//! // 使用内置 prompt（默认中文）
//! let tmpl = prompt("tool");
//!
//! // 指定语言
//! let tmpl = prompt_with_lang("tool", "en");
//!
//! // 渲染
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

pub use built_in::{all_by_category, BuiltInPrompt, Language, PromptCategory};
pub use template::PromptTemplate;

/// 按名称获取 prompt（默认中文）
///
/// 优先查找内置 prompt，然后尝试加载文件。
///
/// # Arguments
/// - `name`: 内置名称 (如 "tool", "search") 或文件路径
pub fn prompt(name: &str) -> PromptTemplate {
    prompt_with_lang(name, "zh")
}

/// 按名称和语言获取 prompt
///
/// # Arguments
/// - `name`: 内置名称或文件路径
/// - `lang`: 语言代码 "zh" 或 "en"
pub fn prompt_with_lang(name: &str, lang: &str) -> PromptTemplate {
    // 1. 尝试内置
    if let Some(p) = BuiltInPrompt::from_name(name) {
        let language = Language::from_str(lang);
        return p.template_with_lang(language);
    }

    // 2. 尝试文件 (如果启用了 fs feature)
    #[cfg(feature = "fs")]
    if let Some(p) = loader::load(name) {
        return p;
    }

    // 3. 默认
    let language = Language::from_str(lang);
    BuiltInPrompt::default().template_with_lang(language)
}

/// 按名称获取并渲染（默认中文）
pub fn prompt_render(name: &str, vars: &std::collections::HashMap<String, String>) -> String {
    prompt(name).render(vars)
}

/// 按名称和语言获取并渲染
pub fn prompt_render_with_lang(
    name: &str,
    lang: &str,
    vars: &std::collections::HashMap<String, String>,
) -> String {
    prompt_with_lang(name, lang).render(vars)
}

/// 获取渲染后的消息（默认中文）
pub fn messages(
    name: &str,
    vars: &std::collections::HashMap<String, String>,
) -> Vec<template::Message> {
    prompt(name).messages(vars)
}

/// 获取渲染后的消息（指定语言）
pub fn messages_with_lang(
    name: &str,
    lang: &str,
    vars: &std::collections::HashMap<String, String>,
) -> Vec<template::Message> {
    prompt_with_lang(name, lang).messages(vars)
}

/// 获取所有内置 prompt 名称
pub fn list_prompts() -> Vec<(&'static str, &'static str)> {
    vec![
        // Agent
        ("agent_simple", "简单问答"),
        ("agent_chat", "对话"),
        ("agent_tool", "工具调用"),
        ("agent_react", "ReAct 推理"),
        ("agent_reflect", "反思"),
        // Tool
        ("tool_search", "搜索"),
        ("tool_summarize", "总结"),
        ("tool_translate", "翻译"),
        ("tool_browse", "浏览"),
        ("tool_code", "代码"),
        ("tool_file", "文件"),
        // Research
        ("research_default", "默认研究"),
        ("research_fast", "快速研究"),
        ("research_academic", "学术研究"),
        ("research_agentic", "自主研究"),
    ]
}
