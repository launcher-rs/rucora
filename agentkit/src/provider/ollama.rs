//! Ollama Provider 实现。
//!
//! 约定：
//! - Base URL 默认 `http://localhost:11434`，也可通过 `OLLAMA_BASE_URL` 覆盖
//! - chat endpoint 使用 `/api/chat`

use std::env;

use agentkit_core::{
    error::ProviderError,
    provider::{
        types::{ChatMessage, ChatRequest, ChatResponse, ChatStreamChunk, ResponseFormat, Role},
        LlmProvider,
    },
};
use async_trait::async_trait;
use futures_util::{stream::BoxStream, StreamExt};
use serde_json::{json, Value};
use tracing::{debug, trace};

/// Ollama Chat Provider。
///
/// 说明：
/// - 目前仅实现 `chat`（非流式）
/// - tools 暂不做强保证（不同 Ollama 版本对 tools 支持存在差异）
pub struct OllamaProvider {
    client: reqwest::Client,
    base_url: String,
    default_model: Option<String>,
}

impl OllamaProvider {
    /// 从环境变量创建 Provider。
    pub fn from_env() -> Self {
        let base_url =
            env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());
        Self::new(base_url)
    }

    /// 创建 Provider。
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
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
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let model = request
            .model
            .clone()
            .or_else(|| self.default_model.clone())
            .ok_or_else(|| ProviderError::Message("Ollama 请求缺少 model".to_string()))?;

        let url = format!("{}/api/chat", self.base_url.trim_end_matches('/'));
        let mut body = json!({
            "model": model,
            "messages": Self::build_messages(&request.messages),
            "stream": false
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
            provider = "ollama",
            url = %url,
            model = %body.get("model").and_then(|v| v.as_str()).unwrap_or(""),
            messages_len = request.messages.len(),
            last_user = last_user_preview.as_deref().unwrap_or(""),
            "provider.chat.start"
        );
        trace!(provider = "ollama", body = %preview(&body.to_string(), 1200), "provider.chat.request_body");

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
        trace!(provider = "ollama", status = %status, body = %preview(&data.to_string(), 1200), "provider.chat.response_body");

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

        debug!(
            provider = "ollama",
            assistant_content_len = content.len(),
            "provider.chat.parsed"
        );

        Ok(ChatResponse {
            message: ChatMessage {
                role: Role::Assistant,
                content,
                name: None,
            },
            tool_calls: vec![],
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
            .ok_or_else(|| ProviderError::Message("Ollama 请求缺少 model".to_string()))?;

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

        let preview = |s: &str, max: usize| {
            if s.len() <= max {
                s.to_string()
            } else {
                format!("{}...<truncated:{}>", &s[..max], s.len())
            }
        };

        debug!(
            provider = "ollama",
            url = %url,
            model = %body.get("model").and_then(|v| v.as_str()).unwrap_or(""),
            messages_len = request.messages.len(),
            "provider.stream_chat.start"
        );
        trace!(provider = "ollama", body = %preview(&body.to_string(), 1200), "provider.stream_chat.request_body");

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
