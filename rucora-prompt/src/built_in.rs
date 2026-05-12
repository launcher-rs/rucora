//! 内置 Prompt 定义

use super::template::PromptTemplate;

/// 内置 Prompt 枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BuiltInPrompt {
    // === Agent ===
    /// 简单问答
    #[default]
    Simple,
    /// 对话
    Chat,
    /// 工具调用
    Tool,
    /// ReAct 推理
    ReAct,
    /// 反思
    Reflect,

    // === Tool ===
    /// 搜索
    Search,
    /// 总结
    Summarize,
    /// 翻译
    Translate,
    /// 浏览
    Browse,
    /// 代码
    Code,
    /// 文件
    File,

    // === Research ===
    /// 默认研究
    Research,
    /// 快速研究
    FastResearch,
    /// 学术研究
    Academic,
    /// Agentic 研究
    Agentic,
}

impl BuiltInPrompt {
    /// 创建新的自定义 prompt
    pub fn custom(name: &str, system: &str, template: &str) -> PromptTemplate {
        PromptTemplate::new(name, system, template)
    }

    /// 按名称获取
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            // Agent
            "simple" | "agent_simple" => Some(Self::Simple),
            "chat" | "agent_chat" => Some(Self::Chat),
            "tool" | "agent_tool" => Some(Self::Tool),
            "react" | "agent_react" => Some(Self::ReAct),
            "reflect" | "agent_reflect" => Some(Self::Reflect),

            // Tool
            "search" | "tool_search" => Some(Self::Search),
            "summarize" | "tool_summarize" => Some(Self::Summarize),
            "translate" | "tool_translate" => Some(Self::Translate),
            "browse" | "tool_browse" => Some(Self::Browse),
            "code" | "tool_code" => Some(Self::Code),
            "file" | "tool_file" => Some(Self::File),

            // Research
            "research" | "deep_research" => Some(Self::Research),
            "fast" | "fast_research" => Some(Self::FastResearch),
            "academic" | "research_academic" => Some(Self::Academic),
            "agentic" | "research_agentic" => Some(Self::Agentic),

            _ => None,
        }
    }

    /// 获取名称
    pub fn name(&self) -> &'static str {
        match self {
            Self::Simple => "simple",
            Self::Chat => "chat",
            Self::Tool => "tool",
            Self::ReAct => "react",
            Self::Reflect => "reflect",
            Self::Search => "search",
            Self::Summarize => "summarize",
            Self::Translate => "translate",
            Self::Browse => "browse",
            Self::Code => "code",
            Self::File => "file",
            Self::Research => "research",
            Self::FastResearch => "fast_research",
            Self::Academic => "academic",
            Self::Agentic => "agentic",
        }
    }

    /// 转为 PromptTemplate
    pub fn template(&self) -> PromptTemplate {
        PromptTemplate {
            name: self.name().to_string(),
            system: self.system_prompt().to_string(),
            template: self.user_template().to_string(),
        }
    }

    /// System prompt
    pub fn system_prompt(&self) -> &'static str {
        match self {
            // === Agent ===
            Self::Simple => "你是一个简单直接的助手。请简洁准确地回答用户问题。",
            Self::Chat => "你是友好的对话助手。请保持自然、友好的交流风格，记住对话历史。",
            Self::Tool => {
                r#"你是一个智能助手，可以通过工具来完成用户请求的任务。
请遵循以下步骤：
1. 理解用户需求
2. 选择合适的工具
3. 正确调用工具
4. 处理返回结果
5. 给出最终答案"#
            }
            Self::ReAct => {
                r#"你是一个推理助手。请遵循 ReAct (Reasoning + Acting) 模式：
- 思考：分析问题，制定计划
- 行动：调用工具获取信息
- 观察：分析工具返回结果
- 重复直到完成目标"#
            }
            Self::Reflect => {
                r#"你是一个反思型助手。请在完成任务后进行反思：
1. 评估结果质量
2. 识别不足之处
3. 提出改进方案
4. 必要时重新执行"#
            }

            // === Tool ===
            Self::Search => "你是一个搜索助手。请根据用户需求生成有效的搜索查询。",
            Self::Summarize => "你是一个总结助手。请将给定内容压缩为简洁摘要，保留核心信息。",
            Self::Translate => {
                "你是一个翻译助手。请准确翻译给定内容，保持原意，使用目标语言的自然表达。"
            }
            Self::Browse => "你是一个网页分析助手。请分析提取关键信息，识别主要内容。",
            Self::Code => "你是一个编程助手。请帮助用户完成编程任务，提供清晰可用的代码。",
            Self::File => "你是一个文件处理助手。请帮助用户处理文件操作。",

            // === Research ===
            Self::Research => {
                r#"你是一名专业研究助手，负责对给定主题进行深入研究。
请遵循以下原则：
1. 基于可靠的信息源
2. 提供有据可查的分析
3. 保持客观中立
4. 结构化输出结果"#
            }
            Self::FastResearch => {
                "你是快速研究助手。请简洁地回答用户问题，突出重点，快速获取信息。"
            }
            Self::Academic => {
                r#"你是学术研究助手。请以学术规范进行分析：
- 引用权威来源，标注出处
- 使用专业术语
- 保持客观严谨"#
            }
            Self::Agentic => {
                r#"你是自主研究 Agent。你可以根据研究进展自主决策：
1. 决定搜索策略和方向
2. 判断信息是否足够
3. 选择下一步行动
4. 在适当时机终止研究"#
            }
        }
    }

    /// User prompt 模板
    pub fn user_template(&self) -> &'static str {
        match self {
            // Agent
            Self::Simple | Self::Chat => "{{input}}",
            Self::Tool => "任务: {{input}}\n可用工具: {{tools}}",
            Self::ReAct => "问题: {{input}}\n历史: {{history}}",
            Self::Reflect => "任务: {{input}}\n结果: {{result}}",

            // Tool
            Self::Search => "搜索: {{query}}\n目标: {{goal}}",
            Self::Summarize => "总结:\n{{content}}\n\n要求: {{requirements}}",
            Self::Translate => "翻译:\n{{content}}\n\n从 {{from}} 到 {{to}}",
            Self::Browse => "网址: {{url}}\n内容: {{content}}",
            Self::Code => "任务: {{task}}\n\n语言: {{language}}\n要求: {{requirements}}",
            Self::File => "文件: {{path}}\n操作: {{action}}",

            // Research
            Self::Research | Self::FastResearch => "研究主题: {{topic}}",
            Self::Academic => "学术研究主题: {{topic}}\n参考: {{sources}}",
            Self::Agentic => "研究任务: {{topic}}\n已收集: {{collected}}\n轮次: {{iteration}}",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_name() {
        assert_eq!(BuiltInPrompt::from_name("tool"), Some(BuiltInPrompt::Tool));
        assert_eq!(
            BuiltInPrompt::from_name("search"),
            Some(BuiltInPrompt::Search)
        );
        assert_eq!(
            BuiltInPrompt::from_name("research"),
            Some(BuiltInPrompt::Research)
        );
        assert_eq!(BuiltInPrompt::from_name("unknown"), None);
    }

    #[test]
    fn test_template() {
        let tmpl = BuiltInPrompt::Tool.template();
        assert_eq!(tmpl.name, "tool");
        assert!(!tmpl.system.is_empty());
        assert!(!tmpl.template.is_empty());
    }
}
