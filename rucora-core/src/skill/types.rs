//! Skill（技能）相关的类型定义。
//!
//! 本模块只包含技能相关的数据结构和类型定义，不包含具体实现。
//! 具体的技能实现应该放在 `rucora` crate 的 `skills` 模块中。
//!
//! 设计原则：
//! - 核心层只定义"技能是什么"，不包含任何执行逻辑
//! - 保持类型定义的序列化友好性，便于配置文件和网络传输
//! - 为技能的声明、注册和执行提供统一的数据结构

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Skill 元信息。
///
/// 技能的基本描述信息，用于技能的识别、分类和管理。
///
/// 说明：
/// - 参考 zeroclaw 的 `SkillMeta`/`SkillManifest` 结构
/// - 核心层只描述"技能是什么"，不包含任何 IO（读取文件、执行脚本等）逻辑
/// - 支持序列化，便于从配置文件或网络加载技能定义
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillMeta {
    /// 技能名称（建议唯一）。
    ///
    /// 用于在技能注册表中唯一标识一个技能。
    pub name: String,
    /// 技能描述。
    ///
    /// 详细说明技能的功能、用途和使用方法。
    pub description: String,
    /// 版本号（语义化版本号或任意字符串）。
    ///
    /// 用于技能的版本管理和兼容性控制。
    pub version: String,
    /// 作者（可选）。
    ///
    /// 技能的作者或维护者信息。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// 标签。
    ///
    /// 用于技能的分类、搜索和筛选。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

impl SkillMeta {
    /// 创建 SkillMeta。
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            version: version.into(),
            author: None,
            tags: Vec::new(),
        }
    }

    /// 设置作者。
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// 追加一个标签。
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

/// 由 Skill 定义的工具（可能是 shell/http/script 等）。
///
/// 技能可能需要调用各种工具来完成特定任务。
///
/// 说明：
/// - 核心层只描述 tool 的声明信息，不规定执行方式
/// - 执行语义应由 runtime/项目侧自行实现
/// - 支持多种工具类型，如 shell 命令、HTTP 请求、脚本执行等
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillToolDefinition {
    /// 工具名称。
    ///
    /// 在技能范围内唯一的工具标识符。
    pub name: String,
    /// 工具描述。
    ///
    /// 详细说明工具的功能、用途和参数要求。
    pub description: String,
    /// 工具类型（例如："shell"、"http"、"script"）。
    ///
    /// 决定了工具的执行方式和参数解析方式。
    pub kind: String,
    /// 命令/URL/脚本入口（具体解释由实现层决定）。
    ///
    /// 根据 kind 的不同，这里可能是：
    /// - shell: 要执行的命令
    /// - http: 要请求的 URL
    /// - script: 要执行的脚本路径
    pub command: String,
    /// 可选参数（键值对）。
    ///
    /// 工具执行时需要的额外参数配置。
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub args: std::collections::HashMap<String, String>,
}

impl SkillToolDefinition {
    /// 创建 SkillToolDefinition。
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        kind: impl Into<String>,
        command: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            kind: kind.into(),
            command: command.into(),
            args: std::collections::HashMap::new(),
        }
    }

    /// 追加一个参数。
    pub fn with_arg(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.args.insert(key.into(), value.into());
        self
    }
}

/// 技能定义。
///
/// 完整的技能声明，包含元信息、工具定义和提示词。
/// 这是技能的静态描述，不包含具体的执行逻辑。
///
/// 说明：
/// - 参考 zeroclaw 的 `Skill` 结构
/// - 与具体技能实现区分：这里是"声明/配置"，具体实现在 rucora/skills 中
/// - 支持序列化，便于从配置文件加载技能定义
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillDefinition {
    /// 元信息。
    ///
    /// 技能的基本描述信息，包括名称、描述、版本等。
    pub meta: SkillMeta,
    /// 该技能包含的工具声明。
    ///
    /// 技能执行过程中可能需要调用的工具列表。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<SkillToolDefinition>,
    /// 该技能附带的 prompts（可作为 system/指令模板等）。
    ///
    /// 用于指导 AI 模型如何正确使用这个技能的提示词。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prompts: Vec<String>,
    /// 可选：技能来源路径（例如从本地目录加载）。
    ///
    /// 注意：仅用于调试/追踪，不参与序列化。
    #[serde(skip)]
    pub location: Option<PathBuf>,
}

impl SkillDefinition {
    /// 创建一个空的技能定义。
    pub fn new(meta: SkillMeta) -> Self {
        Self {
            meta,
            tools: Vec::new(),
            prompts: Vec::new(),
            location: None,
        }
    }

    /// 追加一个工具声明。
    pub fn with_tool(mut self, tool: SkillToolDefinition) -> Self {
        self.tools.push(tool);
        self
    }

    /// 追加一个 prompt。
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompts.push(prompt.into());
        self
    }

    /// 设置来源路径。
    pub fn with_location(mut self, location: impl Into<PathBuf>) -> Self {
        self.location = Some(location.into());
        self
    }
}

/// 技能执行上下文。
///
/// 技能执行时需要的输入和环境信息。
///
/// 设计原则：
/// - 为了保持核心层通用性，这里使用 JSON 作为输入载体
/// - 支持灵活的参数传递，可以适应各种技能的输入需求
/// - 结构简洁，便于序列化和反序列化
#[derive(Debug, Clone)]
pub struct SkillContext {
    /// 技能输入参数。
    ///
    /// 包含技能执行所需的所有输入数据，
    /// 具体格式由各个技能实现自行定义和解析。
    pub input: Value,
}

/// 技能执行结果。
///
/// 技能执行完成后的输出数据。
///
/// 设计原则：
/// - 使用 JSON 作为输出载体，保证灵活性和可扩展性
/// - 结构简洁，便于序列化传输和存储
/// - 支持任意格式的输出数据，由具体技能决定
#[derive(Debug, Clone)]
pub struct SkillOutput {
    /// 技能输出。
    ///
    /// 包含技能执行的结果数据，
    /// 具体格式和内容由各个技能实现自行定义。
    pub output: Value,
}

