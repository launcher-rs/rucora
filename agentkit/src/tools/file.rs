//! 文件工具模块。
//!
//! 提供文件读写和编辑功能。

use agentkit_core::{
    error::ToolError,
    tool::{Tool, ToolCategory},
};
use async_trait::async_trait;
use serde_json::{Value, json};

/// 文件读取工具：读取文件内容。
///
/// 适用场景：
/// - 读取文件内容
///
/// 输入格式：
/// ```json
/// {
///   "path": "/path/to/file"
/// }
/// ```
pub struct FileReadTool;

impl FileReadTool {
    /// 创建一个新的 FileReadTool 实例。
    pub fn new() -> Self {
        Self
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
        Some("读取文件内容")
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
                    "description": "文件路径"
                }
            },
            "required": ["path"]
        })
    }

    /// 执行工具的核心逻辑。
    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        // 验证输入是否包含必需的字段
        if !input.get("path").is_some() {
            return Err(ToolError::Message("缺少必需的 'path' 字段".to_string()));
        }

        let path = input["path"].as_str().unwrap();

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| ToolError::Message(format!("读取文件失败: {}", e)))?;
        Ok(json!({
            "content": content
        }))
    }
}

/// 文件写入工具：写入文件内容。
///
/// 适用场景：
/// - 写入文件内容
///
/// 输入格式：
/// ```json
/// {
///   "path": "/path/to/file",
///   "content": "文件内容"
/// }
/// ```
pub struct FileWriteTool;

impl FileWriteTool {
    /// 创建一个新的 FileWriteTool 实例。
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileWriteTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FileWriteTool {
    /// 返回工具名称。
    fn name(&self) -> &str {
        "file_write"
    }

    /// 返回工具描述。
    fn description(&self) -> Option<&str> {
        Some("写入文件内容")
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
                    "description": "文件路径"
                },
                "content": {
                    "type": "string",
                    "description": "文件内容"
                }
            },
            "required": ["path", "content"]
        })
    }

    /// 执行工具的核心逻辑。
    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        // 验证输入是否包含必需的字段
        if !input.get("path").is_some() || !input.get("content").is_some() {
            return Err(ToolError::Message(
                "缺少必需的 'path' 或 'content' 字段".to_string(),
            ));
        }

        let path = input["path"].as_str().unwrap();
        let content = input["content"].as_str().unwrap();

        tokio::fs::write(path, content)
            .await
            .map_err(|e| ToolError::Message(format!("写入文件失败: {}", e)))?;
        Ok(json!({}))
    }
}

/// 文件编辑工具：通过精确替换编辑文件。
///
/// 使用 old_string → new_string 的精确替换方式来编辑文件内容。
/// old_string 必须在文件中精确匹配出现一次（0次=未找到，多次=歧义）。
/// new_string 可以为空以删除匹配的文本。
///
/// 输入格式：
/// ```json
/// {
///   "path": "文件路径",
///   "old_string": "要替换的原文",
///   "new_string": "新文本"
/// }
/// ```
pub struct FileEditTool;

impl FileEditTool {
    /// 创建一个新的 FileEditTool 实例。
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileEditTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FileEditTool {
    /// 返回工具名称。
    fn name(&self) -> &str {
        "file_edit"
    }

    /// 返回工具描述。
    fn description(&self) -> Option<&str> {
        Some("通过精确字符串替换编辑文件内容")
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
                    "description": "文件路径"
                },
                "old_string": {
                    "type": "string",
                    "description": "要查找并替换的精确文本（必须在文件中精确出现一次）"
                },
                "new_string": {
                    "type": "string",
                    "description": "替换后的文本（空字符串表示删除）"
                }
            },
            "required": ["path", "old_string", "new_string"]
        })
    }

    /// 执行文件编辑。
    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少必需的 'path' 字段".to_string()))?;

        let old_string = input
            .get("old_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少必需的 'old_string' 字段".to_string()))?;

        let new_string = input
            .get("new_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少必需的 'new_string' 字段".to_string()))?;

        if old_string.is_empty() {
            return Err(ToolError::Message("old_string 不能为空".to_string()));
        }

        // 读取文件内容
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| ToolError::Message(format!("读取文件失败: {}", e)))?;

        // 检查匹配次数
        let matches = content.matches(old_string).count();
        if matches == 0 {
            return Err(ToolError::Message(format!(
                "未找到匹配文本: {}",
                old_string
            )));
        }
        if matches > 1 {
            return Err(ToolError::Message(format!(
                "找到 {} 处匹配，匹配歧义。请提供更精确的唯一匹配文本",
                matches
            )));
        }

        // 执行替换
        let new_content = content.replacen(old_string, new_string, 1);

        // 写回文件
        tokio::fs::write(path, new_content)
            .await
            .map_err(|e| ToolError::Message(format!("写入文件失败: {}", e)))?;

        Ok(json!({
            "success": true,
            "path": path,
            "replacements": 1
        }))
    }
}
