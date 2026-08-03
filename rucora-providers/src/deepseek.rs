//! DeepSeek Provider 实现。
//!
//! 约定：
//! - API Key 从 `DEEPSEEK_API_KEY` 环境变量读取
//! - Base URL 默认 `https://api.deepseek.com/v1`
//! - 使用 OpenAI 兼容的 API 格式
//! - 支持 DeepSeek 系列模型（DeepSeek-V3, DeepSeek-R1 等）

use std::{collections::BTreeMap, env};

use crate::{
    helpers::{apply_sampling_params, map_http_error, map_reqwest_error, parse_finish_reason},
    http_config::{
        build_client, build_client_with_timeout, DEFAULT_CONNECT_TIMEOUT_SECS,
        DEFAULT_REQUEST_TIMEOUT_SECS,
    },
    preview,
};
use async_trait::async_trait;
use futures_util::{StreamExt, stream::BoxStream};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use rucora_core::{
    error::ProviderError,
    provider::{
        LlmProvider,
        types::{
            ChatMessage, ChatRequest, ChatResponse, ChatStreamChunk, FinishReason, ResponseFormat,
            Role, Usage,
        },
    },
    tool::types::{ToolCall, ToolDefinition},
};
use serde_json::{Value, json};
use tracing::debug;

/// DeepSeek 默认模型。
pub const DEEPSEEK_DEFAULT_MODEL: &str = "deepseek-chat";

/// DeepSeek Provider。
///
/// 支持 DeepSeek 系列模型：
/// - deepseek-chat (DeepSeek-V3)
/// - deepseek-reasoner (DeepSeek-R1)
///
/// # 使用示例
///
/// ```rust,no_run
/// use rucora_providers::DeepSeekProvider;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // 从环境变量加载
/// let provider = DeepSeekProvider::from_env()?;
///
/// // 或手动配置
/// let provider = DeepSeekProvider::with_api_key("sk-...");
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
/// | `DEEPSEEK_DEFAULT_MODEL` | 默认模型 | `deepseek-chat` |
#[derive(Clone)]
pub struct DeepSeekProvider {
    client: reqwest::Client,
    headers: HeaderMap,
    base_url: String,
    default_model: String,
    request_timeout_secs: Option<u64>,
    connect_timeout_secs: Option<u64>,
}

impl DeepSeekProvider {
    fn build_headers(api_key: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Ok(v) = HeaderValue::from_str(&format!("Bearer {api_key}")) {
            headers.insert(AUTHORIZATION, v);
        }
        headers
    }

    fn build_http_client(headers: &HeaderMap, request_timeout_secs: Option<u64>, connect_timeout_secs: Option<u64>) -> reqwest::Client {
        match (request_timeout_secs, connect_timeout_secs) {
            (Some(rt), Some(ct)) => build_client_with_timeout(headers.clone(), rt, ct),
            (Some(rt), None) => build_client_with_timeout(headers.clone(), rt, DEFAULT_CONNECT_TIMEOUT_SECS),
            (None, Some(ct)) => build_client_with_timeout(headers.clone(), DEFAULT_REQUEST_TIMEOUT_SECS, ct),
            (None, None) => build_client(headers.clone()),
        }
    }

    /// 从环境变量创建 Provider。
    ///
    /// 默认模型优先级：
    /// 1. `DEEPSEEK_DEFAULT_MODEL` 环境变量
    /// 2. 内置默认值 `deepseek-chat`
    pub fn from_env() -> Result<Self, ProviderError> {
        let api_key = env::var("DEEPSEEK_API_KEY")
            .map_err(|_| ProviderError::Message("缺少环境变量 DEEPSEEK_API_KEY".to_string()))?;
        let base_url = env::var("DEEPSEEK_BASE_URL")
            .unwrap_or_else(|_| "https://api.deepseek.com/v1".to_string());
        let default_model = env::var("DEEPSEEK_DEFAULT_MODEL")
            .unwrap_or_else(|_| DEEPSEEK_DEFAULT_MODEL.to_string());

        Ok(Self::with_model(base_url, api_key, default_model))
    }

    /// 创建 Provider。
    ///
    /// 使用内置默认模型 `deepseek-chat`。
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self::with_model(base_url, api_key, DEEPSEEK_DEFAULT_MODEL.to_string())
    }

    /// 创建 Provider 并指定默认模型。
    pub fn with_model(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        default_model: impl Into<String>,
    ) -> Self {
        let api_key = api_key.into();
        let headers = Self::build_headers(&api_key);
        let client = Self::build_http_client(&headers, None, None);

        Self {
            client,
            headers,
            base_url: base_url.into(),
            default_model: default_model.into(),
            request_timeout_secs: None,
            connect_timeout_secs: None,
        }
    }

    /// 仅使用 API Key 创建 Provider（使用默认 base_url 和默认模型）。
    pub fn with_api_key(api_key: impl Into<String>) -> Self {
        Self::with_model(
            "https://api.deepseek.com/v1",
            api_key,
            DEEPSEEK_DEFAULT_MODEL.to_string(),
        )
    }

    /// 设置默认模型。
    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    pub fn with_request_timeout(mut self, secs: Option<u64>) -> Self {
        self.request_timeout_secs = secs;
        self.client = Self::build_http_client(&self.headers, self.request_timeout_secs, self.connect_timeout_secs);
        self
    }

    pub fn with_connect_timeout(mut self, secs: Option<u64>) -> Self {
        self.connect_timeout_secs = secs;
        self.client = Self::build_http_client(&self.headers, self.request_timeout_secs, self.connect_timeout_secs);
        self
    }

    pub fn with_client(mut self, client: reqwest::Client) -> Self {
        self.client = client;
        self
    }

    /// 获取默认模型。
    pub fn default_model(&self) -> &str {
        &self.default_model
    }

    fn build_messages(messages: &[ChatMessage]) -> Vec<Value> {
        crate::helpers::build_openai_messages(messages)
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
impl LlmProvider for DeepSeekProvider {
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
            .map(|m| preview(m.content_text(), 600));

        debug!(
            provider = "deepseek",
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
            .map_err(|e| map_reqwest_error(e, start.elapsed()))?;

        let status = resp.status();
        let data: Value = resp
            .json()
            .await
            .map_err(|e| map_reqwest_error(e, start.elapsed()))?;

        let elapsed_ms = start.elapsed().as_millis() as u64;
        debug!(
            provider = "deepseek",
            status = %status,
            elapsed_ms,
            "provider.chat.http.done"
        );
        debug!(
            provider = "deepseek",
            status = %status,
            body = %preview(&data.to_string(), 1200),
            "provider.chat.response_body"
        );

        if !status.is_success() {
            return Err(map_http_error(
                status,
                format!("DeepSeek 请求失败：status={status} body={data}"),
            ));
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
            message: ChatMessage::assistant_with_tool_calls(content, tool_calls),
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
                .map_err(|e| map_reqwest_error(e, std::time::Duration::ZERO))?;

            let status = resp.status();
            if !status.is_success() {
                Err(map_http_error(
                    status,
                    format!("DeepSeek 流式请求失败：status={status}"),
                ))?;
            }

            let mut buf = String::new();
            let mut bytes_stream = resp.bytes_stream();
            let mut tool_call_parts: BTreeMap<usize, (String, String, String)> = BTreeMap::new();
            let mut done = false;

            while let Some(item) = bytes_stream.next().await {
                let bytes = item.map_err(|e| ProviderError::Message(e.to_string()))?;
                let chunk = String::from_utf8_lossy(&bytes);
                buf.push_str(&chunk);

                while let Some(idx) = buf.find("\r\n\r\n").or_else(|| buf.find("\n\n")) {
                    let sep_len = if buf[idx..].starts_with("\r\n\r\n") { 4 } else { 2 };
                    let event = buf.drain(..idx + sep_len).collect::<String>();
                    let event = event.trim_end_matches('\n').trim_end_matches('\r');

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
                        done = true;
                        break;
                    }

                    let v: Value = serde_json::from_str(&data)
                        .map_err(|e| ProviderError::Message(format!("SSE 解析失败：{e} data={data}")))?;

                    let choice = v
                        .get("choices")
                        .and_then(|c| c.as_array())
                        .and_then(|arr| arr.first());
                    let delta_obj = choice.and_then(|c0| c0.get("delta"));

                    let delta = delta_obj
                        .and_then(|d| d.get("content"))
                        .and_then(|s| s.as_str())
                        .map(|s| s.to_string());

                    // 增量累积工具调用碎片（工具参数可能是分片 JSON）
                    if let Some(tool_calls) = delta_obj
                        .and_then(|d| d.get("tool_calls"))
                        .and_then(|tc| tc.as_array())
                    {
                        for item in tool_calls {
                            let index = item.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                            let entry = tool_call_parts
                                .entry(index)
                                .or_insert_with(|| (String::new(), String::new(), String::new()));

                            if let Some(id) = item.get("id").and_then(|v| v.as_str())
                                && !id.is_empty()
                            {
                                entry.0 = id.to_string();
                            }

                            if let Some(function) = item.get("function") {
                                if let Some(name) = function.get("name").and_then(|v| v.as_str()) {
                                    entry.1.push_str(name);
                                }
                                if let Some(arguments) = function.get("arguments").and_then(|v| v.as_str()) {
                                    entry.2.push_str(arguments);
                                }
                            }
                        }
                    }

                    let finish_reason = choice
                        .and_then(|c0| c0.get("finish_reason"))
                        .and_then(|fr| fr.as_str())
                        .map(parse_finish_reason);

                    if delta.is_some() {
                        yield ChatStreamChunk {
                            delta,
                            tool_calls: vec![],
                            usage: None,
                            finish_reason,
                        };
                    }

                    if matches!(finish_reason, Some(FinishReason::ToolCall)) {
                        let tool_calls = tool_call_parts
                            .values()
                            .filter_map(|(id, name, args_raw)| {
                                if name.is_empty() {
                                    return None;
                                }
                                let input = serde_json::from_str(args_raw)
                                    .unwrap_or_else(|_| Value::String(args_raw.clone()));
                                Some(ToolCall {
                                    id: id.clone(),
                                    name: name.clone(),
                                    input,
                                })
                            })
                            .collect::<Vec<_>>();

                        if !tool_calls.is_empty() {
                            yield ChatStreamChunk {
                                delta: None,
                                tool_calls,
                                usage: None,
                                finish_reason,
                            };
                        }
                    }
                }

                if done {
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
    fn test_deepseek_provider_creation() {
        let provider = DeepSeekProvider::with_api_key("test-key");
        assert_eq!(provider.base_url, "https://api.deepseek.com/v1");
        assert_eq!(provider.default_model(), DEEPSEEK_DEFAULT_MODEL);
    }

    #[test]
    fn test_deepseek_provider_with_custom_model() {
        let provider = DeepSeekProvider::with_model(
            "https://api.deepseek.com/v1",
            "test-key",
            "deepseek-chat",
        );
        assert_eq!(provider.default_model(), "deepseek-chat");
    }
}
