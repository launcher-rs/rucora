//! agentkit-core 的共享类型定义。
//!
//! 该模块尽量保持“协议层”职责：
//! - Provider/Tool/Agent/Memory/Channel 之间交换的数据结构
//! - 统一的错误类型（仅描述错误，不包含具体实现细节）

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 对话消息角色。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    /// 系统提示词。
    System,
    /// 用户输入。
    User,
    /// 模型/助手输出。
    Assistant,
    /// 工具输出（作为消息的一种角色）。
    Tool,
}

/// 一条对话消息。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    /// 角色。
    pub role: Role,
    /// 文本内容。
    pub content: String,
    /// 可选的发送者名称（例如 tool 名称或特定 persona）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

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

/// 模型消耗统计（token usage）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    /// 提示词 token 数。
    #[serde(default)]
    pub prompt_tokens: u32,
    /// 输出 token 数。
    #[serde(default)]
    pub completion_tokens: u32,
    /// 总 token 数。
    #[serde(default)]
    pub total_tokens: u32,
}

/// 生成结束原因。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FinishReason {
    /// 正常停止。
    Stop,
    /// 达到长度限制。
    Length,
    /// 触发工具调用。
    ToolCall,
    /// 其他原因。
    Other,
}

/// Provider 的对话请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatRequest {
    /// 对话历史。
    pub messages: Vec<ChatMessage>,
    /// 目标模型（可选，具体 provider 可能有默认值）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// 可用工具列表（可选）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    /// 温度参数（可选）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// 最大输出 token（可选）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// 透传元数据（便于实现层做 tracing/路由/调试）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// Provider 的对话响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatResponse {
    /// 模型生成的最终消息。
    pub message: ChatMessage,
    /// 模型请求执行的工具调用列表。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    /// token 使用统计（可选）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    /// 结束原因（可选）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,
}

/// 流式对话的增量 chunk。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatStreamChunk {
    /// 增量文本（如果 provider 以 token/delta 方式返回文本）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta: Option<String>,
    /// 增量工具调用（有些 provider 会在流中逐步返回 tool_call 信息）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    /// token 使用统计（可选）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    /// 结束原因（可选）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,
}

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

/// 一条记忆数据。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryItem {
    /// 记忆 id。
    pub id: String,
    /// 记忆内容。
    pub content: String,
    /// 可选元数据。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// 记忆查询。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryQuery {
    /// 查询文本。
    pub text: String,
    /// 结果数量限制。
    #[serde(default)]
    pub limit: usize,
}

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

/// Provider 错误。
#[derive(thiserror::Error, Debug)]
pub enum ProviderError {
    /// 通用错误信息。
    #[error("provider error: {0}")]
    Message(String),
}

/// Tool 错误。
#[derive(thiserror::Error, Debug)]
pub enum ToolError {
    /// 通用错误信息。
    #[error("tool error: {0}")]
    Message(String),
}

/// Skill 错误。
#[derive(thiserror::Error, Debug)]
pub enum SkillError {
    /// 通用错误信息。
    #[error("skill error: {0}")]
    Message(String),
}

/// Agent 错误。
#[derive(thiserror::Error, Debug)]
pub enum AgentError {
    /// 通用错误信息。
    #[error("agent error: {0}")]
    Message(String),
}

/// Memory 错误。
#[derive(thiserror::Error, Debug)]
pub enum MemoryError {
    /// 通用错误信息。
    #[error("memory error: {0}")]
    Message(String),
}

/// Channel 错误。
#[derive(thiserror::Error, Debug)]
pub enum ChannelError {
    /// 通用错误信息。
    #[error("channel error: {0}")]
    Message(String),
}
