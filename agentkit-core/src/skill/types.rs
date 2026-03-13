//! Skill（技能）相关的类型定义。

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Skill 元信息。
///
/// 说明：
/// - 参考 zeroclaw 的 `SkillMeta`/`SkillManifest` 结构
/// - core 层只描述“技能是什么”，不包含任何 IO（读取文件、执行脚本等）逻辑
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillMeta {
    /// 技能名称（建议唯一）。
    pub name: String,
    /// 技能描述。
    pub description: String,
    /// 版本号（语义化版本号或任意字符串）。
    pub version: String,
    /// 作者（可选）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// 标签。
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
/// 说明：
/// - core 层只描述 tool 的声明信息，不规定执行方式
/// - 执行语义应由 runtime/项目侧自行实现
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillToolDefinition {
    /// 工具名称。
    pub name: String,
    /// 工具描述。
    pub description: String,
    /// 工具类型（例如："shell"、"http"、"script"）。
    pub kind: String,
    /// 命令/URL/脚本入口（具体解释由实现层决定）。
    pub command: String,
    /// 可选参数（键值对）。
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
/// 说明：
/// - 参考 zeroclaw 的 `Skill` 结构
/// - 与 `trait Skill` 区分：这里是“声明/配置”，trait 是“可执行实现”
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillDefinition {
    /// 元信息。
    pub meta: SkillMeta,
    /// 该技能包含的工具声明。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<SkillToolDefinition>,
    /// 该技能附带的 prompts（可作为 system/指令模板等）。
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
/// 为了保持 core 层通用性，这里使用 JSON 作为输入载体。
#[derive(Debug, Clone)]
pub struct SkillContext {
    /// 技能输入参数。
    pub input: Value,
}

/// 技能执行结果。
#[derive(Debug, Clone)]
pub struct SkillOutput {
    /// 技能输出。
    pub output: Value,
}
