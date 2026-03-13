//! Channel（通信渠道）相关的类型定义。

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    provider::types::ChatMessage,
    tool::types::{ToolCall, ToolResult},
};

/// Channel 层传递的事件类型。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChannelEvent {
    /// 对话消息事件。
    Message(ChatMessage),
    /// 工具调用事件。
    ToolCall(ToolCall),
    /// 工具结果事件。
    ToolResult(ToolResult),
    /// 原始事件（用于透传实现层自定义结构）。
    Raw(Value),
}
