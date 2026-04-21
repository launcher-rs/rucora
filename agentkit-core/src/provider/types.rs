//! Provider（LLM 提供者）相关的类型定义。

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::tool::types::{ToolCall, ToolDefinition};

/// 结构化输出请求。
///
/// 不同 provider 对结构化输出的支持程度不同：
/// - JSON Object：要求输出为合法 JSON
/// - JSON Schema：要求输出满足给定 schema（如果 provider 支持）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseFormat {
    /// 要求模型输出为合法 JSON 对象。
    JsonObject,
    /// 要求模型输出满足 JSON Schema。
    ///
    /// `schema` 为 JSON Schema（建议为 object schema）。
    JsonSchema {
        /// schema 名称（部分 provider 需要）。
        name: String,
        /// JSON Schema 内容。
        schema: Value,
        /// 是否严格模式（如果 provider 支持）。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        strict: Option<bool>,
    },
}

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

/// LLM 请求参数集合。
///
/// 统一管理所有 LLM 采样和生成参数，便于在 Agent 层面配置并传递到 ChatRequest。
/// 所有字段均为 `Option`，`None` 表示使用模型默认值。
///
/// # 使用示例
///
/// ```rust
/// use agentkit_core::provider::types::LlmParams;
///
/// // 创建默认参数
/// let params = LlmParams::default();
///
/// // 使用 builder 方法
/// let params = LlmParams::new()
///     .temperature(0.5)
///     .top_p(0.9)
///     .max_tokens(4096);
///
/// // 应用到 ChatRequest
/// let mut request = ChatRequest::new(vec![]);
/// params.apply_to(&mut request);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LlmParams {
    /// 温度参数（0.0 - 2.0）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top P（核采样参数）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Top K（某些 provider 支持）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    /// 最大输出 token 数。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// 频率惩罚。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    /// 存在惩罚。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    /// Stop 序列。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    /// 结构化输出格式。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    /// 额外参数（provider 特定）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra: Option<Value>,
}

impl LlmParams {
    /// 创建空的参数集合（所有字段为 None，使用模型默认值）。
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置 temperature。
    pub fn temperature(mut self, value: f32) -> Self {
        self.temperature = Some(value);
        self
    }

    /// 设置 top_p。
    pub fn top_p(mut self, value: f32) -> Self {
        self.top_p = Some(value);
        self
    }

    /// 设置 top_k。
    pub fn top_k(mut self, value: u32) -> Self {
        self.top_k = Some(value);
        self
    }

    /// 设置 max_tokens。
    pub fn max_tokens(mut self, value: u32) -> Self {
        self.max_tokens = Some(value);
        self
    }

    /// 设置 frequency_penalty。
    pub fn frequency_penalty(mut self, value: f32) -> Self {
        self.frequency_penalty = Some(value);
        self
    }

    /// 设置 presence_penalty。
    pub fn presence_penalty(mut self, value: f32) -> Self {
        self.presence_penalty = Some(value);
        self
    }

    /// 设置 stop 序列。
    pub fn stop(mut self, value: Vec<String>) -> Self {
        self.stop = Some(value);
        self
    }

    /// 设置 response_format。
    pub fn response_format(mut self, value: ResponseFormat) -> Self {
        self.response_format = Some(value);
        self
    }

    /// 设置 extra 参数。
    pub fn extra(mut self, value: Value) -> Self {
        self.extra = Some(value);
        self
    }

    /// 将参数合并到 ChatRequest 中（仅覆盖非 None 的字段）。
    pub fn apply_to(&self, request: &mut ChatRequest) {
        if let Some(v) = self.temperature {
            request.temperature = Some(v);
        }
        if let Some(v) = self.top_p {
            request.top_p = Some(v);
        }
        if let Some(v) = self.top_k {
            request.top_k = Some(v);
        }
        if let Some(v) = self.max_tokens {
            request.max_tokens = Some(v);
        }
        if let Some(v) = self.frequency_penalty {
            request.frequency_penalty = Some(v);
        }
        if let Some(v) = self.presence_penalty {
            request.presence_penalty = Some(v);
        }
        if let Some(ref v) = self.stop {
            request.stop = Some(v.clone());
        }
        if let Some(ref v) = self.response_format {
            request.response_format = Some(v.clone());
        }
        if let Some(ref v) = self.extra {
            request.extra = Some(v.clone());
        }
    }

    /// 从 ChatRequest 中提取参数。
    pub fn from_request(request: &ChatRequest) -> Self {
        Self {
            temperature: request.temperature,
            top_p: request.top_p,
            top_k: request.top_k,
            max_tokens: request.max_tokens,
            frequency_penalty: request.frequency_penalty,
            presence_penalty: request.presence_penalty,
            stop: request.stop.clone(),
            response_format: request.response_format.clone(),
            extra: request.extra.clone(),
        }
    }
}

impl Default for LlmParams {
    fn default() -> Self {
        Self {
            temperature: None,
            top_p: None,
            top_k: None,
            max_tokens: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            response_format: None,
            extra: None,
        }
    }
}

/// Provider 的对话请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
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

    /// 结构化输出控制（可选）。
    ///
    /// 如果设置，provider 可以尝试让模型输出严格的 JSON 或满足 schema。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    /// 透传元数据（便于实现层做 tracing/路由/调试）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,

    // === 以下为扩展参数，支持更多 provider 特性 ===
    /// Top P（核采样参数，可选）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Top K（可选，某些 provider 支持）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,

    /// Frequency Penalty（频率惩罚，可选）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,

    /// Presence Penalty（存在惩罚，可选）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,

    /// Stop 序列（可选）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,

    /// 额外参数（用于支持 provider 特定的参数，如 NVIDIA 的 reasoning_budget 等）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra: Option<Value>,
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
            response_format: None,
            metadata: None,
            top_p: None,
            top_k: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            extra: None,
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

    /// 设置结构化输出格式。
    pub fn with_response_format(mut self, response_format: ResponseFormat) -> Self {
        self.response_format = Some(response_format);
        self
    }

    /// 设置 metadata。
    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// 设置 top_p。
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// 设置 top_k。
    pub fn with_top_k(mut self, top_k: u32) -> Self {
        self.top_k = Some(top_k);
        self
    }

    /// 设置 frequency_penalty。
    pub fn with_frequency_penalty(mut self, penalty: f32) -> Self {
        self.frequency_penalty = Some(penalty);
        self
    }

    /// 设置 presence_penalty。
    pub fn with_presence_penalty(mut self, penalty: f32) -> Self {
        self.presence_penalty = Some(penalty);
        self
    }

    /// 设置 stop 序列。
    pub fn with_stop(mut self, stop: Vec<String>) -> Self {
        self.stop = Some(stop);
        self
    }

    /// 设置 extra 参数。
    pub fn with_extra(mut self, extra: Value) -> Self {
        self.extra = Some(extra);
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
