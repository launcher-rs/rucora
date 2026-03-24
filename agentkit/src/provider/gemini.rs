//! Google Gemini Provider 实现。
//!
//! 约定：
//! - API Key 从 `GOOGLE_API_KEY` 或 `GEMINI_API_KEY` 环境变量读取
//! - Base URL 默认 `https://generativelanguage.googleapis.com/v1beta`
//! - 默认模型为 `gemini-1.5-flash`
//! - 使用 Google Generative AI API 格式
//! - 支持 Gemini 系列模型（gemini-1.5-pro, gemini-1.5-flash, gemini-pro 等）
//!
//! # 默认模型优先级
//!
//! 1. 手动设置：通过 `with_default_model()` 方法设置
//! 2. 环境变量：从 `GEMINI_DEFAULT_MODEL` 环境变量读取
//! 3. 内置默认值：`gemini-1.5-flash`

use std::env;

use crate::provider::helpers::parse_finish_reason;
use crate::provider::preview;
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

/// Gemini 默认模型名称。
pub const GEMINI_DEFAULT_MODEL: &str = "gemini-1.5-flash";

/// Google Gemini Provider。
///
/// 支持 Gemini 系列模型：
/// - gemini-1.5-pro
/// - gemini-1.5-flash
/// - gemini-1.0-pro
/// - gemini-pro
///
/// # 使用示例
///
/// ```rust,no_run
/// use agentkit::provider::GeminiProvider;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // 从环境变量加载（包括 GEMINI_DEFAULT_MODEL）
/// let provider = GeminiProvider::from_env()?;
///
/// // 或手动配置（使用内置默认模型 gemini-1.5-flash）
/// let provider = GeminiProvider::new("your-api-key");
///
/// // 使用特定模型
/// let provider = provider.with_default_model("gemini-1.5-pro");
/// # Ok(())
/// # }
/// ```
///
/// # 环境变量
///
/// | 变量名 | 说明 | 示例 |
/// |--------|------|------|
/// | `GOOGLE_API_KEY` | Google API Key | `...` |
/// | `GEMINI_API_KEY` | Gemini API Key（备选） | `...` |
/// | `GOOGLE_BASE_URL` | Google API Base URL | `https://generativelanguage.googleapis.com/v1beta` |
/// | `GEMINI_DEFAULT_MODEL` | 默认模型名称 | `gemini-1.5-pro` |
///
/// # 默认模型优先级
///
/// 1. 手动设置：通过 `with_default_model()` 方法设置
/// 2. 环境变量：从 `GEMINI_DEFAULT_MODEL` 环境变量读取
/// 3. 内置默认值：`gemini-1.5-flash`
pub struct GeminiProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    default_model: String,
}

impl GeminiProvider {
    /// 从环境变量创建 Provider。
    ///
    /// # 环境变量优先级
    ///
    /// 1. `GOOGLE_API_KEY` 或 `GEMINI_API_KEY` - API Key
    /// 2. `GOOGLE_BASE_URL` - Base URL（可选，默认 `https://generativelanguage.googleapis.com/v1beta`）
    /// 3. `GEMINI_DEFAULT_MODEL` - 默认模型（可选，默认 `gemini-1.5-flash`）
    pub fn from_env() -> Result<Self, ProviderError> {
        let api_key = env::var("GOOGLE_API_KEY")
            .or_else(|_| env::var("GEMINI_API_KEY"))
            .map_err(|_| {
                ProviderError::Message("缺少环境变量 GOOGLE_API_KEY 或 GEMINI_API_KEY".to_string())
            })?;
        let base_url = env::var("GOOGLE_BASE_URL")
            .unwrap_or_else(|_| "https://generativelanguage.googleapis.com/v1beta".to_string());
        let default_model =
            env::var("GEMINI_DEFAULT_MODEL").unwrap_or_else(|_| GEMINI_DEFAULT_MODEL.to_string());

        Ok(Self::with_model(base_url, api_key, default_model))
    }

    /// 创建 Provider。
    ///
    /// 使用内置默认模型 `gemini-1.5-flash`。
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self::with_model(base_url, api_key, GEMINI_DEFAULT_MODEL.to_string())
    }

    /// 创建 Provider 并指定默认模型。
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

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .expect("reqwest client build failed");

        Self {
            client,
            base_url: base_url.into(),
            api_key,
            default_model: default_model.into(),
        }
    }

    /// 仅使用 API Key 创建 Provider（使用默认 base_url 和默认模型）。
    pub fn with_api_key(api_key: impl Into<String>) -> Self {
        Self::with_model(
            "https://generativelanguage.googleapis.com/v1beta",
            api_key,
            GEMINI_DEFAULT_MODEL.to_string(),
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

    fn map_role(role: &Role) -> &'static str {
        match role {
            Role::System => "model", // Gemini 没有 system role，用 model 替代
            Role::User => "user",
            Role::Assistant => "model",
            Role::Tool => "user",
        }
    }

    fn build_system_instruction(messages: &[ChatMessage]) -> Option<String> {
        messages
            .iter()
            .find(|m| m.role == Role::System)
            .map(|m| m.content.clone())
    }

    fn build_messages(messages: &[ChatMessage]) -> Vec<Value> {
        messages
            .iter()
            .filter(|m| m.role != Role::System) // System instruction 单独处理
            .map(|m| {
                let role = Self::map_role(&m.role);
                json!({
                    "role": role,
                    "parts": [{
                        "text": m.content
                    }]
                })
            })
            .collect()
    }

    fn build_tools(tools: &[ToolDefinition]) -> Value {
        // Gemini 工具格式
        let function_declarations = tools
            .iter()
            .map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.input_schema,
                })
            })
            .collect::<Vec<_>>();

        json!([{
            "functionDeclarations": function_declarations
        }])
    }

    fn parse_tool_calls(content: &Value) -> Vec<ToolCall> {
        let mut out = Vec::new();

        // Gemini 的 functionCall 在 parts 数组中
        if let Some(parts) = content.get("parts").and_then(|v| v.as_array()) {
            for part in parts {
                if let Some(fn_call) = part.get("functionCall") {
                    let name = fn_call
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    // Gemini 使用 name 作为 id
                    let id = name.clone();
                    let args = fn_call.get("args").cloned().unwrap_or_else(|| json!({}));

                    if !name.is_empty() {
                        out.push(ToolCall {
                            id,
                            name,
                            input: args,
                        });
                    }
                }
            }
        }

        out
    }

    fn extract_text_content(content: &Value) -> String {
        // Gemini 返回的 content 包含 parts 数组
        if let Some(parts) = content.get("parts").and_then(|v| v.as_array()) {
            let mut texts = Vec::new();
            for part in parts {
                if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                    texts.push(text);
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
impl LlmProvider for GeminiProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let model = request
            .model
            .clone()
            .unwrap_or_else(|| self.default_model.clone());

        // Gemini URL 格式：{base_url}/models/{model}:generateContent?key={api_key}
        let url = format!(
            "{}/models/{}:generateContent?key={}",
            self.base_url.trim_end_matches('/'),
            model,
            self.api_key
        );

        let system_instruction = Self::build_system_instruction(&request.messages);
        let messages = Self::build_messages(&request.messages);

        let last_user_preview = request
            .messages
            .iter()
            .rev()
            .find(|m| m.role == Role::User)
            .map(|m| preview(&m.content, 600));

        debug!(
            provider = "gemini",
            url = %url,
            model = %model,
            messages_len = messages.len(),
            tools_len = request.tools.as_ref().map(|t| t.len()).unwrap_or(0),
            last_user = last_user_preview.as_deref().unwrap_or(""),
            "provider.chat.start"
        );

        let mut body = json!({
            "contents": messages,
        });

        // Gemini 支持 systemInstruction 作为顶层字段
        if let Some(instruction) = system_instruction {
            if let Some(map) = body.as_object_mut() {
                map.insert(
                    "systemInstruction".to_string(),
                    json!({
                        "parts": [{
                            "text": instruction
                        }]
                    }),
                );
            }
        }

        if let Some(tools) = request.tools.as_ref() {
            if let Some(map) = body.as_object_mut() {
                map.insert("tools".to_string(), Self::build_tools(tools));
            }
        }

        // Gemini 配置
        let mut generation_config = json!({});
        if let Some(t) = request.temperature {
            if let Some(map) = generation_config.as_object_mut() {
                map.insert("temperature".to_string(), json!(t));
            }
        }
        if let Some(max_tokens) = request.max_tokens {
            if let Some(map) = generation_config.as_object_mut() {
                map.insert("maxOutputTokens".to_string(), json!(max_tokens));
            }
        }
        if let Some(map) = body.as_object_mut() {
            map.insert("generationConfig".to_string(), generation_config);
        }

        debug!(
            provider = "gemini",
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
            provider = "gemini",
            status = %status,
            elapsed_ms,
            "provider.chat.http.done"
        );
        debug!(
            provider = "gemini",
            status = %status,
            body = %preview(&data.to_string(), 1200),
            "provider.chat.response_body"
        );

        if !status.is_success() {
            return Err(ProviderError::Message(format!(
                "Gemini 请求失败：status={} body={}",
                status, data
            )));
        }

        // Gemini 响应格式
        let content = data
            .get("candidates")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|c| c.get("content"))
            .cloned()
            .unwrap_or_else(|| json!({}));

        let text_content = Self::extract_text_content(&content);
        let tool_calls = Self::parse_tool_calls(&content);

        // Gemini 使用 tokenMetadata 统计
        let usage_metadata = data.get("usageMetadata");
        let usage = usage_metadata.map(|u| Usage {
            prompt_tokens: u
                .get("promptTokenCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32,
            completion_tokens: u
                .get("candidatesTokenCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32,
            total_tokens: u
                .get("totalTokenCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32,
        });

        let finish_reason = data
            .get("candidates")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|c| c.get("finishReason"))
            .and_then(|v| v.as_str())
            .unwrap_or("STOP")
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

        // Gemini 流式 URL
        let url = format!(
            "{}/models/{}:streamGenerateContent?alt=sse&key={}",
            self.base_url.trim_end_matches('/'),
            model,
            self.api_key
        );

        let system_instruction = Self::build_system_instruction(&request.messages);
        let messages = Self::build_messages(&request.messages);

        let mut body = json!({
            "contents": messages,
        });

        if let Some(instruction) = system_instruction {
            if let Some(map) = body.as_object_mut() {
                map.insert(
                    "systemInstruction".to_string(),
                    json!({
                        "parts": [{
                            "text": instruction
                        }]
                    }),
                );
            }
        }

        if let Some(tools) = request.tools.as_ref() {
            if let Some(map) = body.as_object_mut() {
                map.insert("tools".to_string(), Self::build_tools(tools));
            }
        }

        let mut generation_config = json!({});
        if let Some(t) = request.temperature {
            if let Some(map) = generation_config.as_object_mut() {
                map.insert("temperature".to_string(), json!(t));
            }
        }
        if let Some(max_tokens) = request.max_tokens {
            if let Some(map) = generation_config.as_object_mut() {
                map.insert("maxOutputTokens".to_string(), json!(max_tokens));
            }
        }
        if let Some(map) = body.as_object_mut() {
            map.insert("generationConfig".to_string(), generation_config);
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
                    "Gemini 流式请求失败：status={}",
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

                    // Gemini SSE 格式：每行一个 JSON 对象
                    for line in event.lines() {
                        let trimmed = line.trim();
                        if trimmed.is_empty() || trimmed.starts_with('[') {
                            continue;
                        }

                        let json: Value = serde_json::from_str(trimmed)
                            .map_err(|e| ProviderError::Message(format!("SSE 解析失败：{} data={}", e, trimmed)))?;

                        let content = json
                            .get("candidates")
                            .and_then(|v| v.as_array())
                            .and_then(|arr| arr.first())
                            .and_then(|c| c.get("content"))
                            .cloned()
                            .unwrap_or_else(|| json!({}));

                        let delta = Self::extract_text_content(&content);
                        if !delta.is_empty() {
                            yield ChatStreamChunk {
                                delta: Some(delta),
                                tool_calls: vec![],
                                usage: None,
                                finish_reason: None,
                            };
                        }
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
    fn test_gemini_provider_creation() {
        let provider = GeminiProvider::with_api_key("test-key");
        assert_eq!(
            provider.base_url,
            "https://generativelanguage.googleapis.com/v1beta"
        );
        assert_eq!(provider.default_model(), GEMINI_DEFAULT_MODEL);
    }

    #[test]
    fn test_gemini_provider_with_custom_model() {
        let provider = GeminiProvider::with_model(
            "https://generativelanguage.googleapis.com/v1beta",
            "test-key",
            "gemini-1.5-pro",
        );
        assert_eq!(provider.default_model(), "gemini-1.5-pro");
    }

    #[test]
    fn test_gemini_provider_with_default_model_builder() {
        let provider =
            GeminiProvider::with_api_key("test-key").with_default_model("gemini-1.5-pro");
        assert_eq!(provider.default_model(), "gemini-1.5-pro");
    }

    #[test]
    fn test_gemini_default_model_constant() {
        assert_eq!(GEMINI_DEFAULT_MODEL, "gemini-1.5-flash");
    }
}
