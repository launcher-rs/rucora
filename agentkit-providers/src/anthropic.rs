//! Anthropic Provider 实现。
//!
//! 约定：
//! - API Key 从 `ANTHROPIC_API_KEY` 环境变量读取
//! - Base URL 默认 `https://api.anthropic.com/v1`
//! - 使用 Anthropic Messages API 格式
//! - 支持 Claude 系列模型（Claude 2, Claude 3 等）

use std::env;

use crate::{helpers::parse_finish_reason, http_config::build_client, preview};
use agentkit_core::{
    error::ProviderError,
    provider::{
        LlmProvider,
        types::{ChatMessage, ChatRequest, ChatResponse, ChatStreamChunk, Role, Usage},
    },
    tool::types::{ToolCall, ToolDefinition},
};
use async_trait::async_trait;
use futures_util::{StreamExt, stream::BoxStream};
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::{Value, json};
use tracing::debug;

/// Anthropic 默认模型。
pub const ANTHROPIC_DEFAULT_MODEL: &str = "claude-3-5-sonnet-20241022";

/// Anthropic Claude Provider。
///
/// 支持 Claude 系列模型：
/// - claude-3-5-sonnet-20241022 (最新，默认)
/// - claude-3-5-haiku-20241022
/// - claude-3-opus-20240229
/// - claude-3-sonnet-20240229
/// - claude-3-haiku-20240307
/// - claude-2.1
///
/// # 默认模型优先级
///
/// 1. **手动设置**：通过 `with_model()` 或 `with_default_model()` 显式设置
/// 2. **环境变量**：`ANTHROPIC_DEFAULT_MODEL` 环境变量
/// 3. **内置默认值**：`claude-3-5-sonnet-20241022`
///
/// # 使用示例
///
/// ```rust,no_run
/// use agentkit::provider::AnthropicProvider;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // 从环境变量加载（包括 ANTHROPIC_DEFAULT_MODEL）
/// let provider = AnthropicProvider::from_env()?;
///
/// // 或手动配置（使用内置默认模型）
/// let provider = AnthropicProvider::new("sk-ant-...");
///
/// // 使用特定模型（手动设置优先级最高）
/// let provider = provider.with_default_model("claude-3-opus-20240229");
/// # Ok(())
/// # }
/// ```
///
/// # 环境变量
///
/// | 变量名 | 说明 | 示例 |
/// |--------|------|------|
/// | `ANTHROPIC_API_KEY` | Anthropic API Key | `sk-ant-...` |
/// | `ANTHROPIC_BASE_URL` | Anthropic Base URL | `https://api.anthropic.com/v1` |
/// | `ANTHROPIC_DEFAULT_MODEL` | 默认模型 | `claude-3-5-sonnet-20241022` |
pub struct AnthropicProvider {
    client: reqwest::Client,
    base_url: String,
    default_model: String,
    api_version: String,
}

impl AnthropicProvider {
    /// 从环境变量创建 Provider。
    ///
    /// 从以下环境变量加载配置：
    /// - `ANTHROPIC_API_KEY`: API Key（必需）
    /// - `ANTHROPIC_BASE_URL`: Base URL（可选，默认 `https://api.anthropic.com/v1`）
    /// - `ANTHROPIC_DEFAULT_MODEL`: 默认模型（可选，默认 `claude-3-5-sonnet-20241022`）
    pub fn from_env() -> Result<Self, ProviderError> {
        let api_key = env::var("ANTHROPIC_API_KEY")
            .map_err(|_| ProviderError::Message("缺少环境变量 ANTHROPIC_API_KEY".to_string()))?;
        let base_url = env::var("ANTHROPIC_BASE_URL")
            .unwrap_or_else(|_| "https://api.anthropic.com/v1".to_string());
        let default_model = env::var("ANTHROPIC_DEFAULT_MODEL")
            .unwrap_or_else(|_| ANTHROPIC_DEFAULT_MODEL.to_string());

        Ok(Self::with_model(base_url, api_key, default_model))
    }

    /// 创建 Provider（使用内置默认模型）。
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self::with_model(base_url, api_key, ANTHROPIC_DEFAULT_MODEL.to_string())
    }

    /// 创建 Provider（指定默认模型）。
    ///
    /// # 参数
    ///
    /// * `base_url` - API Base URL
    /// * `api_key` - API Key
    /// * `default_model` - 默认模型名称
    pub fn with_model(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        default_model: impl Into<String>,
    ) -> Self {
        let api_key = api_key.into();
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        // Anthropic 使用 x-api-key 头部
        if let Ok(v) = HeaderValue::from_str(&api_key) {
            headers.insert("x-api-key", v);
        }
        // Anthropic 需要版本号
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));

        let client = build_client(headers);

        Self {
            client,
            base_url: base_url.into(),
            default_model: default_model.into(),
            api_version: "2023-06-01".to_string(),
        }
    }

    /// 仅使用 API Key 创建 Provider（使用默认 base_url 和内置默认模型）。
    pub fn with_api_key(api_key: impl Into<String>) -> Self {
        Self::with_model(
            "https://api.anthropic.com/v1",
            api_key,
            ANTHROPIC_DEFAULT_MODEL,
        )
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

    /// 设置 API 版本。
    pub fn with_api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = version.into();
        self
    }

    fn build_system_prompt(messages: &[ChatMessage]) -> Option<String> {
        messages
            .iter()
            .find(|m| m.role == Role::System)
            .map(|m| m.content.clone())
    }

    fn build_messages(messages: &[ChatMessage]) -> Vec<Value> {
        messages
            .iter()
            .filter(|m| m.role != Role::System) // System prompt 单独处理
            .map(|m| {
                let role = match m.role {
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    _ => "user", // Tool role 在 Anthropic 中映射为 user
                };
                json!({
                    "role": role,
                    "content": m.content,
                })
            })
            .collect()
    }

    fn build_tools(tools: &[ToolDefinition]) -> Vec<Value> {
        tools
            .iter()
            .map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.input_schema,
                })
            })
            .collect()
    }

    fn parse_tool_calls(content: &Value) -> Vec<ToolCall> {
        let mut out = Vec::new();

        // Anthropic 的 tool_use 在 content 数组中
        if let Some(content_arr) = content.as_array() {
            for item in content_arr {
                if let Some(tool_type) = item.get("type").and_then(|v| v.as_str()) {
                    if tool_type == "tool_use" {
                        let id = item
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let name = item
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let input = item.get("input").cloned().unwrap_or_else(|| json!({}));

                        if !id.is_empty() && !name.is_empty() {
                            out.push(ToolCall { id, name, input });
                        }
                    }
                }
            }
        }

        out
    }

    fn extract_text_content(content: &Value) -> String {
        // Anthropic 返回的 content 可能是数组或字符串
        if let Some(text) = content.as_str() {
            return text.to_string();
        }

        // 如果是数组，提取所有 text 类型的内容
        if let Some(content_arr) = content.as_array() {
            let mut texts = Vec::new();
            for item in content_arr {
                if let Some(tool_type) = item.get("type").and_then(|v| v.as_str()) {
                    if tool_type == "text" {
                        if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                            texts.push(text);
                        }
                    }
                }
            }
            if !texts.is_empty() {
                return texts.join("\n");
            }
        }

        String::new()
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let model = request
            .model
            .clone()
            .unwrap_or_else(|| self.default_model.clone());

        let url = format!("{}/messages", self.base_url.trim_end_matches('/'));

        let system_prompt = Self::build_system_prompt(&request.messages);
        let messages = Self::build_messages(&request.messages);

        let last_user_preview = request
            .messages
            .iter()
            .rev()
            .find(|m| m.role == Role::User)
            .map(|m| preview(&m.content, 600));

        debug!(
            provider = "anthropic",
            url = %url,
            model = %model,
            messages_len = messages.len(),
            tools_len = request.tools.as_ref().map(|t| t.len()).unwrap_or(0),
            last_user = last_user_preview.as_deref().unwrap_or(""),
            "provider.chat.start"
        );

        let mut body = json!({
            "model": model,
            "messages": messages,
        });

        // Anthropic 支持 system prompt 作为顶层字段
        if let Some(system) = system_prompt {
            if let Some(map) = body.as_object_mut() {
                map.insert("system".to_string(), json!(system));
            }
        }

        if let Some(tools) = request.tools.as_ref() {
            if let Some(map) = body.as_object_mut() {
                map.insert("tools".to_string(), Value::Array(Self::build_tools(tools)));
            }
        }
        if let Some(max_tokens) = request.max_tokens {
            if let Some(map) = body.as_object_mut() {
                map.insert("max_tokens".to_string(), json!(max_tokens));
            }
        } else {
            // Anthropic 要求必须指定 max_tokens
            if let Some(map) = body.as_object_mut() {
                map.insert("max_tokens".to_string(), json!(4096));
            }
        }
        if let Some(t) = request.temperature {
            if let Some(map) = body.as_object_mut() {
                map.insert("temperature".to_string(), json!(t));
            }
        }

        debug!(
            provider = "anthropic",
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
            provider = "anthropic",
            status = %status,
            elapsed_ms,
            "provider.chat.http.done"
        );
        debug!(
            provider = "anthropic",
            status = %status,
            body = %preview(&data.to_string(), 1200),
            "provider.chat.response_body"
        );

        if !status.is_success() {
            return Err(ProviderError::Message(format!(
                "Anthropic 请求失败：status={} body={}",
                status, data
            )));
        }

        // Anthropic 响应格式
        let content = data.get("content").cloned().unwrap_or_else(|| json!([]));
        let text_content = Self::extract_text_content(&content);
        let tool_calls = Self::parse_tool_calls(&content);

        let usage = data.get("usage").map(|u| Usage {
            prompt_tokens: u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            completion_tokens: u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            total_tokens: u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32
                + u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        });

        let finish_reason = data
            .get("stop_reason")
            .and_then(|v| v.as_str())
            .unwrap_or("end_turn")
            .to_string();

        Ok(ChatResponse {
            message: ChatMessage {
                role: Role::Assistant,
                content: text_content,
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
            .unwrap_or_else(|| self.default_model.clone());

        let url = format!("{}/messages", self.base_url.trim_end_matches('/'));

        let system_prompt = Self::build_system_prompt(&request.messages);
        let messages = Self::build_messages(&request.messages);

        let mut body = json!({
            "model": model,
            "messages": messages,
            "stream": true,
        });

        if let Some(system) = system_prompt {
            if let Some(map) = body.as_object_mut() {
                map.insert("system".to_string(), json!(system));
            }
        }

        if let Some(tools) = request.tools.as_ref() {
            if let Some(map) = body.as_object_mut() {
                map.insert("tools".to_string(), Value::Array(Self::build_tools(tools)));
            }
        }
        if let Some(max_tokens) = request.max_tokens {
            if let Some(map) = body.as_object_mut() {
                map.insert("max_tokens".to_string(), json!(max_tokens));
            }
        } else {
            if let Some(map) = body.as_object_mut() {
                map.insert("max_tokens".to_string(), json!(4096));
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
                    "Anthropic 流式请求失败：status={}",
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

                    let json: Value = serde_json::from_str(&data)
                        .map_err(|e| ProviderError::Message(format!("SSE 解析失败：{} data={}", e, data)))?;

                    let event_type = json
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    if event_type == "content_block_delta" {
                        let delta = json
                            .get("delta")
                            .and_then(|d| d.get("text"))
                            .and_then(|v| v.as_str())
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

                if buf.contains("[DONE]") {
                    break;
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
    fn test_anthropic_provider_creation() {
        let provider = AnthropicProvider::with_api_key("test-key");
        assert_eq!(provider.base_url, "https://api.anthropic.com/v1");
        assert_eq!(provider.default_model(), ANTHROPIC_DEFAULT_MODEL);
    }

    #[test]
    fn test_anthropic_provider_with_custom_model() {
        let provider = AnthropicProvider::new("https://api.anthropic.com/v1", "test-key")
            .with_default_model("claude-3-5-sonnet-20241022");
        assert_eq!(provider.default_model(), "claude-3-5-sonnet-20241022");
    }

    #[test]
    fn test_anthropic_provider_with_model() {
        let provider = AnthropicProvider::with_model(
            "https://api.anthropic.com/v1",
            "test-key",
            "claude-3-opus-20240229",
        );
        assert_eq!(provider.default_model(), "claude-3-opus-20240229");
    }

    #[test]
    fn test_anthropic_default_model_constant() {
        assert_eq!(ANTHROPIC_DEFAULT_MODEL, "claude-3-5-sonnet-20241022");
    }
}


