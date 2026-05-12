# rucora Prompt 完整设计方案

## 设计目标

为 rucora 所有模块提供统一的 prompt 配置能力：
- Deep Research - 研究策略
- Agent - 各类 Agent
- 工具调用 - Tool 执行的 prompt
- MCP - MCP 工具配置

---

## 整体架构

```
┌─────────────────────────────────────────────────────────┐
│                     PromptManager                        │
├─────────────────────────────────────────────────────────┤
│  BuiltInPrompts    │  CustomPrompts    │  Inline       │
│  ┌──────────────┐  │  ┌──────────────┐  │  ┌────────┐  │
│  │ research    │  │  │ 文件加载     │  │  │ 内联   │  │
│  │ agent       │  │  │ (TOML/YAML)  │  │  │ 字符串 │  │
│  │ tool        │  │  │              │  │  │        │  │
│  │ mcp         │  │  │              │  │  │        │  │
│  └──────────────┘  │  └──────────────┘  │  └────────┘  │
└─────────────────────────────────────────────────────────┘
```

---

## 1. 统一枚举定义

```rust
// rucora-core/src/prompt/mod.rs

use std::path::PathBuf;

/// Prompt 类别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptCategory {
    /// 研究类
    Research,
    /// Agent 类
    Agent,
    /// 工具类
    Tool,
    /// MCP 类
    Mcp,
}

/// 内置 Prompt 枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BuiltInPrompt {
    // ========== 研究类 ==========
    #[default]
    ResearchDefault,
    ResearchConcise,
    ResearchDetailed,
    ResearchAcademic,
    ResearchAgentic,

    // ========== Agent 类 ==========
    /// 简单问答
    AgentSimple,
    /// 对话
    AgentChat,
    /// 工具调用
    AgentTool,
    /// ReAct 推理
    AgentReAct,
    /// 反思
    AgentReflect,

    // ========== 工具类 ==========
    /// 搜索
    ToolSearch,
    /// 浏览器
    ToolBrowse,
    /// 总结
    ToolSummarize,
    /// 翻译
    ToolTranslate,
    /// 代码执行
    ToolCode,
    /// 文件处理
    ToolFile,

    // ========== MCP 类 ==========
    /// MCP 通用
    McpDefault,
    /// MCP 文件操作
    McpFile,
    /// MCP 数据库
    McpDatabase,
}

impl BuiltInPrompt {
    /// 获取类别
    pub fn category(&self) -> PromptCategory {
        match self {
            // Research
            Self::ResearchDefault | Self::ResearchConcise | Self::ResearchDetailed 
            | Self::ResearchAcademic | Self::ResearchAgentic => PromptCategory::Research,
            
            // Agent
            Self::AgentSimple | Self::AgentChat | Self::AgentTool 
            | Self::AgentReAct | Self::AgentReflect => PromptCategory::Agent,
            
            // Tool
            Self::ToolSearch | Self::ToolBrowse | Self::ToolSummarize 
            | Self::ToolTranslate | Self::ToolCode | Self::ToolFile => PromptCategory::Tool,
            
            // MCP
            Self::McpDefault | Self::McpFile | Self::McpDatabase => PromptCategory::Mcp,
        }
    }

    /// 获取名称
    pub fn name(&self) -> &'static str {
        match self {
            Self::ResearchDefault => "research_default",
            Self::ResearchConcise => "research_concise",
            Self::ResearchDetailed => "research_detailed",
            Self::ResearchAcademic => "research_academic",
            Self::ResearchAgentic => "research_agentic",
            Self::AgentSimple => "agent_simple",
            Self::AgentChat => "agent_chat",
            Self::AgentTool => "agent_tool",
            Self::AgentReAct => "agent_react",
            Self::AgentReflect => "agent_reflect",
            Self::ToolSearch => "tool_search",
            Self::ToolBrowse => "tool_browse",
            Self::ToolSummarize => "tool_summarize",
            Self::ToolTranslate => "tool_translate",
            Self::ToolCode => "tool_code",
            Self::ToolFile => "tool_file",
            Self::McpDefault => "mcp_default",
            Self::McpFile => "mcp_file",
            Self::McpDatabase => "mcp_database",
        }
    }

    /// 获取 system prompt
    pub fn system_prompt(&self) -> &'static str {
        match self {
            // ========== 研究类 ==========
            Self::ResearchDefault => r#"
你是一名专业研究助手，负责对给定主题进行深入研究。
请遵循以下原则：
1. 基于可靠的信息源
2. 提供有据可查的分析
3. 保持客观中立
4. 结构化输出结果
"#,
            Self::ResearchConcise => r#"
你是研究助手。请简洁地回答用户问题，突出重点。
"#,
            Self::ResearchDetailed => r#"
你是资深研究专家。请进行深入全面的分析：
- 多角度审视问题
- 提供详实的案例和数据
- 识别潜在风险和机会
- 给出具体的建议
"#,
            Self::ResearchAcademic => r#"
你是学术研究助手。请以学术规范进行分析：
- 引用权威来源，标注出处
- 使用专业术语
- 保持客观严谨
"#,
            Self::ResearchAgentic => r#"
你是自主研究Agent。你可以根据研究进展自主决策：
1. 决定搜索策略和方向
2. 判断信息是否足够
3. 选择下一步行动
4. 在适当时机终止研究
"#,

            // ========== Agent 类 ==========
            Self::AgentSimple => r#"
你是一个简单直接的助手。请简洁准确地回答用户问题。
"#,
            Self::AgentChat => r#"
你是一个友好的对话助手。请保持自然、友好的交流风格。
记住对话历史，提供连贯的回复。
"#,
            Self::AgentTool => r#"
你是一个智能助手，可以通过工具来完成用户请求的任务。
在执行任务时：
1. 先理解用户意图
2. 合理选择和调用工具
3. 评估工具返回的结果
4. 根据结果决定下一步行动
"#,
            Self::AgentReAct => r#"
你是一个推理助手。请遵循 ReAct (Reasoning + Acting) 模式：
1. 思考：分析问题，制定计划
2. 行动：调用工具获取信息
3. 观察：分析工具返回结果
4. 重复直到完成目标
"#,
            Self::AgentReflect => r#"
你是一个反思型助手。请在完成任务后进行反思：
1. 评估结果质量
2. 识别不足之处
3. 提出改进方案
4. 必要时重新执行
"#,

            // ========== 工具类 ==========
            Self::ToolSearch => r#"
你是一个搜索助手。请根据用户需求生成有效的搜索查询。
规则：
1. 提取核心关键词
2. 使用精确的搜索术语
3. 考虑可能的同义词
4. 添加时间和范围限制（如需要）
"#,
            Self::ToolBrowse => r#"
你是一个网页浏览助手。请分析网页内容：
1. 提取关键信息
2. 识别主要内容
3. 总结核心观点
4. 注意信息来源的可靠性
"#,
            Self::ToolSummarize => r#"
你是一个总结助手。请将给定内容压缩为简洁摘要：
1. 保留核心信息
2. 去除冗余细节
3. 使用清晰的结构
4. 控制在指定长度内
"#,
            Self::ToolTranslate => r#"
你是一个翻译助手。请准确翻译给定内容：
1. 保持原意不变
2. 使用目标语言的自然表达
3. 注意专业术语的翻译
4. 必要时提供解释
"#,
            Self::ToolCode => r#"
你是一个编程助手。请帮助用户完成编程任务：
1. 理解需求和约束
2. 提供清晰可用的代码
3. 解释关键实现逻辑
4. 指出可能的边界情况
"#,
            Self::ToolFile => r#"
你是一个文件处理助手。请帮助用户处理文件：
1. 理解文件内容和结构
2. 按需求进行处理
3. 说明操作结果
4. 提醒注意事项
"#,

            // ========== MCP 类 ==========
            Self::McpDefault => r#"
你是 MCP 工具的协调助手。请：
1. 理解用户请求
2. 选择合适的工具
3. 构建正确的参数
4. 处理工具返回结果
"#,
            Self::McpFile => r#"
你是一个文件管理助手。请：
1. 理解文件操作需求
2. 使用适当的文件工具
3. 验证操作结果
4. 提供清晰的反馈
"#,
            Self::McpDatabase => r#"
你是数据库助手。请：
1. 理解数据查询需求
2. 编写正确的 SQL
3. 解释查询结果
4. 建议数据使用方式
"#,
        }
    }

    /// 获取 user prompt 模板
    pub fn user_template(&self) -> &'static str {
        match self {
            // 研究类
            Self::ResearchDefault | Self::ResearchConcise | Self::ResearchDetailed 
            | Self::ResearchAcademic | Self::ResearchAgentic => "{{topic}}",
            
            // Agent 类
            Self::AgentSimple | Self::AgentChat | Self::AgentTool 
            | Self::AgentReAct | Self::AgentReflect => "{{input}}",
            
            // 工具类
            Self::ToolSearch => "搜索: {{query}}",
            Self::ToolBrowse => "分析网址: {{url}}\n内容: {{content}}",
            Self::ToolSummarize => "总结以下内容:\n{{content}}\n\n要求: {{requirements}}",
            Self::ToolTranslate => "翻译以下内容:\n{{content}}\n\n从 {{from_lang}} 到 {{to_lang}}",
            Self::ToolCode => "任务: {{task}}\n\n要求: {{requirements}}",
            Self::ToolFile => "文件: {{path}}\n操作: {{action}}",
            
            // MCP 类
            Self::McpDefault | Self::McpFile | Self::McpDatabase => "{{input}}",
        }
    }

    /// 获取需要的变量
    pub fn required_vars(&self) -> Vec<&'static str> {
        match self {
            Self::ToolSummarize => vec!["content", "requirements"],
            Self::ToolTranslate => vec!["content", "from_lang", "to_lang"],
            Self::ToolCode => vec!["task"],
            Self::ToolFile => vec!["path", "action"],
            _ => vec![],
        }
    }
}

/// Prompt 来源
#[derive(Debug, Clone)]
pub enum PromptSource {
    /// 内置
    BuiltIn(BuiltInPrompt),
    /// 文件路径
    File(PathBuf),
    /// 内联
    Inline { system: String, user: String },
}

impl Default for PromptSource {
    fn default() -> Self {
        Self::BuiltIn(BuiltInPrompt::default())
    }
}
```

---

## 2. Prompt 管理器

```rust
/// Prompt 管理器
pub struct PromptManager {
    /// 内置 prompt（静态）
    built_in: BuiltInPrompt,
    /// 自定义文件路径
    custom_path: Option<PathBuf>,
    /// 内联内容
    inline: Option<(String, String)>,
}

impl PromptManager {
    /// 使用内置
    pub fn built_in(prompt: BuiltInPrompt) -> Self {
        Self {
            built_in: prompt,
            custom_path: None,
            inline: None,
        }
    }

    /// 使用自定义文件
    pub fn file(path: impl Into<PathBuf>) -> Self {
        Self {
            built_in: BuiltInPrompt::default(),
            custom_path: Some(path.into()),
            inline: None,
        }
    }

    /// 使用内联
    pub fn inline(system: &str, user: &str) -> Self {
        Self {
            built_in: BuiltInPrompt::default(),
            custom_path: None,
            inline: Some((system.to_string(), user.to_string())),
        }
    }

    /// 获取 system prompt
    pub fn system(&self) -> String {
        if let Some((system, _)) = &self.inline {
            return system.clone();
        }
        if let Some(path) = &self.custom_path {
            // 从文件加载
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(config) = toml::from_str::<CustomPromptConfig>(&content) {
                    return config.system.content;
                }
            }
        }
        self.built_in.system_prompt().to_string()
    }

    /// 获取并渲染 user prompt
    pub fn render_user(&self, vars: &std::collections::HashMap<String, String>) -> String {
        let template = if let Some((_, user)) = &self.inline {
            user.clone()
        } else if let Some(path) = &self.custom_path {
            // 从文件加载
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(config) = toml::from_str::<CustomPromptConfig>(&content) {
                    return render_template(&config.user.template, vars);
                }
            }
            String::new()
        } else {
            self.built_in.user_template().to_string()
        };
        
        render_template(&template, vars)
    }
}

/// 自定义 Prompt 配置 (TOML)
#[derive(Debug, Deserialize)]
struct CustomPromptConfig {
    #[serde(rename = "system")]
    system: SystemConfig,
    #[serde(rename = "user")]
    user: UserConfig,
}

#[derive(Debug, Deserialize)]
struct SystemConfig {
    content: String,
}

#[derive(Debug, Deserialize)]
struct UserConfig {
    template: String,
}

/// 简单模板渲染
fn render_template(template: &str, vars: &std::collections::HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (k, v) in vars {
        result = result.replace(&format!("{{{{{}}}}}", k), v);
    }
    result
}
```

---

## 3. 使用示例

### Agent 配置

```rust
use rucora::agent::{ToolAgent, ToolAgentBuilder};
use rucora_core::prompt::{BuiltInPrompt, PromptManager};

// 使用内置 prompt
let agent = ToolAgent::builder()
    .prompt(PromptManager::built_in(BuiltInPrompt::AgentTool))
    .build();

// 使用自定义文件
let agent = ToolAgent::builder()
    .prompt(PromptManager::file("config/my_agent_prompt.toml"))
    .build();
```

### 工具配置

```rust
use rucora_tools::{SearchTool, SummarizeTool};
use rucora_core::prompt::BuiltInPrompt;

// 搜索工具使用专门的 prompt
let search_tool = SearchTool::new()
    .with_prompt(BuiltInPrompt::ToolSearch);

// 总结工具使用专门的 prompt  
let summarize_tool = SummarizeTool::new()
    .with_prompt(BuiltInPrompt::ToolSummarize);
```

### MCP 配置

```rust
use rucora_core::prompt::{BuiltInPrompt, PromptManager};

let mcp_config = McpServerConfig::new()
    .tool_prompt(PromptManager::built_in(BuiltInPrompt::McpDefault))
    .file_prompt(PromptManager::built_in(BuiltInPrompt::McpFile));
```

---

## 4. 自定义 Prompt 文件示例

### 搜索工具自定义

```toml
# config/prompts/tool_search_custom.toml

[system]
content = """
你是一个专业的搜索引擎优化专家。
请生成能获得高质量搜索结果的查询：
- 使用精准的关键词
- 考虑搜索引擎算法
- 添加合适的限定词
"""

[user]
template = """
请为以下查询生成优化的搜索词：

原始查询: {{query}}

目标: {{goal}}

请生成3-5个优化后的搜索词或短语。
"""
```

### 翻译工具自定义

```toml
# config/prompts/tool_translate_cn_to_en.toml

[system]
content = """
你是一个专业翻译师。
请准确、自然地翻译以下内容。
注意：
- 保持原文的语气和风格
- 术语翻译要符合目标语言习惯
- 必要时添加注释说明
"""

[user]
template = """
翻译以下{{from_lang}}内容到{{to_lang}}：

{{content}}

---
附加要求：
{{requirements}}
"""
```

---

## 5. 完整内置 Prompt 清单

| 类别 | 名称 | 用途 |
|------|------|------|
| **Research** | research_default | 默认研究 |
| | research_concise | 简洁研究 |
| | research_detailed | 深度研究 |
| | research_academic | 学术研究 |
| | research_agentic | 自主研究 |
| **Agent** | agent_simple | 简单问答 |
| | agent_chat | 对话 |
| | agent_tool | 工具调用 |
| | agent_react | ReAct 推理 |
| | agent_reflect | 反思 |
| **Tool** | tool_search | 搜索 |
| | tool_browse | 浏览 |
| | tool_summarize | 总结 |
| | tool_translate | 翻译 |
| | tool_code | 代码 |
| | tool_file | 文件 |
| **MCP** | mcp_default | MCP 通用 |
| | mcp_file | MCP 文件 |
| | mcp_database | MCP 数据库 |

---

## 6. API 一览

```rust
// 方式1: 使用内置
PromptManager::built_in(BuiltInPrompt::AgentTool)

// 方式2: 自定义文件
PromptManager::file("config/my_prompt.toml")

// 方式3: 内联
PromptManager::inline("system prompt", "user template")

// 在各种地方使用
ToolAgent::builder().prompt(prompt_manager).build()
SearchTool::new().with_prompt(BuiltInPrompt::ToolSearch)
McpServer::new().default_prompt(PromptManager::built_in(BuiltInPrompt::McpDefault))
```

这个设计覆盖了所有场景，用户可以根据需求选择或自定义。