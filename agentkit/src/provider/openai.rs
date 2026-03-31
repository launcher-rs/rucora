//! OpenAI Provider 实现。
//!
//! 约定：
//! - API Key 从 `OPENAI_API_KEY` 环境变量读取
//! - Base URL 默认 `https://api.openai.com/v1`，也可通过 `OPENAI_BASE_URL` 覆盖
//! - 默认模型优先级：1) 手动设置 `with_default_model()` 2) `OPENAI_DEFAULT_MODEL` 环境变量 3) 内置默认值 `gpt-4o-mini`

use std::env;

use crate::provider::preview;
use agentkit_core::{
    error::ProviderError,
    provider::{
        LlmProvider,
        types::{ChatMessage, ChatRequest, ChatResponse, ChatStreamChunk, FinishReason, ResponseFormat, Role},
    },
    tool::types::{ToolCall, ToolDefinition},
};
use async_trait::async_trait;
use futures_util::{StreamExt, stream::BoxStream};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::{Value, json};
use tracing::debug;

/// OpenAI 默认模型（当未指定时使用）
const OPENAI_DEFAULT_MODEL: &str = "gpt-4o-mini";

/// OpenAI Chat Completions Provider。
///
/// 说明：
/// - 目前仅实现 `chat`（非流式）
/// - `tools` 会按 OpenAI 的 function tools 格式传入
///
/// # 默认模型
///
/// 默认模型的优先级顺序：
/// 1. 手动调用 `with_default_model()` 设置的值
/// 2. `OPENAI_DEFAULT_MODEL` 环境变量
/// 3. 内置默认值 `gpt-4o-mini`
///
/// # 示例
///
/// ```rust,no_run
/// use agentkit::provider::OpenAiProvider;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // 方式 1：使用内置默认模型（gpt-4o-mini）
/// let provider = OpenAiProvider::from_env()?;
///
/// // 方式 2：通过环境变量指定（OPENAI_DEFAULT_MODEL=claude-3-5-sonnet）
/// let provider = OpenAiProvider::from_env()?;
///
/// // 方式 3：手动指定（优先级最高）
/// let provider = OpenAiProvider::from_env()?
///     .with_default_model("gpt-4o");
/// # Ok(())
/// # }
/// ```
pub struct OpenAiProvider {
    client: reqwest::Client,
    base_url: String,
    default_model: String,
}

impl OpenAiProvider {
    /// 从环境变量创建 Provider。
    ///
    /// 默认模型来源（按优先级）：
    /// 1. `OPENAI_DEFAULT_MODEL` 环境变量
    /// 2. 内置默认值 `gpt-4o-mini`
    pub fn from_env() -> Result<Self, ProviderError> {
        let api_key = env::var("OPENAI_API_KEY")
            .map_err(|_| ProviderError::Message("缺少环境变量 OPENAI_API_KEY".to_string()))?;
        let base_url =
            env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let default_model =
            env::var("OPENAI_DEFAULT_MODEL").unwrap_or_else(|_| OPENAI_DEFAULT_MODEL.to_string());

        Ok(Self::with_model(base_url, api_key, default_model))
    }

    /// 创建 Provider（使用内置默认模型 `gpt-4o-mini`）。
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self::with_model(base_url, api_key, OPENAI_DEFAULT_MODEL.to_string())
    }

    /// 创建 Provider（指定默认模型）。
    ///
    /// # 参数
    ///
    /// - `base_url`: API 基础 URL
    /// - `api_key`: API Key
    /// - `default_model`: 默认使用的模型名称
    pub fn with_model(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        default_model: impl Into<String>,
    ) -> Self {
        let api_key = api_key.into();
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Ok(v) = HeaderValue::from_str(&format!("Bearer {}", api_key)) {
            headers.insert(AUTHORIZATION, v);
        }

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .expect("reqwest client build failed");

        Self {
            client,
            base_url: base_url.into(),
            default_model: default_model.into(),
        }
    }

    /// 设置默认模型（覆盖环境变量或内置默认值）。
    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    /// 获取当前配置的默认模型
    pub fn default_model(&self) -> &str {
        &self.default_model
    }

    fn map_role(role: &Role) -> &'static str {
        match role {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::Tool => "tool",
        }
    }

    fn build_messages(messages: &[ChatMessage]) -> Vec<Value> {
        messages
            .iter()
            .map(|m| {
                let mut obj = json!({
                    "role": Self::map_role(&m.role),
                    "content": m.content,
                });
                if let Some(name) = &m.name {
                    if let Some(map) = obj.as_object_mut() {
                        map.insert("name".to_string(), Value::String(name.clone()));
                    }
                }
                obj
            })
            .collect()
    }

    fn build_response_format(fmt: &ResponseFormat) -> Value {
        match fmt {
            ResponseFormat::JsonObject => json!({"type": "json_object"}),
            ResponseFormat::JsonSchema {
                name,
                schema,
                strict,
            } => {
                let mut obj = json!({
                    "type": "json_schema",
                    "json_schema": {
                        "name": name,
                        "schema": schema,
                    }
                });
                if let Some(strict) = strict {
                    if let Some(root) = obj.as_object_mut() {
                        if let Some(js) =
                            root.get_mut("json_schema").and_then(|v| v.as_object_mut())
                        {
                            js.insert("strict".to_string(), json!(strict));
                        }
                    }
                }
                obj
            }
        }
    }

    fn build_tools(tools: &[ToolDefinition]) -> Vec<Value> {
        tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.input_schema,
                    }
                })
            })
            .collect()
    }

    fn parse_tool_calls(message: &Value) -> Vec<ToolCall> {
        // OpenAI: message.tool_calls: [{id,type,function:{name,arguments}}]
        let mut out = Vec::new();
        let Some(tool_calls) = message.get("tool_calls") else {
            return out;
        };
        let Some(arr) = tool_calls.as_array() else {
            return out;
        };

        for item in arr {
            let id = item
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let function = item.get("function").cloned().unwrap_or(Value::Null);
            let name = function
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let args_raw = function
                .get("arguments")
                .and_then(|v| v.as_str())
                .unwrap_or("{}");

            let input: Value = serde_json::from_str(args_raw).unwrap_or_else(|_| {
                // 如果 arguments 不是合法 JSON，则退化为字符串。
                Value::String(args_raw.to_string())
            });

            if !id.is_empty() && !name.is_empty() {
                out.push(ToolCall { id, name, input });
            }
        }

        out
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        // 优先级：1) 请求中指定的 model 2) Provider 默认模型
        let model = request
            .model
            .clone()
            .unwrap_or_else(|| self.default_model.clone());

        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let messages = Self::build_messages(&request.messages);

        let last_user_preview = request
            .messages
            .iter()
            .rev()
            .find(|m| m.role == Role::User)
            .map(|m| preview(&m.content, 600));

        debug!(
            provider = "openai",
            url = %url,
            model = %model,
            messages_len = request.messages.len(),
            tools_len = request.tools.as_ref().map(|t| t.len()).unwrap_or(0),
            last_user = last_user_preview.as_deref().unwrap_or(""),
            "provider.chat.start"
        );

        let mut body = json!({
            "model": model,
            "messages": messages,
        });

        if let Some(tools) = request.tools.as_ref() {
            if let Some(map) = body.as_object_mut() {
                map.insert("tools".to_string(), Value::Array(Self::build_tools(tools)));
            }
        }
        if let Some(t) = request.temperature {
            if let Some(map) = body.as_object_mut() {
                map.insert("temperature".to_string(), json!(t));
            }
        }
        if let Some(max_tokens) = request.max_tokens {
            if let Some(map) = body.as_object_mut() {
                map.insert("max_tokens".to_string(), json!(max_tokens));
            }
        }
        if let Some(fmt) = request.response_format.as_ref() {
            if let Some(map) = body.as_object_mut() {
                map.insert(
                    "response_format".to_string(),
                    Self::build_response_format(fmt),
                );
            }
        }

        // 支持更多参数
        if let Some(top_p) = request.top_p {
            if let Some(map) = body.as_object_mut() {
                map.insert("top_p".to_string(), json!(top_p));
            }
        }
        if let Some(top_k) = request.top_k {
            if let Some(map) = body.as_object_mut() {
                map.insert("top_k".to_string(), json!(top_k));
            }
        }
        if let Some(frequency_penalty) = request.frequency_penalty {
            if let Some(map) = body.as_object_mut() {
                map.insert("frequency_penalty".to_string(), json!(frequency_penalty));
            }
        }
        if let Some(presence_penalty) = request.presence_penalty {
            if let Some(map) = body.as_object_mut() {
                map.insert("presence_penalty".to_string(), json!(presence_penalty));
            }
        }
        if let Some(stop) = request.stop.as_ref() {
            if let Some(map) = body.as_object_mut() {
                map.insert("stop".to_string(), json!(stop));
            }
        }

        // 额外参数（用于支持 provider 特定的参数，如 NVIDIA 的 reasoning_budget 等）
        if let Some(extra) = request.extra.as_ref() {
            if let Some(map) = body.as_object_mut() {
                for (key, value) in extra.as_object().unwrap_or(&serde_json::Map::new()) {
                    map.insert(key.clone(), value.clone());
                }
            }
        }

        debug!(
            provider = "openai",
            model = %model,
            body = %preview(&body.to_string(), 1200),
            "provider.chat.request_body"
        );

        let start = std::time::Instant::now();

        let resp = self
            .client
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        let status = resp.status();

        // 先读取原始文本，再尝试解析 JSON
        let text = resp
            .text()
            .await
            .map_err(|e| ProviderError::Message(format!("读取响应失败：{}", e)))?;

        let elapsed_ms = start.elapsed().as_millis() as u64;
        debug!(
            provider = "openai",
            status = %status,
            elapsed_ms,
            "provider.chat.http.done"
        );
        debug!(
            provider = "openai",
            status = %status,
            body = %preview(&text, 1200),
            "provider.chat.response_body"
        );

        if !status.is_success() {
            return Err(ProviderError::Message(format!(
                "OpenAI 请求失败：status={} body={} ",
                status, text
            )));
        }

        // 尝试解析 JSON，提供更友好的错误信息
        let data: Value = serde_json::from_str(&text).map_err(|e| {
            ProviderError::Message(format!(
                "解析响应 JSON 失败：{}。响应内容：{}",
                e,
                preview(&text, 500)
            ))
        })?;

        // 解析响应，兼容第三方 API
        let message = data
            .get("choices")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|c| c.get("message"));

        if message.is_none() {
            // 尝试兼容某些第三方 API 的格式
            if let Some(error) = data.get("error") {
                return Err(ProviderError::Message(format!("API 返回错误：{}", error)));
            }

            return Err(ProviderError::Message(format!(
                "OpenAI 响应格式不兼容。响应内容：{}",
                preview(&text, 500)
            )));
        }

        let message = message.unwrap().clone();

        // 解析 content，兼容多种格式
        let mut content = message
            .get("content")
            .and_then(|v| {
                // 可能是字符串
                if let Some(s) = v.as_str() {
                    return Some(s.to_string());
                }
                // 可能是对象（如某些第三方 API）
                if let Some(obj) = v.as_object() {
                    // 尝试获取 text 字段
                    if let Some(text) = obj.get("text").and_then(|t| t.as_str()) {
                        return Some(text.to_string());
                    }
                }
                None
            })
            .unwrap_or_default();

        let tool_calls = Self::parse_tool_calls(&message);

        // 兼容部分第三方 API：把最终回答写进 reasoning 字段而 content 为空。
        if content.trim().is_empty() && tool_calls.is_empty() {
            if let Some(r) = message.get("reasoning").and_then(|v| v.as_str()) {
                if !r.trim().is_empty() {
                    content = r.to_string();
                }
            }
        }

        if !tool_calls.is_empty() {
            let names: Vec<&str> = tool_calls.iter().map(|c| c.name.as_str()).collect();
            debug!(
                provider = "openai",
                tool_calls_len = tool_calls.len(),
                tool_call_names = ?names,
                "provider.chat.tool_calls"
            );
        }

        // 解析 usage 字段
        let usage = data
            .get("usage")
            .and_then(|u| u.as_object())
            .map(|usage_obj| agentkit_core::provider::types::Usage {
                prompt_tokens: usage_obj
                    .get("prompt_tokens")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32)
                    .unwrap_or(0),
                completion_tokens: usage_obj
                    .get("completion_tokens")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32)
                    .unwrap_or(0),
                total_tokens: usage_obj
                    .get("total_tokens")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32)
                    .unwrap_or(0),
            });

        // 解析 finish_reason 字段
        let finish_reason = data
            .get("choices")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|c| c.get("finish_reason"))
            .and_then(|fr| fr.as_str())
            .map(|fr| match fr {
                "stop" => FinishReason::Stop,
                "length" => FinishReason::Length,
                "tool_calls" => FinishReason::ToolCall,
                _ => FinishReason::Other,
            });

        debug!(
            provider = "openai",
            assistant_content_len = content.len(),
            "provider.chat.parsed"
        );

        Ok(ChatResponse {
            message: ChatMessage {
                role: Role::Assistant,
                content,
                name: None,
            },
            tool_calls,
            usage,
            finish_reason,
        })
    }

    fn stream_chat(
        &self,
        request: ChatRequest,
    ) -> Result<BoxStream<'static, Result<ChatStreamChunk, ProviderError>>, ProviderError> {
        // 优先级：1) 请求中指定的 model 2) Provider 默认模型
        let model = request
            .model
            .clone()
            .unwrap_or_else(|| self.default_model.clone());

        let client = self.client.clone();
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

        let preview = |s: &str, max: usize| {
            if s.len() <= max {
                s.to_string()
            } else {
                // 使用 char_indices 找到正确的字符边界，避免截断多字节字符
                let truncated: String = s.char_indices().take(max).map(|(_, c)| c).collect();
                format!("{}...<truncated:{}>", truncated, s.len())
            }
        };

        debug!(
            provider = "openai",
            url = %url,
            model = %model,
            messages_len = request.messages.len(),
            tools_len = request.tools.as_ref().map(|t| t.len()).unwrap_or(0),
            "provider.stream_chat.start"
        );

        let mut body = json!({
            "model": model,
            "messages": Self::build_messages(&request.messages),
            "stream": true,
        });

        if let Some(tools) = request.tools.as_ref() {
            if let Some(map) = body.as_object_mut() {
                map.insert("tools".to_string(), Value::Array(Self::build_tools(tools)));
            }
        }
        if let Some(t) = request.temperature {
            if let Some(map) = body.as_object_mut() {
                map.insert("temperature".to_string(), json!(t));
            }
        }
        if let Some(max_tokens) = request.max_tokens {
            if let Some(map) = body.as_object_mut() {
                map.insert("max_tokens".to_string(), json!(max_tokens));
            }
        }

        if let Some(fmt) = request.response_format.as_ref() {
            if let Some(map) = body.as_object_mut() {
                map.insert(
                    "response_format".to_string(),
                    Self::build_response_format(fmt),
                );
            }
        }

        debug!(
            provider = "openai",
            model = %model,
            body = %preview(&body.to_string(), 1200),
            "provider.stream_chat.request_body"
        );

        // 说明：OpenAI 的流式输出是 SSE（data: ... \n\n）。
        // 这里实现一个尽量健壮的解析器：把 bytes 累积成字符串，按 "\n\n" 切分事件。
        let stream = async_stream::try_stream! {
            let start = std::time::Instant::now();
            let resp = client
                .post(url)
                .json(&body)
                .send()
                .await
                .map_err(|e| ProviderError::Message(e.to_string()))?;

            let status = resp.status();
            if !status.is_success() {
                Err(ProviderError::Message(format!(
                    "OpenAI stream 请求失败：status={}",
                    status
                )))?;
            }

            debug!(
                provider = "openai",
                status = %status,
                elapsed_ms = start.elapsed().as_millis() as u64,
                "provider.stream_chat.http.started"
            );

            let mut buf = String::new();
            let mut bytes_stream = resp.bytes_stream();

            while let Some(item) = bytes_stream.next().await {
                let bytes = item.map_err(|e| ProviderError::Message(e.to_string()))?;
                let chunk = String::from_utf8_lossy(&bytes);
                buf.push_str(&chunk);

                // SSE 事件以空行分隔。
                while let Some(idx) = buf.find("\n\n") {
                    let event = buf[..idx].to_string();
                    buf = buf[idx + 2..].to_string();

                    // 只处理 data 行（可能有多行 data）。
                    let mut data_lines: Vec<&str> = Vec::new();
                    for line in event.lines() {
                        let line = line.trim();
                        if let Some(rest) = line.strip_prefix("data:") {
                            data_lines.push(rest.trim());
                        }
                    }

                    if data_lines.is_empty() {
                        continue;
                    }

                    let data = data_lines.join("\n");
                    if data == "[DONE]" {
                        break;
                    }

                    let v: Value = serde_json::from_str(&data)
                        .map_err(|e| ProviderError::Message(format!("SSE JSON 解析失败: {} data={}", e, data)))?;

                    // OpenAI: choices[0].delta.content
                    let delta = v
                        .get("choices")
                        .and_then(|c| c.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|c0| c0.get("delta"))
                        .and_then(|d| d.get("content"))
                        .and_then(|s| s.as_str())
                        .map(|s| s.to_string());

                    // 当前实现仅输出文本增量；tool_calls/usage/finish_reason 可后续扩展。
                    if delta.is_some() {
                        yield ChatStreamChunk {
                            delta,
                            tool_calls: vec![],
                            usage: None,
                            finish_reason: None,
                        };
                    }
                }

                // 如果已经收到 [DONE]，buf 会在上面的 break 后保留剩余内容；这里直接结束流。
                if buf.contains("[DONE]") {
                    break;
                }
            }
        };

        Ok(Box::pin(stream))
    }
}
