//! 文件读取工具
//!
//! 提供安全的文件读取功能，带有扩展名白名单和路径限制

use async_trait::async_trait;
use rucora_core::{
    error::ToolError,
    tool::{Tool, ToolCategory},
};
use serde_json::{Value, json};
use std::path::PathBuf;

use super::FileToolConfig;

/// 文件读取工具：读取文件内容。
///
/// 安全限制：
/// - 仅允许读取白名单扩展名的文件
/// - 禁止访问系统敏感路径
/// - 支持配置允许的工作目录
///
/// 输入格式：
/// ```json
/// {
///   "path": "/path/to/file"
/// }
/// ```
pub struct FileReadTool {
    config: FileToolConfig,
}

impl FileReadTool {
    /// 创建一个新的 FileReadTool 实例。
    pub fn new() -> Self {
        Self {
            config: FileToolConfig::new(),
        }
    }

    /// 设置允许的工作目录
    pub fn with_allowed_dirs(self, dirs: Vec<PathBuf>) -> Self {
        Self {
            config: self.config.with_allowed_dirs(dirs),
        }
    }

    /// 设置最大文件大小
    pub fn with_max_file_size(self, size: u64) -> Self {
        Self {
            config: self.config.with_max_file_size(size),
        }
    }
}

impl Default for FileReadTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FileReadTool {
    /// 返回工具名称。
    fn name(&self) -> &str {
        "file_read"
    }

    /// 返回工具描述。
    fn description(&self) -> Option<&str> {
        Some("读取文件内容（有安全限制：仅允许特定扩展名，禁止系统路径）")
    }

    /// 返回工具分类。
    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::File]
    }

    /// 返回输入参数的 JSON Schema。
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "文件路径（相对路径或绝对路径）"
                }
            },
            "required": ["path"]
        })
    }

    /// 执行工具的核心逻辑。
    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let path_str = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少必需的 'path' 字段".to_string()))?;

        let path = self.config.validate_path_for_read(path_str)?;

        // 检查文件大小
        let metadata = tokio::fs::metadata(&path)
            .await
            .map_err(|e| ToolError::Message(format!("无法获取文件信息：{e}")))?;

        self.config.check_file_size(metadata.len(), "文件")?;

        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| ToolError::Message(format!("读取文件失败：{e}")))?;

        Ok(json!({
            "path": path_str,
            "content": content,
            "size": content.len()
        }))
    }
}
