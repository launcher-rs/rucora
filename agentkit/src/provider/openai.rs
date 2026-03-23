//! OpenAI Provider 实现。
//!
//! 约定：
//! - API Key 从 `OPENAI_API_KEY` 环境变量读取
//! - Base URL 默认 `https://api.openai.com/v1`，也可通过 `OPENAI_BASE_URL` 覆盖

use std::env;

use agentkit_core::{
    error::ProviderError,
    provider::{
        LlmProvider,
        types::{ChatMessage, ChatRequest, ChatResponse, ChatStreamChunk, ResponseFormat, Role},
    },
    tool::types::{ToolCall, ToolDefinition},
};
use async_trait::async_trait;
use futures_util::{StreamExt, stream::BoxStream};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::{Value, json};
use tracing::{debug, trace};

/// OpenAI Chat Completions Provider。
///
/// 说明：
/// - 目前仅实现 `chat`（非流式）
/// - `tools` 会按 OpenAI 的 function tools 格式传入
pub struct OpenAiProvider {
    client: reqwest::Client,
    base_url: String,
    default_model: Option<String>,
}

impl OpenAiProvider {
    /// 从环境变量创建 Provider。
    pub fn from_env() -> Result<Self, ProviderError> {
        let api_key = env::var("OPENAI_API_KEY")
            .map_err(|_| ProviderError::Message("缺少环境变量 OPENAI_API_KEY".to_string()))?;
        let base_url =
            env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".to_string());

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
        let model = request
            .model
            .clone()
            .or_else(|| self.default_model.clone())
            .ok_or_else(|| ProviderError::Message("OpenAI 请求缺少 model".to_string()))?;

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

        trace!(
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
        let data: Value = resp
            .json()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        let elapsed_ms = start.elapsed().as_millis() as u64;
        debug!(
            provider = "openai",
            status = %status,
            elapsed_ms,
            "provider.chat.http.done"
        );
        trace!(
            provider = "openai",
            status = %status,
            body = %preview(&data.to_string(), 1200),
            "provider.chat.response_body"
        );

        if !status.is_success() {
            return Err(ProviderError::Message(format!(
                "OpenAI 请求失败：status={} body={} ",
                status, data
            )));
        }

        let message = data
            .get("choices")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|c| c.get("message"))
            .cloned()
            .ok_or_else(|| {
                ProviderError::Message("OpenAI 响应缺少 choices[0].message".to_string())
            })?;

        let content = message
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let tool_calls = Self::parse_tool_calls(&message);

        if !tool_calls.is_empty() {
            let names: Vec<&str> = tool_calls.iter().map(|c| c.name.as_str()).collect();
            debug!(
                provider = "openai",
                tool_calls_len = tool_calls.len(),
                tool_call_names = ?names,
                "provider.chat.tool_calls"
            );
        }

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
            usage: None,
            finish_reason: None,
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
            .ok_or_else(|| ProviderError::Message("OpenAI 请求缺少 model".to_string()))?;

        let client = self.client.clone();
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

        let preview = |s: &str, max: usize| {
            if s.len() <= max {
                s.to_string()
            } else {
                format!("{}...<truncated:{}>", &s[..max], s.len())
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

        trace!(
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
