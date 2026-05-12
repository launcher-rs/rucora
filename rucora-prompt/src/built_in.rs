//! 内置 Prompt 定义
//!
//! 按类别组织，支持多语言（中文/英文）

use super::template::PromptTemplate;
use std::collections::HashMap;

/// Prompt 类别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PromptCategory {
    /// Agent 类
    #[default]
    Agent,
    /// 工具类
    Tool,
    /// 研究类
    Research,
    /// 其他
    Other,
}

/// 语言
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    /// 中文
    #[default]
    Chinese,
    /// 英文
    English,
}

impl Language {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "en" | "english" | "en-US" => Language::English,
            _ => Language::Chinese,
        }
    }
}

/// 内置 Prompt 定义（静态常量）
/// 格式: (name, english, chinese)
pub static PROMPT_DEFS: &[(&str, &str, &str)] = &[
    // Agent
    (
        "agent_simple",
        "You are a simple and direct assistant.",
        "你是一个简单直接的助手。",
    ),
    (
        "agent_chat",
        "You are a friendly chat assistant.",
        "你是友好的对话助手。",
    ),
    (
        "agent_tool",
        "You are an intelligent assistant that can use tools to complete tasks.",
        "你是一个智能助手，可以通过工具完成任务。",
    ),
    (
        "agent_react",
        "You are a reasoning assistant. Follow ReAct: Think -> Act -> Observe -> Repeat",
        "你是一个推理助手。请遵循 ReAct 模式。",
    ),
    (
        "agent_reflect",
        "You are a reflective assistant. Evaluate, identify, improve, re-execute if needed.",
        "你是一个反思型助手。评估、识别不足、必要时重做。",
    ),
    // Tool
    (
        "tool_search",
        "You are a search assistant.",
        "你是一个搜索助手。",
    ),
    (
        "tool_summarize",
        "You are a summarization assistant.",
        "你是一个总结助手。",
    ),
    (
        "tool_translate",
        "You are a translation assistant.",
        "你是一个翻译助手。",
    ),
    (
        "tool_browse",
        "You are a web analysis assistant.",
        "你是一个网页分析助手。",
    ),
    (
        "tool_code",
        "You are a programming assistant.",
        "你是一个编程助手。",
    ),
    (
        "tool_file",
        "You are a file processing assistant.",
        "你是一个文件处理助手。",
    ),
    // Research
    (
        "research_default",
        "You are a professional research assistant.",
        "你是一个专业研究助手。",
    ),
    (
        "research_fast",
        "You are a quick research assistant.",
        "你是快速研究助手。",
    ),
    (
        "research_academic",
        "You are an academic research assistant.",
        "你是一个学术研究助手。",
    ),
    (
        "research_agentic",
        "You are an autonomous research agent.",
        "你是一个自主研究 Agent。",
    ),
];

/// 内置 Prompt 枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BuiltInPrompt {
    // === Agent ===
    #[default]
    AgentSimple,
    AgentChat,
    AgentTool,
    AgentReAct,
    AgentReflect,

    // === Tool ===
    ToolSearch,
    ToolSummarize,
    ToolTranslate,
    ToolBrowse,
    ToolCode,
    ToolFile,

    // === Research ===
    ResearchDefault,
    ResearchFast,
    ResearchAcademic,
    ResearchAgentic,
}

impl BuiltInPrompt {
    /// 按名称获取
    pub fn from_name(name: &str) -> Option<Self> {
        let n = name.to_lowercase();
        match n.as_str() {
            // Agent
            "simple" | "agent_simple" => Some(Self::AgentSimple),
            "chat" | "agent_chat" => Some(Self::AgentChat),
            "tool" | "agent_tool" => Some(Self::AgentTool),
            "react" | "agent_react" => Some(Self::AgentReAct),
            "reflect" | "agent_reflect" => Some(Self::AgentReflect),
            // Tool
            "search" | "tool_search" => Some(Self::ToolSearch),
            "summarize" | "tool_summarize" => Some(Self::ToolSummarize),
            "translate" | "tool_translate" => Some(Self::ToolTranslate),
            "browse" | "tool_browse" => Some(Self::ToolBrowse),
            "code" | "tool_code" => Some(Self::ToolCode),
            "file" | "tool_file" => Some(Self::ToolFile),
            // Research
            "research" | "research_default" => Some(Self::ResearchDefault),
            "fast" | "research_fast" => Some(Self::ResearchFast),
            "academic" | "research_academic" => Some(Self::ResearchAcademic),
            "agentic" | "research_agentic" => Some(Self::ResearchAgentic),
            _ => None,
        }
    }

    /// 获取类别
    pub fn category(&self) -> PromptCategory {
        match self {
            Self::AgentSimple
            | Self::AgentChat
            | Self::AgentTool
            | Self::AgentReAct
            | Self::AgentReflect => PromptCategory::Agent,
            Self::ToolSearch
            | Self::ToolSummarize
            | Self::ToolTranslate
            | Self::ToolBrowse
            | Self::ToolCode
            | Self::ToolFile => PromptCategory::Tool,
            Self::ResearchDefault
            | Self::ResearchFast
            | Self::ResearchAcademic
            | Self::ResearchAgentic => PromptCategory::Research,
        }
    }

    /// 获取名称
    pub fn name(&self) -> &'static str {
        match self {
            Self::AgentSimple => "agent_simple",
            Self::AgentChat => "agent_chat",
            Self::AgentTool => "agent_tool",
            Self::AgentReAct => "agent_react",
            Self::AgentReflect => "agent_reflect",
            Self::ToolSearch => "tool_search",
            Self::ToolSummarize => "tool_summarize",
            Self::ToolTranslate => "tool_translate",
            Self::ToolBrowse => "tool_browse",
            Self::ToolCode => "tool_code",
            Self::ToolFile => "tool_file",
            Self::ResearchDefault => "research_default",
            Self::ResearchFast => "research_fast",
            Self::ResearchAcademic => "research_academic",
            Self::ResearchAgentic => "research_agentic",
        }
    }

    /// 获取 prompt 定义（英文/中文）
    fn get_prompt_def(&self) -> (&'static str, &'static str) {
        for (name, en, zh) in PROMPT_DEFS {
            if *name == self.name() {
                return (*en, *zh);
            }
        }
        ("", "")
    }

    /// 转为 PromptTemplate（使用指定语言）
    pub fn template_with_lang(&self, lang: Language) -> PromptTemplate {
        let (en, zh) = self.get_prompt_def();
        let system = match lang {
            Language::English => en,
            Language::Chinese => zh,
        };
        PromptTemplate {
            name: self.name().to_string(),
            system: system.to_string(),
            template: self.user_template().to_string(),
        }
    }

    /// 转为 PromptTemplate（默认中文）
    pub fn template(&self) -> PromptTemplate {
        self.template_with_lang(Language::Chinese)
    }

    /// User prompt 模板
    fn user_template(&self) -> &'static str {
        match self {
            Self::AgentSimple | Self::AgentChat => "{{input}}",
            Self::AgentTool => "Task: {{input}}",
            Self::AgentReAct => "Question: {{input}}",
            Self::AgentReflect => "Task: {{input}}",
            Self::ToolSearch => "Search: {{query}}",
            Self::ToolSummarize => "Content: {{content}}",
            Self::ToolTranslate => "From {{from}} to {{to}}: {{content}}",
            Self::ToolBrowse => "URL: {{url}}",
            Self::ToolCode => "Task: {{task}}",
            Self::ToolFile => "File: {{path}}",
            Self::ResearchDefault | Self::ResearchFast => "Topic: {{topic}}",
            Self::ResearchAcademic => "Topic: {{topic}}",
            Self::ResearchAgentic => "Topic: {{topic}}, Iteration: {{iteration}}",
        }
    }
}

/// 获取所有内置 prompt 名称
pub fn all_prompt_names() -> Vec<&'static str> {
    PROMPT_DEFS.iter().map(|(name, _, _)| *name).collect()
}

/// 按类别获取所有 prompt
pub fn all_by_category() -> HashMap<PromptCategory, Vec<BuiltInPrompt>> {
    let mut map = HashMap::new();
    map.insert(
        PromptCategory::Agent,
        vec![
            BuiltInPrompt::AgentSimple,
            BuiltInPrompt::AgentChat,
            BuiltInPrompt::AgentTool,
            BuiltInPrompt::AgentReAct,
            BuiltInPrompt::AgentReflect,
        ],
    );
    map.insert(
        PromptCategory::Tool,
        vec![
            BuiltInPrompt::ToolSearch,
            BuiltInPrompt::ToolSummarize,
            BuiltInPrompt::ToolTranslate,
            BuiltInPrompt::ToolBrowse,
            BuiltInPrompt::ToolCode,
            BuiltInPrompt::ToolFile,
        ],
    );
    map.insert(
        PromptCategory::Research,
        vec![
            BuiltInPrompt::ResearchDefault,
            BuiltInPrompt::ResearchFast,
            BuiltInPrompt::ResearchAcademic,
            BuiltInPrompt::ResearchAgentic,
        ],
    );
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_name() {
        assert_eq!(
            BuiltInPrompt::from_name("tool"),
            Some(BuiltInPrompt::AgentTool)
        );
        assert_eq!(
            BuiltInPrompt::from_name("search"),
            Some(BuiltInPrompt::ToolSearch)
        );
    }

    #[test]
    fn test_language() {
        assert_eq!(Language::from_str("en"), Language::English);
        assert_eq!(Language::from_str("zh"), Language::Chinese);
    }

    #[test]
    fn test_template_with_lang() {
        let tmpl_en = BuiltInPrompt::AgentSimple.template_with_lang(Language::English);
        let tmpl_zh = BuiltInPrompt::AgentSimple.template_with_lang(Language::Chinese);

        assert!(tmpl_en.system.contains("assistant"));
        assert!(tmpl_zh.system.contains("助手"));
    }

    #[test]
    fn test_category() {
        assert_eq!(BuiltInPrompt::AgentTool.category(), PromptCategory::Agent);
        assert_eq!(BuiltInPrompt::ToolSearch.category(), PromptCategory::Tool);
    }
}
