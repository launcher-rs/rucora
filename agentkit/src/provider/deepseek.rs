//! DeepSeek Provider 实现。
//!
//! 约定：
//! - API Key 从 `DEEPSEEK_API_KEY` 环境变量读取
//! - Base URL 默认 `https://api.deepseek.com/v1`
//! - 使用 OpenAI 兼容的 API 格式
//! - 支持 DeepSeek 系列模型（DeepSeek-V3, DeepSeek-R1 等）

use std::env;

use crate::provider::helpers::parse_finish_reason;
use agentkit_core::{
    error::ProviderError,
    provider::{
        types::{
            ChatMessage, ChatRequest, ChatResponse, ChatStreamChunk, ResponseFormat, Role, Usage,
        },
        LlmProvider,
    },
    tool::types::{ToolCall, ToolDefinition},
};
use async_trait::async_trait;
use futures_util::{stream::BoxStream, StreamExt};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};
use tracing::{debug, trace};

/// DeepSeek Provider。
///
/// 支持 DeepSeek 系列模型：
/// - deepseek-chat (DeepSeek-V3)
/// - deepseek-reasoner (DeepSeek-R1)
///
/// # 使用示例
///
/// ```rust,no_run
/// use agentkit::provider::DeepSeekProvider;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // 从环境变量加载
/// let provider = DeepSeekProvider::from_env()?;
///
/// // 或手动配置
/// let provider = DeepSeekProvider::new("sk-...");
///
/// // 使用特定模型
/// let provider = provider.with_default_model("deepseek-chat");
/// # Ok(())
/// # }
/// ```
///
/// # 环境变量
///
/// | 变量名 | 说明 | 示例 |
/// |--------|------|------|
/// | `DEEPSEEK_API_KEY` | DeepSeek API Key | `sk-...` |
/// | `DEEPSEEK_BASE_URL` | DeepSeek Base URL | `https://api.deepseek.com/v1` |
pub struct DeepSeekProvider {
    client: reqwest::Client,
    base_url: String,
    default_model: Option<String>,
}

impl DeepSeekProvider {
    /// 从环境变量创建 Provider。
    pub fn from_env() -> Result<Self, ProviderError> {
        let api_key = env::var("DEEPSEEK_API_KEY")
            .map_err(|_| ProviderError::Message("缺少环境变量 DEEPSEEK_API_KEY".to_string()))?;
        let base_url = env::var("DEEPSEEK_BASE_URL")
            .unwrap_or_else(|_| "https://api.deepseek.com/v1".to_string());

        Ok(Self::new(base_url, api_key))
    }

    /// 创建 Provider。
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
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
            default_model: None,
        }
    }

    /// 仅使用 API Key 创建 Provider（使用默认 base_url）。
    pub fn with_api_key(api_key: impl Into<String>) -> Self {
        Self::new("https://api.deepseek.com/v1", api_key)
    }

    /// 设置默认模型。
    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = Some(model.into());
        self
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

            let input: Value = serde_json::from_str(args_raw)
                .unwrap_or_else(|_| Value::String(args_raw.to_string()));

            if !id.is_empty() && !name.is_empty() {
                out.push(ToolCall { id, name, input });
            }
        }

        out
    }
}

#[async_trait]
impl LlmProvider for DeepSeekProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let model = request
            .model
            .clone()
            .or_else(|| self.default_model.clone())
            .ok_or_else(|| ProviderError::Message("DeepSeek 请求缺少 model".to_string()))?;

        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let messages = Self::build_messages(&request.messages);

        let preview = |s: &str, max: usize| {
            if s.len() <= max {
                s.to_string()
            } else {
                format!("{}...<truncated:{}>", &s[..max], s.len())
            }
        };

        let last_user_preview = request
            .messages
            .iter()
            .rev()
            .find(|m| m.role == Role::User)
            .map(|m| preview(&m.content, 600));

        debug!(
            provider = "deepseek",
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

        trace!(
            provider = "deepseek",
            model = %model,
            body = %preview(&body.to_string(), 1200),
            "provider.chat.request_body"
        );

        let start = std::time::Instant::now();

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        let status = resp.status();
        let data: Value = resp
            .json()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        let elapsed_ms = start.elapsed().as_millis() as u64;
        debug!(
            provider = "deepseek",
            status = %status,
            elapsed_ms,
            "provider.chat.http.done"
        );
        trace!(
            provider = "deepseek",
            status = %status,
            body = %preview(&data.to_string(), 1200),
            "provider.chat.response_body"
        );

        if !status.is_success() {
            return Err(ProviderError::Message(format!(
                "DeepSeek 请求失败：status={} body={}",
                status, data
            )));
        }

        let message = data
            .get("choices")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|c| c.get("message"))
            .cloned()
            .unwrap_or_else(|| json!({}));

        let content = message
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let tool_calls = Self::parse_tool_calls(&message);

        let usage = data.get("usage").map(|u| Usage {
            prompt_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            completion_tokens: u
                .get("completion_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32,
            total_tokens: u.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        });

        let finish_reason = data
            .get("choices")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|c| c.get("finish_reason"))
            .and_then(|v| v.as_str())
            .unwrap_or("stop")
            .to_string();

        Ok(ChatResponse {
            message: ChatMessage {
                role: Role::Assistant,
                content,
                name: None,
            },
            tool_calls,
            usage,
            finish_reason: Some(parse_finish_reason(&finish_reason)),
        })
    }

    fn stream_chat(
        &self,
        request: ChatRequest,
    ) -> Result<BoxStream<'static, Result<ChatStreamChunk, ProviderError>>, ProviderError> {
        let model = request
            .model
            .clone()
            .or_else(|| self.default_model.clone())
            .ok_or_else(|| ProviderError::Message("DeepSeek 请求缺少 model".to_string()))?;

        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let messages = Self::build_messages(&request.messages);

        let mut body = json!({
            "model": model,
            "messages": messages,
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

        let client = self.client.clone();
        let stream = async_stream::try_stream! {
            let resp = client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| ProviderError::Message(e.to_string()))?;

            let status = resp.status();
            if !status.is_success() {
                Err(ProviderError::Message(format!(
                    "DeepSeek 流式请求失败：status={}",
                    status
                )))?;
            }

            let mut buf = String::new();
            let mut bytes_stream = resp.bytes_stream();

            while let Some(item) = bytes_stream.next().await {
                let bytes = item.map_err(|e| ProviderError::Message(e.to_string()))?;
                let chunk = String::from_utf8_lossy(&bytes);
                buf.push_str(&chunk);

                while let Some(idx) = buf.find("\n\n") {
                    let event = buf[..idx].to_string();
                    buf = buf[idx + 2..].to_string();

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
                        .map_err(|e| ProviderError::Message(format!("SSE 解析失败：{} data={}", e, data)))?;

                    let delta = v
                        .get("choices")
                        .and_then(|c| c.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|c0| c0.get("delta"))
                        .and_then(|d| d.get("content"))
                        .and_then(|s| s.as_str())
                        .map(|s| s.to_string());

                    if delta.is_some() {
                        yield ChatStreamChunk {
                            delta,
                            tool_calls: vec![],
                            usage: None,
                            finish_reason: None,
                        };
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deepseek_provider_creation() {
        let provider = DeepSeekProvider::with_api_key("test-key");
        assert_eq!(provider.base_url, "https://api.deepseek.com/v1");
    }

    #[test]
    fn test_deepseek_provider_with_custom_model() {
        let provider = DeepSeekProvider::new("https://api.deepseek.com/v1", "test-key")
            .with_default_model("deepseek-chat");
        assert_eq!(provider.default_model, Some("deepseek-chat".to_string()));
    }
}
