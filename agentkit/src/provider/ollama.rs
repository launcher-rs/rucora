//! Ollama Provider 实现。
//!
//! 约定：
//! - Base URL 默认 `http://localhost:11434`，也可通过 `OLLAMA_BASE_URL` 覆盖
//! - chat endpoint 使用 `/api/chat`

use std::env;

use crate::provider::preview;
use agentkit_core::{
    error::ProviderError,
    provider::{
        LlmProvider,
        types::{
            ChatMessage, ChatRequest, ChatResponse, ChatStreamChunk, FinishReason, ResponseFormat,
            Role, Usage,
        },
    },
};
use async_trait::async_trait;
use futures_util::{StreamExt, stream::BoxStream};
use serde_json::{Value, json};
use tracing::debug;

/// Ollama 默认模型。
pub const OLLAMA_DEFAULT_MODEL: &str = "llama3.1:8b";

/// Ollama Chat Provider。
///
/// 说明：
/// - 目前仅实现 `chat`（非流式）
/// - tools 暂不做强保证（不同 Ollama 版本对 tools 支持存在差异）
///
/// # 环境变量
///
/// | 变量名 | 说明 | 示例 |
/// |--------|------|------|
/// | `OLLAMA_BASE_URL` | Ollama Base URL | `http://localhost:11434` |
/// | `OLLAMA_DEFAULT_MODEL` | 默认模型 | `llama3.1:8b` |
pub struct OllamaProvider {
    client: reqwest::Client,
    base_url: String,
    default_model: String,
}

impl OllamaProvider {
    /// 从环境变量创建 Provider。
    ///
    /// 默认模型优先级：
    /// 1. `OLLAMA_DEFAULT_MODEL` 环境变量
    /// 2. 内置默认值 `llama3.1:8b`
    pub fn from_env() -> Self {
        let base_url =
            env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());
        let default_model =
            env::var("OLLAMA_DEFAULT_MODEL").unwrap_or_else(|_| OLLAMA_DEFAULT_MODEL.to_string());
        Self::with_model(base_url, default_model)
    }

    /// 创建 Provider。
    ///
    /// 使用内置默认模型 `llama3.1:8b`。
    pub fn new(base_url: impl Into<String>) -> Self {
        Self::with_model(base_url, OLLAMA_DEFAULT_MODEL.to_string())
    }

    /// 创建 Provider 并指定默认模型。
    pub fn with_model(base_url: impl Into<String>, default_model: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            default_model: default_model.into(),
        }
    }

    /// 设置默认模型。
    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    /// 获取默认模型。
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
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let model = request
            .model
            .clone()
            .unwrap_or_else(|| self.default_model.clone());

        let url = format!("{}/api/chat", self.base_url.trim_end_matches('/'));
        let mut body = json!({
            "model": model,
            "messages": Self::build_messages(&request.messages),
            "stream": false
        });

        // 添加工具定义（如果存在）
        if let Some(tools) = &request.tools {
            if let Some(map) = body.as_object_mut() {
                let tools_array: Vec<Value> = tools
                    .iter()
                    .map(|tool_def| {
                        json!({
                            "type": "function",
                            "function": {
                                "name": tool_def.name,
                                "description": tool_def.description.as_deref().unwrap_or(""),
                                "parameters": tool_def.input_schema
                            }
                        })
                    })
                    .collect();
                map.insert("tools".to_string(), json!(tools_array));
            }
        }

        if let Some(fmt) = request.response_format.as_ref() {
            match fmt {
                ResponseFormat::JsonObject => {
                    if let Some(map) = body.as_object_mut() {
                        map.insert("format".to_string(), json!("json"));
                    }
                }
                ResponseFormat::JsonSchema { .. } => {
                    return Err(ProviderError::Message(
                        "Ollama provider 暂不支持 JsonSchema 结构化输出".to_string(),
                    ));
                }
            }
        }

        let last_user_preview = request
            .messages
            .iter()
            .rev()
            .find(|m| m.role == Role::User)
            .map(|m| preview(&m.content, 600));

        debug!(
            provider = "ollama",
            url = %url,
            model = %body.get("model").and_then(|v| v.as_str()).unwrap_or(""),
            messages_len = request.messages.len(),
            tools_len = request.tools.as_ref().map(|t| t.len()).unwrap_or(0),
            last_user = last_user_preview.as_deref().unwrap_or(""),
            "provider.chat.start"
        );
        debug!(provider = "ollama", body = %preview(&body.to_string(), 1200), "provider.chat.request_body");

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
        debug!(provider = "ollama", status = %status, elapsed_ms, "provider.chat.http.done");
        debug!(provider = "ollama", status = %status, body = %preview(&data.to_string(), 1200), "provider.chat.response_body");

        if !status.is_success() {
            return Err(ProviderError::Message(format!(
                "Ollama 请求失败：status={} body={}",
                status, data
            )));
        }

        let content = data
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // 解析 tool_calls（如果存在）
        let tool_calls = data
            .get("message")
            .and_then(|m| m.get("tool_calls"))
            .and_then(|tc| tc.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|tc| {
                        let function = tc.get("function")?;
                        let name = function.get("name")?.as_str()?.to_string();
                        let arguments = function
                            .get("arguments")
                            .and_then(|a| a.as_str())
                            .unwrap_or("{}");
                        let input: Value = serde_json::from_str(arguments)
                            .unwrap_or_else(|_| json!({"_raw": arguments}));
                        let id = tc
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        if name.is_empty() {
                            None
                        } else {
                            Some(agentkit_core::tool::types::ToolCall { id, name, input })
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        debug!(
            provider = "ollama",
            assistant_content_len = content.len(),
            tool_calls_len = tool_calls.len(),
            "provider.chat.parsed"
        );

        // 解析 usage 字段（Ollama 某些版本支持）
        let usage = data.get("usage").map(|u| Usage {
            prompt_tokens: u
                .get("prompt_tokens")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
                .unwrap_or(0),
            completion_tokens: u
                .get("completion_tokens")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
                .unwrap_or(0),
            total_tokens: u
                .get("total_tokens")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
                .unwrap_or(0),
        });

        // 解析 finish_reason 字段
        let finish_reason = data
            .get("done_reason")
            .and_then(|v| v.as_str())
            .map(|fr| match fr {
                "stop" => FinishReason::Stop,
                "length" => FinishReason::Length,
                "tool_calls" => FinishReason::ToolCall,
                _ => FinishReason::Other,
            });

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
        let model = request
            .model
            .clone()
            .unwrap_or_else(|| self.default_model.clone());

        let client = self.client.clone();
        let url = format!("{}/api/chat", self.base_url.trim_end_matches('/'));
        let mut body = json!({
            "model": model,
            "messages": Self::build_messages(&request.messages),
            "stream": true
        });

        if let Some(fmt) = request.response_format.as_ref() {
            match fmt {
                ResponseFormat::JsonObject => {
                    if let Some(map) = body.as_object_mut() {
                        map.insert("format".to_string(), json!("json"));
                    }
                }
                ResponseFormat::JsonSchema { .. } => {
                    return Err(ProviderError::Message(
                        "Ollama provider 暂不支持 JsonSchema 结构化输出".to_string(),
                    ));
                }
            }
        }

        debug!(
            provider = "ollama",
            url = %url,
            model = %body.get("model").and_then(|v| v.as_str()).unwrap_or(""),
            messages_len = request.messages.len(),
            "provider.stream_chat.start"
        );
        debug!(provider = "ollama", body = %preview(&body.to_string(), 1200), "provider.stream_chat.request_body");

        // 说明：Ollama 的流式输出通常是“每行一个 JSON”（NDJSON）。
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
                    "Ollama stream 请求失败：status={}",
                    status
                )))?;
            }

            debug!(
                provider = "ollama",
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

                while let Some(idx) = buf.find('\n') {
                    let line = buf[..idx].trim().to_string();
                    buf = buf[idx + 1..].to_string();

                    if line.is_empty() {
                        continue;
                    }

                    let v: Value = serde_json::from_str(&line)
                        .map_err(|e| ProviderError::Message(format!("NDJSON 解析失败: {} line={}", e, line)))?;

                    let done = v.get("done").and_then(|d| d.as_bool()).unwrap_or(false);
                    let delta = v
                        .get("message")
                        .and_then(|m| m.get("content"))
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

                    if done {
                        break;
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }
}
