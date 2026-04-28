//! OpenRouter Provider 实现。
//!
//! 约定：
//! - API Key 从 `OPENROUTER_API_KEY` 环境变量读取
//! - Base URL 默认 `https://openrouter.ai/api/v1`
//! - 支持 OpenAI 兼容的 API 格式
//! - 支持额外的 OpenRouter 特定功能（如路由策略、提供商偏好等）

use std::env;

use crate::{
    helpers::{apply_sampling_params, parse_finish_reason},
    http_config::build_client,
    preview,
};
use rucora_core::{
    error::ProviderError,
    provider::{
        LlmProvider,
        types::{
            ChatMessage, ChatRequest, ChatResponse, ChatStreamChunk, ResponseFormat, Role, Usage,
        },
    },
    tool::types::{ToolCall, ToolDefinition},
};
use async_trait::async_trait;
use futures_util::{StreamExt, stream::BoxStream};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::{Value, json};
use tracing::debug;

/// OpenRouter 默认模型。
pub const OPENROUTER_DEFAULT_MODEL: &str = "anthropic/claude-3-5-sonnet";

/// OpenRouter Provider。
///
/// OpenRouter 是一个聚合了多种模型提供商的 API 服务，支持：
/// - OpenAI (GPT-4, GPT-3.5)
/// - Anthropic (Claude)
/// - Google (Gemini)
/// - Meta (Llama)
/// - Mistral
/// - 以及更多...
///
/// # 使用示例
///
/// ```rust,no_run
/// use rucora::provider::OpenRouterProvider;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // 从环境变量加载
/// let provider = OpenRouterProvider::from_env()?;
///
/// // 或手动配置
/// let provider = OpenRouterProvider::new("sk-or-...");
///
/// // 使用特定模型
/// let provider = provider.with_default_model("anthropic/claude-3.5-sonnet");
/// # Ok(())
/// # }
/// ```
///
/// # 环境变量
///
/// | 变量名 | 说明 | 示例 |
/// |--------|------|------|
/// | `OPENROUTER_API_KEY` | OpenRouter API Key | `sk-or-...` |
/// | `OPENROUTER_BASE_URL` | OpenRouter Base URL | `https://openrouter.ai/api/v1` |
/// | `OPENROUTER_SITE_URL` | 网站 URL（可选） | `https://your-app.com` |
/// | `OPENROUTER_SITE_NAME` | 网站名称（可选） | `Your App` |
/// | `OPENROUTER_DEFAULT_MODEL` | 默认模型 | `anthropic/claude-3-5-sonnet` |
pub struct OpenRouterProvider {
    client: reqwest::Client,
    base_url: String,
    default_model: String,
    site_url: Option<String>,
    site_name: Option<String>,
}

impl OpenRouterProvider {
    /// 从环境变量创建 Provider。
    ///
    /// 默认模型优先级：
    /// 1. `OPENROUTER_DEFAULT_MODEL` 环境变量
    /// 2. 内置默认值 `anthropic/claude-3-5-sonnet`
    pub fn from_env() -> Result<Self, ProviderError> {
        let api_key = env::var("OPENROUTER_API_KEY")
            .map_err(|_| ProviderError::Message("缺少环境变量 OPENROUTER_API_KEY".to_string()))?;
        let base_url = env::var("OPENROUTER_BASE_URL")
            .unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string());
        let site_url = env::var("OPENROUTER_SITE_URL").ok();
        let site_name = env::var("OPENROUTER_SITE_NAME").ok();
        let default_model = env::var("OPENROUTER_DEFAULT_MODEL")
            .unwrap_or_else(|_| OPENROUTER_DEFAULT_MODEL.to_string());

        let mut provider = Self::with_model(base_url, api_key, default_model);
        if let Some(url) = site_url {
            provider = provider.with_site_url(url);
        }
        if let Some(name) = site_name {
            provider = provider.with_site_name(name);
        }

        Ok(provider)
    }

    /// 创建 Provider。
    ///
    /// 使用内置默认模型 `anthropic/claude-3-5-sonnet`。
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self::with_model(base_url, api_key, OPENROUTER_DEFAULT_MODEL.to_string())
    }

    /// 创建 Provider 并指定默认模型。
    pub fn with_model(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        default_model: impl Into<String>,
    ) -> Self {
        let api_key = api_key.into();
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Ok(v) = HeaderValue::from_str(&format!("Bearer {api_key}")) {
            headers.insert(AUTHORIZATION, v);
        }

        let client = build_client(headers);

        Self {
            client,
            base_url: base_url.into(),
            default_model: default_model.into(),
            site_url: None,
            site_name: None,
        }
    }

    /// 仅使用 API Key 创建 Provider（使用默认 base_url 和默认模型）。
    pub fn with_api_key(api_key: impl Into<String>) -> Self {
        Self::with_model(
            "https://openrouter.ai/api/v1",
            api_key,
            OPENROUTER_DEFAULT_MODEL.to_string(),
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

    /// 设置网站 URL（用于 OpenRouter 统计）。
    pub fn with_site_url(mut self, url: impl Into<String>) -> Self {
        self.site_url = Some(url.into());
        self
    }

    /// 设置网站名称（用于 OpenRouter 统计）。
    pub fn with_site_name(mut self, name: impl Into<String>) -> Self {
        self.site_name = Some(name.into());
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
                if let Some(name) = &m.name
                    && let Some(map) = obj.as_object_mut()
                {
                    map.insert("name".to_string(), Value::String(name.clone()));
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
                if let Some(strict) = strict
                    && let Some(root) = obj.as_object_mut()
                    && let Some(js) = root.get_mut("json_schema").and_then(|v| v.as_object_mut())
                {
                    js.insert("strict".to_string(), json!(strict));
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
impl LlmProvider for OpenRouterProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
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
            provider = "openrouter",
            url = %url,
            model = %model,
            messages_len = request.messages.len(),
            tools_len = request.tools.as_ref().map_or(0, |t| t.len()),
            last_user = last_user_preview.as_deref().unwrap_or(""),
            "provider.chat.start"
        );

        let mut body = json!({
            "model": model,
            "messages": messages,
        });

        // 添加 OpenRouter 特定的 metadata
        if self.site_url.is_some() || self.site_name.is_some() {
            let mut metadata = json!({});
            if let Some(url) = &self.site_url
                && let Some(map) = metadata.as_object_mut()
            {
                map.insert("site_url".to_string(), json!(url));
            }
            if let Some(name) = &self.site_name
                && let Some(map) = metadata.as_object_mut()
            {
                map.insert("site_name".to_string(), json!(name));
            }
            if let Some(map) = body.as_object_mut() {
                map.insert("metadata".to_string(), metadata);
            }
        }

        if let Some(tools) = request.tools.as_ref()
            && let Some(map) = body.as_object_mut()
        {
            map.insert("tools".to_string(), Value::Array(Self::build_tools(tools)));
        }
        if let Some(map) = body.as_object_mut() {
            apply_sampling_params(
                map,
                request.temperature,
                request.top_p,
                request.top_k,
                request.max_tokens,
                request.frequency_penalty,
                request.presence_penalty,
                request.stop.as_ref(),
                request.extra.as_ref(),
            );
        }
        if let Some(fmt) = request.response_format.as_ref()
            && let Some(map) = body.as_object_mut()
        {
            map.insert(
                "response_format".to_string(),
                Self::build_response_format(fmt),
            );
        }

        debug!(
            provider = "openrouter",
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
            provider = "openrouter",
            status = %status,
            elapsed_ms,
            "provider.chat.http.done"
        );
        debug!(
            provider = "openrouter",
            status = %status,
            body = %preview(&data.to_string(), 1200),
            "provider.chat.response_body"
        );

        if !status.is_success() {
            return Err(ProviderError::Message(format!(
                "OpenRouter 请求失败：status={status} body={data}"
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
            .unwrap_or_else(|| self.default_model.clone());

        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let messages = Self::build_messages(&request.messages);

        let mut body = json!({
            "model": model,
            "messages": messages,
            "stream": true,
        });

        if let Some(tools) = request.tools.as_ref()
            && let Some(map) = body.as_object_mut()
        {
            map.insert("tools".to_string(), Value::Array(Self::build_tools(tools)));
        }
        if let Some(map) = body.as_object_mut() {
            apply_sampling_params(
                map,
                request.temperature,
                request.top_p,
                request.top_k,
                request.max_tokens,
                request.frequency_penalty,
                request.presence_penalty,
                request.stop.as_ref(),
                request.extra.as_ref(),
            );
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
                    "OpenRouter 流式请求失败：status={status}"
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
                        .map_err(|e| ProviderError::Message(format!("SSE 解析失败：{e} data={data}")))?;

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
    fn test_openrouter_provider_creation() {
        let provider = OpenRouterProvider::with_api_key("test-key");
        assert_eq!(provider.base_url, "https://openrouter.ai/api/v1");
        assert_eq!(provider.default_model(), OPENROUTER_DEFAULT_MODEL);
    }

    #[test]
    fn test_openrouter_provider_with_custom_model() {
        let provider = OpenRouterProvider::with_model(
            "https://custom.openrouter.ai/api/v1",
            "test-key",
            "anthropic/claude-3.5-sonnet",
        );
        assert_eq!(provider.default_model(), "anthropic/claude-3.5-sonnet");
    }
}
