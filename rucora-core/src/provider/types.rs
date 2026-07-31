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

/// 消息内容类型，枚举所有合法的消息内容形态。
///
/// # 类型安全
///
/// 不同角色仅支持合法的内容形态：
/// - `System`/`User`：仅 `Text`
/// - `Assistant`：`Text` 或 `ToolCalls`
/// - `Tool`：仅 `ToolResult`
///
/// # 示例
///
/// ```rust
/// use rucora_core::provider::types::MessageContent;
///
/// let text = MessageContent::Text("你好".to_string());
/// assert_eq!(text.as_text(), Some("你好"));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum MessageContent {
    /// 纯文本内容（用于 system、user、assistant 纯文本消息）。
    Text(String),
    /// 工具调用块（仅 assistant 角色），附带助手文本内容。
    ToolCalls {
        /// 助手文本内容（可能为空）。
        text: String,
        /// 工具调用列表。
        calls: Vec<ToolCall>,
    },
    /// 工具执行结果（仅 tool 角色）。
    ToolResult {
        /// 工具名称。
        name: String,
        /// 对应的工具调用 ID。
        tool_call_id: String,
        /// 工具输出内容（JSON 字符串化后的结果）。
        content: String,
    },
}

/// 自定义反序列化辅助：根据 `type` 字段或字段存在性识别变体。
impl<'de> Deserialize<'de> for MessageContent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::String(s) => Ok(MessageContent::Text(s)),
            serde_json::Value::Object(map) => {
                // 优先使用显式 type 判别字段
                if let Some(type_name) = map.get("type").and_then(|v| v.as_str()) {
                    return match type_name {
                        "tool_calls" => {
                            let text = map
                                .get("text")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string();
                            let calls = map
                                .get("calls")
                                .cloned()
                                .unwrap_or(serde_json::Value::Array(Vec::new()));
                            let calls = serde_json::from_value(calls)
                                .map_err(serde::de::Error::custom)?;
                            Ok(MessageContent::ToolCalls { text, calls })
                        }
                        "tool_result" => {
                            let name = map
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string();
                            let tool_call_id = map
                                .get("tool_call_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string();
                            let content = map
                                .get("content")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string();
                            Ok(MessageContent::ToolResult {
                                name,
                                tool_call_id,
                                content,
                            })
                        }
                        other => Err(serde::de::Error::custom(format!(
                            "未知的消息内容类型：{other}"
                        ))),
                    };
                }

                // 向后兼容：无 type 字段时根据字段存在性识别
                if map.contains_key("calls") {
                    let text = map
                        .get("text")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let calls = map
                        .get("calls")
                        .cloned()
                        .unwrap_or(serde_json::Value::Array(Vec::new()));
                    let calls =
                        serde_json::from_value(calls).map_err(serde::de::Error::custom)?;
                    Ok(MessageContent::ToolCalls { text, calls })
                } else if map.contains_key("tool_call_id") {
                    let name = map
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let tool_call_id = map
                        .get("tool_call_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let content = map
                        .get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    Ok(MessageContent::ToolResult {
                        name,
                        tool_call_id,
                        content,
                    })
                } else {
                    Err(serde::de::Error::custom("无法识别的消息内容格式"))
                }
            }
            _ => Err(serde::de::Error::custom("消息内容必须是字符串或对象")),
        }
    }
}

impl Serialize for MessageContent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serde_json::Map::new();
        match self {
            MessageContent::Text(t) => return serializer.serialize_str(t),
            MessageContent::ToolCalls { text, calls } => {
                map.insert("type".to_string(), serde_json::Value::String("tool_calls".into()));
                map.insert("text".to_string(), serde_json::Value::String(text.clone()));
                map.insert(
                    "calls".to_string(),
                    serde_json::to_value(calls).map_err(serde::ser::Error::custom)?,
                );
            }
            MessageContent::ToolResult {
                name,
                tool_call_id,
                content,
            } => {
                map.insert("type".to_string(), serde_json::Value::String("tool_result".into()));
                map.insert("name".to_string(), serde_json::Value::String(name.clone()));
                map.insert(
                    "tool_call_id".to_string(),
                    serde_json::Value::String(tool_call_id.clone()),
                );
                map.insert("content".to_string(), serde_json::Value::String(content.clone()));
            }
        }
        serde_json::Value::Object(map)
            .serialize(serializer)
    }
}

impl MessageContent {
    /// 返回文本内容（仅对 `Text` 变体有意义）。
    pub fn as_text(&self) -> Option<&str> {
        match self {
            MessageContent::Text(t) => Some(t.as_str()),
            _ => None,
        }
    }

    /// 返回工具调用列表。
    pub fn as_tool_calls(&self) -> Option<&[ToolCall]> {
        match self {
            MessageContent::ToolCalls { calls, .. } => Some(calls.as_slice()),
            _ => None,
        }
    }

    /// 返回工具结果信息（仅对 `ToolResult` 变体有意义）。
    pub fn as_tool_result(&self) -> Option<(&str, &str, &str)> {
        match self {
            MessageContent::ToolResult {
                name,
                tool_call_id,
                content,
            } => Some((name, tool_call_id, content)),
            _ => None,
        }
    }

    /// 提取文本内容（无论变体，尝试获取可展示的文本）。
    /// `Text` 返回自身，`ToolCalls` 返回其中的 text，`ToolResult` 返回 content。
    pub fn to_display(&self) -> &str {
        match self {
            MessageContent::Text(t) => t.as_str(),
            MessageContent::ToolCalls { text, .. } => text.as_str(),
            MessageContent::ToolResult { content, .. } => content.as_str(),
        }
    }
}

impl From<String> for MessageContent {
    fn from(s: String) -> Self {
        MessageContent::Text(s)
    }
}

impl From<&str> for MessageContent {
    fn from(s: &str) -> Self {
        MessageContent::Text(s.to_string())
    }
}

impl Default for MessageContent {
    fn default() -> Self {
        MessageContent::Text(String::new())
    }
}

impl std::fmt::Display for MessageContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageContent::Text(t) => write!(f, "{t}"),
            MessageContent::ToolCalls { text, calls } => {
                if !text.is_empty() {
                    writeln!(f, "{text}")?;
                }
                write!(f, "{}", serde_json::to_string(calls).unwrap_or_default())
            }
            MessageContent::ToolResult { content, .. } => write!(f, "{content}"),
        }
    }
}

/// 一条对话消息。
///
/// # 类型安全
///
/// 使用 `MessageContent` 确保角色和内容的合法组合。
/// 通过构造器方法而非直接字段赋值来创建消息：
///
/// - [`ChatMessage::system`] -> role=System, content=Text
/// - [`ChatMessage::user`] -> role=User, content=Text
/// - [`ChatMessage::assistant`] -> role=Assistant, content=Text
/// - [`ChatMessage::assistant_with_tool_calls`] -> role=Assistant, content=ToolCalls
/// - [`ChatMessage::tool_result`] -> role=Tool, content=ToolResult
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    /// 角色。
    pub role: Role,
    /// 消息内容（类型安全的 enum）。
    pub content: MessageContent,
    /// 可选的发送者名称（例如 tool 名称或特定 persona）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl ChatMessage {
    /// 创建一条 system 消息。
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: MessageContent::Text(content.into()),
            name: None,
        }
    }

    /// 创建一条 user 消息。
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: MessageContent::Text(content.into()),
            name: None,
        }
    }

    /// 创建一条 assistant 消息。
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: MessageContent::Text(content.into()),
            name: None,
        }
    }

    /// 创建一条携带工具调用的 assistant 消息。
    pub fn assistant_with_tool_calls(
        content: impl Into<String>,
        tool_calls: Vec<ToolCall>,
    ) -> Self {
        Self {
            role: Role::Assistant,
            content: MessageContent::ToolCalls {
                text: content.into(),
                calls: tool_calls,
            },
            name: None,
        }
    }

    /// 创建一条 tool 结果消息。
    pub fn tool_result(
        name: impl Into<String>,
        tool_call_id: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            role: Role::Tool,
            content: MessageContent::ToolResult {
                name: name.into(),
                tool_call_id: tool_call_id.into(),
                content: content.into(),
            },
            name: None,
        }
    }

    /// 获取消息的文本内容。
    ///
    /// 对于 `Text` 和 `ToolResult` 变体返回文本，对于 `ToolCalls` 返回空字符串。
    pub fn content_text(&self) -> &str {
        self.content.to_display()
    }

    /// 获取工具调用列表（仅在 `content` 为 `ToolCalls` 时有效）。
    pub fn tool_calls(&self) -> &[ToolCall] {
        self.content.as_tool_calls().unwrap_or(&[])
    }

    /// 获取工具调用 ID（仅在 `content` 为 `ToolResult` 时有效）。
    pub fn tool_call_id(&self) -> Option<&str> {
        self.content.as_tool_result().map(|(_, id, _)| id)
    }

    /// 获取工具名称（仅在 `content` 为 `ToolResult` 时有效）。
    pub fn tool_name(&self) -> Option<&str> {
        self.content.as_tool_result().map(|(name, _, _)| name)
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
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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
/// ```rust,ignore
/// use rucora_core::provider::types::{LlmParams, ChatRequest, ChatMessage};
///
/// let params = LlmParams::new()
///     .temperature(0.5)
///     .top_p(0.9)
///     .max_tokens(4096);
///
/// let mut request = ChatRequest::new(vec![ChatMessage::user("hi")]);
/// params.apply_to(&mut request);
/// ```
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
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
        assert!(
            (0.0..=2.0).contains(&value),
            "temperature must be between 0.0 and 2.0, got {value}"
        );
        self.temperature = Some(value);
        self
    }

    /// 设置 top_p。
    pub fn top_p(mut self, value: f32) -> Self {
        assert!(
            (0.0..=1.0).contains(&value),
            "top_p must be between 0.0 and 1.0, got {value}"
        );
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
        self.merge_into(&mut request.params);
    }

    /// 将参数合并到 LlmParams 中（仅覆盖非 None 的字段）。
    pub fn merge_into(&self, target: &mut LlmParams) {
        if let Some(v) = self.temperature {
            target.temperature = Some(v);
        }
        if let Some(v) = self.top_p {
            target.top_p = Some(v);
        }
        if let Some(v) = self.top_k {
            target.top_k = Some(v);
        }
        if let Some(v) = self.max_tokens {
            target.max_tokens = Some(v);
        }
        if let Some(v) = self.frequency_penalty {
            target.frequency_penalty = Some(v);
        }
        if let Some(v) = self.presence_penalty {
            target.presence_penalty = Some(v);
        }
        if let Some(ref v) = self.stop {
            target.stop = Some(v.clone());
        }
        if let Some(ref v) = self.response_format {
            target.response_format = Some(v.clone());
        }
        if let Some(ref v) = self.extra {
            target.extra = Some(v.clone());
        }
    }

    /// 从 ChatRequest 中提取参数。
    pub fn from_request(request: &ChatRequest) -> Self {
        request.params.clone()
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
    /// LLM 参数（temperature、top_p、max_tokens 等）。
    ///
    /// 使用 `#[serde(flatten)]` 展平，序列化格式与独立字段一致。
    #[serde(flatten)]
    pub params: LlmParams,
    /// 透传元数据（便于实现层做 tracing/路由/调试）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

impl std::ops::Deref for ChatRequest {
    type Target = LlmParams;

    fn deref(&self) -> &Self::Target {
        &self.params
    }
}

impl std::ops::DerefMut for ChatRequest {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.params
    }
}

impl ChatRequest {
    /// 通过消息列表创建请求（其余字段默认为 None）。
    pub fn new(messages: Vec<ChatMessage>) -> Self {
        Self {
            messages,
            model: None,
            tools: None,
            params: LlmParams::default(),
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
        assert!(
            (0.0..=2.0).contains(&temperature),
            "temperature must be between 0.0 and 2.0, got {temperature}"
        );
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
///
/// # 工具调用
///
/// 工具调用信息统一在 `message.content` 中承载（`MessageContent::ToolCalls`）。
/// 不再单独保留 `tool_calls` 字段，避免与 `ChatResponse` 存在两份真相。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatResponse {
    /// 模型生成的最终消息。
    ///
    /// - 无工具调用时：`content` 为 `Text`
    /// - 有工具调用时：`content` 为 `ToolCalls`
    pub message: ChatMessage,
    /// token 使用统计（可选）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    /// 结束原因（可选）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,
}

impl ChatResponse {
    /// 获取工具调用列表（从 `message.content` 派生）。
    pub fn tool_calls(&self) -> &[ToolCall] {
        self.message.tool_calls()
    }

    /// 获取消息文本内容。
    pub fn text(&self) -> &str {
        self.message.content_text()
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_content_text_roundtrip() {
        let content = MessageContent::Text("你好".to_string());
        let json = serde_json::to_string(&content).unwrap();
        assert_eq!(json, r#""你好""#);
        let back: MessageContent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, content);
    }

    #[test]
    fn test_message_content_tool_calls_roundtrip() {
        let content = MessageContent::ToolCalls {
            text: "正在调用工具".to_string(),
            calls: vec![ToolCall {
                id: "call_1".to_string(),
                name: "calculator".to_string(),
                input: serde_json::json!({"expr": "1+1"}),
            }],
        };
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains(r#""type":"tool_calls""#));
        let back: MessageContent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, content);
    }

    #[test]
    fn test_message_content_tool_result_roundtrip() {
        let content = MessageContent::ToolResult {
            name: "calculator".to_string(),
            tool_call_id: "call_1".to_string(),
            content: "2".to_string(),
        };
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains(r#""type":"tool_result""#));
        let back: MessageContent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, content);
    }

    #[test]
    fn test_message_content_backward_compat_no_type_field() {
        // 兼容旧格式：没有 type 字段时通过字段存在性识别
        let tool_calls = r#"{"text":"hi","calls":[]}"#;
        let content: MessageContent = serde_json::from_str(tool_calls).unwrap();
        assert!(matches!(content, MessageContent::ToolCalls { .. }));

        let tool_result = r#"{"name":"x","tool_call_id":"c1","content":"out"}"#;
        let content: MessageContent = serde_json::from_str(tool_result).unwrap();
        assert!(matches!(content, MessageContent::ToolResult { .. }));
    }
}
