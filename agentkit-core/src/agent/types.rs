//! Agent 相关的类型定义。

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    provider::types::ChatMessage,
    tool::types::ToolResult,
};

/// Agent 输入。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentInput {
    /// 输入消息（通常为对话历史或任务描述）。
    pub messages: Vec<ChatMessage>,
    /// 透传元数据。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// Agent 输出。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentOutput {
    /// 最终输出消息。
    pub message: ChatMessage,
    /// 本次执行过程中产生的工具结果（如果 runtime 支持工具）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_results: Vec<ToolResult>,
}
