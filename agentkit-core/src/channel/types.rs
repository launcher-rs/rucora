//! Channel（通信渠道）相关的类型定义。

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    provider::types::ChatMessage,
    tool::types::{ToolCall, ToolResult},
};

/// Token 流式输出事件。
///
/// 说明：
/// - 用于流式 provider 输出 delta/token。
/// - 这里不强行绑定某个 provider 的 chunk 格式，只保留最常用字段。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenDeltaEvent {
    /// 增量文本（delta）。
    pub delta: String,
}

/// Skill 执行事件。
///
/// 说明：runtime 层不一定直接执行 skills（有的项目会在 tool 层包装 skill），
/// 但统一事件模型仍预留该类型，便于 GUI/CLI 做统一可视化。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillEvent {
    /// skill 名称。
    pub name: String,
    /// 阶段：start/end/error 等。
    pub phase: String,
    /// 可选附加数据（入参/出参/耗时/错误详情等）。
    #[serde(default)]
    pub data: Option<Value>,
}

/// Memory 相关事件（写入/检索/命中等）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryEvent {
    /// 阶段：add/query/hit/error 等。
    pub phase: String,
    /// 可选附加数据。
    #[serde(default)]
    pub data: Option<Value>,
}

/// 调试事件：用于输出 runtime 内部状态（可被 trace/replay 记录）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebugEvent {
    /// 调试消息。
    pub message: String,
    /// 可选结构化字段。
    #[serde(default)]
    pub data: Option<Value>,
}

/// 错误事件：用于统一输出 provider/tool/runtime/skill/memory 等错误。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorEvent {
    /// 错误来源分类（provider/tool/runtime/skill/memory/...）。
    pub kind: String,
    /// 人类可读错误信息。
    pub message: String,
    /// 可选结构化字段。
    #[serde(default)]
    pub data: Option<Value>,
}

/// Channel 层传递的事件类型。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChannelEvent {
    /// 对话消息事件。
    Message(ChatMessage),

    /// Token 流式输出事件（delta）。
    TokenDelta(TokenDeltaEvent),

    /// 工具调用事件。
    ToolCall(ToolCall),
    /// 工具结果事件。
    ToolResult(ToolResult),

    /// Skill 相关事件。
    Skill(SkillEvent),

    /// Memory 相关事件。
    Memory(MemoryEvent),

    /// 调试事件。
    Debug(DebugEvent),

    /// 错误事件。
    Error(ErrorEvent),

    /// 原始事件（用于透传实现层自定义结构）。
    Raw(Value),
}
