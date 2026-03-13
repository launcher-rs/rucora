//! Tool（工具）相关的类型定义。

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 工具定义（用于注册到 provider 的 function-calling / tool-calling 机制）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// 工具名称。
    pub name: String,
    /// 工具描述（可选）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 工具输入参数 schema（通常为 JSON Schema 兼容结构）。
    pub input_schema: Value,
}

/// 一次工具调用（由模型产生，供 runtime 执行）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    /// 工具调用 id，用于把调用与结果关联起来。
    pub id: String,
    /// 工具名称。
    pub name: String,
    /// 工具输入参数。
    pub input: Value,
}

/// 工具调用结果（由 runtime/tool 返回，供模型继续推理）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResult {
    /// 对应的工具调用 id。
    pub tool_call_id: String,
    /// 工具输出。
    pub output: Value,
}
