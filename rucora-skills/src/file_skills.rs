//! 文件操作技能实现。
//!
//! 本模块提供文件操作相关的技能实现，如读取本地文件等。

use rucora_core::{
    error::SkillError,
    skill::types::{SkillContext, SkillOutput},
    tool::ToolCategory,
};
use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::fs;

pub use rucora_core::skill::Skill;

/// 读取本地文件的技能。
///
/// 该技能允许读取指定路径的本地文件内容，并支持限制最大读取字节数。
pub struct FileReadSkill {
    pub name: String,
    pub description: Option<String>,
    pub default_max_bytes: usize,
}

impl FileReadSkill {
    /// 创建一个读取本地文件的 skill。
    ///
    /// 默认：
    /// - name: `file_read_skill`（直接作为 tool name）
    /// - default_max_bytes: 200_000（用于截断输出，避免一次读太大）
    pub fn new() -> Self {
        Self {
            name: "file_read_skill".to_string(),
            description: Some("读取本地文件内容".to_string()),
            default_max_bytes: 200_000,
        }
    }

    /// 获取技能名称。
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取技能描述。
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// 运行文件读取技能。
    ///
    /// 输入参数：
    /// - path: 必填，本地文件路径
    /// - max_bytes: 可选，最多读取字符数（用于截断）
    pub async fn run(&self, ctx: SkillContext) -> Result<SkillOutput, SkillError> {
        let path = ctx
            .input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SkillError::Message("缺少必需的 'path' 字段".to_string()))?
            .to_string();

        let max_bytes = ctx
            .input
            .get("max_bytes")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(self.default_max_bytes);

        let content = fs::read_to_string(&path)
            .await
            .map_err(|e| SkillError::Message(format!("读取文件失败：{}", e)))?;

        let truncated = content.len() > max_bytes;
        let out_content = if truncated {
            content.chars().take(max_bytes).collect::<String>()
        } else {
            content
        };

        Ok(SkillOutput {
            output: json!({
                "path": path,
                "content": out_content,
                "truncated": truncated,
                "max_bytes": max_bytes,
            }),
        })
    }
}

#[async_trait]
impl Skill for FileReadSkill {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::File]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "本地文件路径"},
                "max_bytes": {"type": "integer", "description": "可选，最多读取字符数（用于截断）"}
            },
            "required": ["path"]
        })
    }

    async fn run_value(&self, input: Value) -> Result<Value, SkillError> {
        let out = self.run(SkillContext { input }).await?;
        Ok(out.output)
    }
}



