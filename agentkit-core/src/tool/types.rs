//! Tool（工具）相关的类型定义
//!
//! # 概述
//!
//! 本模块定义了工具相关的核心类型：
//! - [`ToolDefinition`]: 工具定义，用于注册到 LLM
//! - [`ToolCall`]: 工具调用，由 LLM 生成
//! - [`ToolResult`]: 工具结果，返回给 LLM
//!
//! # 工具调用流程
//!
//! ```text
//! 1. 注册工具定义到 LLM
//!    ToolDefinition { name, description, input_schema }
//!
//! 2. LLM 决定调用工具
//!    ▼
//! 3. 生成 ToolCall
//!    ToolCall { id, name, input }
//!
//! 4. Runtime 执行工具
//!    ▼
//! 5. 返回 ToolResult
//!    ToolResult { tool_call_id, output }
//!
//! 6. 将结果返回给 LLM 继续推理
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 默认的工具输出最大字节数
///
/// 用于 runtime 统一截断与标记，防止工具输出过大导致上下文溢出。
///
/// # 说明
///
/// - 默认值：64 KB (64 * 1024 bytes)
/// - 超过此限制的输出会被截断
/// - 截断后会添加标记说明
///
/// # 示例
///
/// ```rust
/// use agentkit_core::tool::types::DEFAULT_TOOL_OUTPUT_MAX_BYTES;
///
/// assert_eq!(DEFAULT_TOOL_OUTPUT_MAX_BYTES, 64 * 1024);
/// ```
pub const DEFAULT_TOOL_OUTPUT_MAX_BYTES: usize = 64 * 1024;

/// 工具定义
///
/// 用于注册到 provider 的 function-calling / tool-calling 机制。
///
/// # 字段说明
///
/// - `name`: 工具名称（必须唯一，用于 LLM 识别和调用）
/// - `description`: 工具描述（帮助 LLM 理解工具用途）
/// - `input_schema`: 输入参数的 JSON Schema（定义工具接受的参数）
///
/// # 示例
///
/// ```rust
/// use agentkit_core::tool::types::ToolDefinition;
/// use serde_json::json;
///
/// let def = ToolDefinition {
///     name: "file_read".to_string(),
///     description: Some("读取文件内容".to_string()),
///     input_schema: json!({
///         "type": "object",
///         "properties": {
///             "path": {
///                 "type": "string",
///                 "description": "文件路径"
///             }
///         },
///         "required": ["path"]
///     }),
/// };
///
/// assert_eq!(def.name, "file_read");
/// ```
///
/// # 与 LLM 集成
///
/// 这个类型会被转换为 LLM provider 的工具定义格式：
///
/// ## OpenAI 格式
///
/// ```json
/// {
///   "type": "function",
///   "function": {
///     "name": "file_read",
///     "description": "读取文件内容",
///     "parameters": {
///       "type": "object",
///       "properties": {
///         "path": {"type": "string"}
///       }
///     }
///   }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// 工具名称
    ///
    /// 必须唯一，用于 LLM 识别和调用工具。
    /// 命名约定：
    /// - 使用小写字母和下划线
    /// - 具有描述性
    /// - 避免与其他工具冲突
    pub name: String,

    /// 工具描述（可选）
    ///
    /// 帮助 LLM 理解工具的用途和使用场景。
    /// 描述应该简洁明了，说明工具的功能。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// 工具输入参数 schema
    ///
    /// 通常为 JSON Schema 兼容结构，定义工具接受的参数。
    /// LLM 会根据这个 schema 生成正确的工具调用参数。
    pub input_schema: Value,
}

/// 工具调用
///
/// 由模型产生，供 runtime 执行。
///
/// # 字段说明
///
/// - `id`: 工具调用 ID，用于把调用与结果关联起来
/// - `name`: 工具名称（必须与注册的 `ToolDefinition.name` 匹配）
/// - `input`: 工具输入参数（应符合 `ToolDefinition.input_schema`）
///
/// # 示例
///
/// ```rust
/// use agentkit_core::tool::types::ToolCall;
/// use serde_json::json;
///
/// let call = ToolCall {
///     id: "call_abc123".to_string(),
///     name: "file_read".to_string(),
///     input: json!({
///         "path": "/path/to/file.txt"
///     }),
/// };
///
/// assert_eq!(call.id, "call_abc123");
/// assert_eq!(call.name, "file_read");
/// ```
///
/// # LLM 生成示例
///
/// 当 LLM 决定调用工具时，会生成类似以下的响应：
///
/// ```json
/// {
///   "role": "assistant",
///   "content": null,
///   "tool_calls": [
///     {
///       "id": "call_abc123",
///       "type": "function",
///       "function": {
///         "name": "file_read",
///         "arguments": "{\"path\": \"/path/to/file.txt\"}"
///       }
///     }
///   ]
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    /// 工具调用 ID
    ///
    /// 用于把调用与结果关联起来。
    /// 通常由 LLM provider 生成，确保唯一性。
    pub id: String,

    /// 工具名称
    ///
    /// 必须与注册的 `ToolDefinition.name` 匹配。
    pub name: String,

    /// 工具输入参数
    ///
    /// 应符合 `ToolDefinition.input_schema` 定义的 schema。
    /// 通常是一个 JSON object。
    pub input: Value,
}

/// 工具调用结果
///
/// 由 runtime/tool 返回，供模型继续推理。
///
/// # 字段说明
///
/// - `tool_call_id`: 对应的工具调用 ID（与 `ToolCall.id` 匹配）
/// - `output`: 工具输出（通常是 JSON object）
///
/// # 示例
///
/// ```rust
/// use agentkit_core::tool::types::ToolResult;
/// use serde_json::json;
///
/// let result = ToolResult {
///     tool_call_id: "call_abc123".to_string(),
///     output: json!({
///         "content": "文件内容...",
///         "size": 1024
///     }),
/// };
///
/// assert_eq!(result.tool_call_id, "call_abc123");
/// ```
///
/// # 返回给 LLM 的格式
///
/// 工具结果会被转换为 LLM 可理解的消息格式：
///
/// ```json
/// {
///   "role": "tool",
///   "tool_call_id": "call_abc123",
///   "content": "{\"content\": \"文件内容...\", \"size\": 1024}"
/// }
/// ```
///
/// # 输出限制
///
/// 工具输出可能很大，需要注意：
///
/// - 使用 [`DEFAULT_TOOL_OUTPUT_MAX_BYTES`] 限制输出大小
/// - 超过限制时进行截断并添加标记
/// - 避免返回过大的输出导致上下文溢出
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResult {
    /// 对应的工具调用 ID
    ///
    /// 必须与 `ToolCall.id` 匹配，用于关联调用与结果。
    pub tool_call_id: String,

    /// 工具输出
    ///
    /// 通常是 JSON object，包含工具执行的结果。
    /// 输出应该：
    /// - 结构化（便于 LLM 理解）
    /// - 简洁（避免过大）
    /// - 包含必要信息（成功/失败、结果数据、错误信息等）
    pub output: Value,
}

use crate::Tool;
use crate::error::ToolError;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// 工具调用上下文
///
/// 在 `Tool::call` 时由执行器注入，工具可从中获取运行时信息。
///
/// # 示例
///
/// ```rust
/// use agentkit_core::tool::types::ToolContext;
///
/// let ctx = ToolContext::new()
///     .with("working_dir", "/tmp/workspace")
///     .with("session_id", "sess_abc123");
///
/// let dir = ctx.get("working_dir"); // Some("/tmp/workspace")
/// ```
#[derive(Debug, Clone, Default)]
pub struct ToolContext {
    /// 键值对上下文数据
    data: HashMap<String, String>,
}

impl ToolContext {
    /// 创建空上下文。
    pub fn new() -> Self {
        Self::default()
    }

    /// 插入一个键值对（builder 风格）。
    pub fn with(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.data.insert(key.into(), value.into());
        self
    }

    /// 插入一个键值对（可变引用风格）
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.data.insert(key.into(), value.into());
    }

    /// 获取值
    pub fn get(&self, key: &str) -> Option<&str> {
        self.data.get(key).map(|s| s.as_str())
    }

    /// 获取工作目录（常用快捷方法）
    pub fn working_dir(&self) -> Option<&str> {
        self.get("working_dir")
    }

    /// 获取会话 ID（常用快捷方法）
    pub fn session_id(&self) -> Option<&str> {
        self.get("session_id")
    }
}

/// ToolRegistry trait - 工具注册表接口。
///
/// 用于管理和调用多个工具。Agent 和 Runtime 通过此接口调用工具。
#[async_trait]
pub trait ToolRegistry: Send + Sync {
    /// 获取指定名称的工具。
    fn get_tool(&self, name: &str) -> Option<Arc<dyn Tool>>;

    /// 列出所有可用的工具。
    fn list_tools(&self) -> Vec<Arc<dyn Tool>>;

    /// 调用指定工具。
    ///
    /// # 参数
    ///
    /// * `name` - 工具名称
    /// * `input` - 工具输入参数
    ///
    /// # 返回
    ///
    /// 返回工具执行结果
    async fn call(&self, name: &str, input: Value) -> Result<Value, ToolError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_definition_serialization() {
        let def = ToolDefinition {
            name: "test_tool".to_string(),
            description: Some("测试工具".to_string()),
            input_schema: json!({"type": "object"}),
        };

        let serialized = serde_json::to_string(&def).unwrap();
        let deserialized: ToolDefinition = serde_json::from_str(&serialized).unwrap();

        assert_eq!(def, deserialized);
    }

    #[test]
    fn test_tool_call_serialization() {
        let call = ToolCall {
            id: "call_123".to_string(),
            name: "test_tool".to_string(),
            input: json!({"param": "value"}),
        };

        let serialized = serde_json::to_string(&call).unwrap();
        let deserialized: ToolCall = serde_json::from_str(&serialized).unwrap();

        assert_eq!(call, deserialized);
    }

    #[test]
    fn test_tool_result_serialization() {
        let result = ToolResult {
            tool_call_id: "call_123".to_string(),
            output: json!({"result": "success"}),
        };

        let serialized = serde_json::to_string(&result).unwrap();
        let deserialized: ToolResult = serde_json::from_str(&serialized).unwrap();

        assert_eq!(result, deserialized);
    }
}
