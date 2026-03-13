//! Provider（LLM 提供者）相关的类型定义。

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::tool::types::{ToolCall, ToolDefinition};

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

impl ChatMessage {
    /// 创建一条 system 消息。
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
            name: None,
        }
    }

    /// 创建一条 user 消息。
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            name: None,
        }
    }

    /// 创建一条 assistant 消息。
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            name: None,
        }
    }

    /// 创建一条 tool 消息（name 通常用于承载 tool 名称）。
    pub fn tool(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: content.into(),
            name: Some(name.into()),
        }
    }
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

impl ChatRequest {
    /// 通过消息列表创建请求（其余字段默认为 None）。
    pub fn new(messages: Vec<ChatMessage>) -> Self {
        Self {
            messages,
            model: None,
            tools: None,
            temperature: None,
            max_tokens: None,
            metadata: None,
        }
    }

    /// 快速创建一个“单条 user 文本输入”的请求。
    pub fn from_user_text(text: impl Into<String>) -> Self {
        Self::new(vec![ChatMessage::user(text)])
    }

    /// 设置 model。
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// 设置 tools。
    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// 设置 temperature。
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// 设置 max_tokens。
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// 设置 metadata。
    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// 在对话最前面插入 system prompt。
    pub fn with_system_prompt(mut self, system_prompt: impl Into<String>) -> Self {
        self.messages.insert(0, ChatMessage::system(system_prompt));
        self
    }

    /// 追加一条消息。
    pub fn push_message(mut self, message: ChatMessage) -> Self {
        self.messages.push(message);
        self
    }
}

impl From<Vec<ChatMessage>> for ChatRequest {
    fn from(value: Vec<ChatMessage>) -> Self {
        Self::new(value)
    }
}

impl From<ChatMessage> for ChatRequest {
    fn from(value: ChatMessage) -> Self {
        Self::new(vec![value])
    }
}

impl From<String> for ChatRequest {
    fn from(value: String) -> Self {
        Self::from_user_text(value)
    }
}

impl From<&str> for ChatRequest {
    fn from(value: &str) -> Self {
        Self::from_user_text(value)
    }
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
