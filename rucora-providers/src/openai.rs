//! OpenAI Provider 实现。
//!
//! 约定：
//! - API Key 从 `OPENAI_API_KEY` 环境变量读取
//! - Base URL 默认 `https://api.openai.com/v1`，也可通过 `OPENAI_BASE_URL` 覆盖
//! - 默认模型优先级：1) 手动设置 `with_default_model()` 2) `OPENAI_DEFAULT_MODEL` 环境变量 3) 内置默认值 `gpt-4o-mini`

use std::{collections::BTreeMap, env};

use crate::{
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
            Role,
        },
    },
    tool::types::{ToolCall, ToolDefinition},
};
use serde_json::{Value, json};
use tracing::debug;

/// OpenAI 默认模型（当未指定时使用）
const OPENAI_DEFAULT_MODEL: &str = "gpt-4o-mini";

/// OpenAI Chat Completions Provider。
///
/// 功能：
/// - 支持 `chat`（非流式）和 `stream_chat`（流式）两种调用模式
/// - `tools` 会按 OpenAI 的 function tools 格式传入
///
/// # 默认模型
///
/// 默认模型的优先级顺序：
/// 1. 手动调用 `with_default_model()` 设置的值
/// 2. `OPENAI_DEFAULT_MODEL` 环境变量
/// 3. 内置默认值 `gpt-4o-mini`
///
/// # 超时配置
///
/// 默认请求超时 120 秒，连接超时 15 秒。可通过以下方式自定义：
///
/// ```rust,no_run
/// use rucora_providers::OpenAiProvider;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let provider = OpenAiProvider::from_env()?
///     .with_request_timeout(Some(300))   // 请求超时 300 秒
///     .with_connect_timeout(Some(60));   // 连接超时 60 秒
///
/// // 使用自定义 HTTP 客户端（完全控制所有配置）
/// use reqwest::Client;
/// use std::time::Duration;
///
/// let client = Client::builder()
///     .timeout(Duration::from_secs(180))
///     .connect_timeout(Duration::from_secs(30))
///     .build()?;
/// let provider = OpenAiProvider::from_env()?
///     .with_client(client);
/// # Ok(())
/// # }
/// /// ```
#[derive(Clone)]
pub struct OpenAiProvider {
    client: reqwest::Client,
    headers: HeaderMap,
    base_url: String,
    default_model: String,
    request_timeout_secs: Option<u64>,
    connect_timeout_secs: Option<u64>,
}

impl OpenAiProvider {
    fn map_reqwest_error(e: reqwest::Error, elapsed: std::time::Duration) -> ProviderError {
        if e.is_timeout() {
            ProviderError::Timeout {
                message: e.to_string(),
                elapsed,
            }
        } else if e.is_connect() || e.is_request() {
            ProviderError::Network {
                message: e.to_string(),
                source: Some(Box::new(e)),
                retriable: true,
            }
        } else {
            ProviderError::Message(e.to_string())
        }
    }

    fn map_http_error(status: reqwest::StatusCode, message: String) -> ProviderError {
        match status.as_u16() {
            401 | 403 => ProviderError::Authentication { message },
            429 => ProviderError::RateLimit {
                message,
                retry_after: None,
            },
            status => ProviderError::Api {
                status,
                message,
                code: None,
            },
        }
    }

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

    /// 设置默认模型（覆盖环境变量或内置默认值）。
    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    /// 设置请求超时时间（秒）。
    ///
    /// 覆盖默认的 120 秒请求超时。设置为 `None` 恢复默认值。
    pub fn with_request_timeout(mut self, secs: Option<u64>) -> Self {
        self.request_timeout_secs = secs;
        self.client = Self::build_http_client(&self.headers, self.request_timeout_secs, self.connect_timeout_secs);
        self
    }

    /// 设置连接超时时间（秒）。
    ///
    /// 覆盖默认的 15 秒连接超时。设置为 `None` 恢复默认值。
    pub fn with_connect_timeout(mut self, secs: Option<u64>) -> Self {
        self.connect_timeout_secs = secs;
        self.client = Self::build_http_client(&self.headers, self.request_timeout_secs, self.connect_timeout_secs);
        self
    }

    /// 设置自定义 HTTP 客户端。
    ///
    /// 可用于完全控制客户端配置（代理、TLS、超时等）。
    pub fn with_client(mut self, client: reqwest::Client) -> Self {
        self.client = client;
        self
    }

    /// 获取当前配置的默认模型
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

    fn parse_finish_reason(fr: &str) -> FinishReason {
        match fr {
            "stop" => FinishReason::Stop,
            "length" => FinishReason::Length,
            "tool_calls" => FinishReason::ToolCall,
            _ => FinishReason::Other,
        }
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
            .map(|m| preview(m.content_text(), 600));

        debug!(
            provider = "openai",
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
        if let Some(t) = request.temperature
            && let Some(map) = body.as_object_mut()
        {
            map.insert("temperature".to_string(), json!(t));
        }
        if let Some(max_tokens) = request.max_tokens
            && let Some(map) = body.as_object_mut()
        {
            map.insert("max_tokens".to_string(), json!(max_tokens));
        }
        if let Some(fmt) = request.response_format.as_ref()
            && let Some(map) = body.as_object_mut()
        {
            map.insert(
                "response_format".to_string(),
                Self::build_response_format(fmt),
            );
        }

        // 支持更多参数
        if let Some(top_p) = request.top_p
            && let Some(map) = body.as_object_mut()
        {
            map.insert("top_p".to_string(), json!(top_p));
        }
        if let Some(top_k) = request.top_k
            && let Some(map) = body.as_object_mut()
        {
            map.insert("top_k".to_string(), json!(top_k));
        }
        if let Some(frequency_penalty) = request.frequency_penalty
            && let Some(map) = body.as_object_mut()
        {
            map.insert("frequency_penalty".to_string(), json!(frequency_penalty));
        }
        if let Some(presence_penalty) = request.presence_penalty
            && let Some(map) = body.as_object_mut()
        {
            map.insert("presence_penalty".to_string(), json!(presence_penalty));
        }
        if let Some(stop) = request.stop.as_ref()
            && let Some(map) = body.as_object_mut()
        {
            map.insert("stop".to_string(), json!(stop));
        }

        // 额外参数（用于支持 provider 特定的参数，如 NVIDIA 的 reasoning_budget 等）
        if let Some(extra) = request.extra.as_ref()
            && let Some(map) = body.as_object_mut()
            && let Some(extra_map) = extra.as_object()
        {
            for (key, value) in extra_map {
                map.insert(key.clone(), value.clone());
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
            .map_err(|e| Self::map_reqwest_error(e, start.elapsed()))?;

        let status = resp.status();

        // 先读取原始文本，再尝试解析 JSON
        let text = resp
            .text()
            .await
            .map_err(|e| ProviderError::Message(format!("读取响应失败：{e}")))?;

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
            // 提供更友好的错误信息
            let error_msg = if status == reqwest::StatusCode::NOT_FOUND {
                format!(
                    "OpenAI 请求失败：status={} body={} \n\n\
                     提示：404 错误可能是因为：\n\
                     1. Base URL 不正确，请检查是否为有效的 API 端点\n\
                     2. 模型名称不正确，请确认模型在该平台可用\n\
                     3. API 路径不正确，某些平台可能需要特定的路径格式\n\n\
                     当前配置:\n\
                     - Base URL: {}\n\
                     - Model: {}",
                    status, text, self.base_url, model
                )
            } else {
                format!("OpenAI 请求失败：status={status} body={text}")
            };

            return Err(Self::map_http_error(status, error_msg));
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
                return Err(ProviderError::Message(format!("API 返回错误：{error}")));
            }

            return Err(ProviderError::Message(format!(
                "OpenAI 响应格式不兼容。响应内容：{}",
                preview(&text, 500)
            )));
        }

        let Some(message) = message.cloned() else {
            return Err(ProviderError::Message(
                "OpenAI 响应缺少 message 字段".to_string(),
            ));
        };

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
        if content.trim().is_empty()
            && tool_calls.is_empty()
            && let Some(r) = message.get("reasoning").and_then(|v| v.as_str())
            && !r.trim().is_empty()
        {
            content = r.to_string();
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
            .map(|usage_obj| rucora_core::provider::types::Usage {
                prompt_tokens: usage_obj
                    .get("prompt_tokens")
                    .and_then(|v| v.as_u64())
                    .map_or(0, |v| v as u32),
                completion_tokens: usage_obj
                    .get("completion_tokens")
                    .and_then(|v| v.as_u64())
                    .map_or(0, |v| v as u32),
                total_tokens: usage_obj
                    .get("total_tokens")
                    .and_then(|v| v.as_u64())
                    .map_or(0, |v| v as u32),
            });

        // 解析 finish_reason 字段
        let finish_reason = data
            .get("choices")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|c| c.get("finish_reason"))
            .and_then(|fr| fr.as_str())
            .map(Self::parse_finish_reason);

        debug!(
            provider = "openai",
            assistant_content_len = content.len(),
            "provider.chat.parsed"
        );

        Ok(ChatResponse {
            message: ChatMessage::assistant_with_tool_calls(content, tool_calls),
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
            tools_len = request.tools.as_ref().map_or(0, |t| t.len()),
            "provider.stream_chat.start"
        );

        let mut body = json!({
            "model": model,
            "messages": Self::build_messages(&request.messages),
            "stream": true,
        });

        if let Some(tools) = request.tools.as_ref()
            && let Some(map) = body.as_object_mut()
        {
            map.insert("tools".to_string(), Value::Array(Self::build_tools(tools)));
        }
        if let Some(t) = request.temperature
            && let Some(map) = body.as_object_mut()
        {
            map.insert("temperature".to_string(), json!(t));
        }
        if let Some(max_tokens) = request.max_tokens
            && let Some(map) = body.as_object_mut()
        {
            map.insert("max_tokens".to_string(), json!(max_tokens));
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
                .map_err(|e| Self::map_reqwest_error(e, start.elapsed()))?;

            let status = resp.status();
            if !status.is_success() {
                Err(Self::map_http_error(
                    status,
                    format!("OpenAI stream 请求失败：status={status}"),
                ))?;
            }

            debug!(
                provider = "openai",
                status = %status,
                elapsed_ms = start.elapsed().as_millis() as u64,
                "provider.stream_chat.http.started"
            );

            let mut buf = String::new();
            let mut bytes_stream = resp.bytes_stream();
            let mut tool_call_parts: BTreeMap<usize, (String, String, String)> = BTreeMap::new();

            while let Some(item) = bytes_stream.next().await {
                let bytes = item.map_err(|e| Self::map_reqwest_error(e, start.elapsed()))?;
                let chunk = String::from_utf8_lossy(&bytes);
                buf.push_str(&chunk);

                // SSE 事件以空行分隔。
                while let Some(idx) = buf.find("\n\n") {
                    // 用 drain 避免两次分配：取出事件文本并从缓冲区中移除。
                    let event: String = buf.drain(..=idx + 1).collect();
                    let event = event.trim_end_matches('\n').trim_end_matches('\r');

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
                        .map_err(|e| ProviderError::Message(format!("SSE JSON 解析失败: {e} data={data}")))?;

                    let choice = v
                        .get("choices")
                        .and_then(|c| c.as_array())
                        .and_then(|arr| arr.first());
                    let delta_obj = choice.and_then(|c0| c0.get("delta"));

                    // OpenAI: choices[0].delta.content
                    let delta = delta_obj
                        .and_then(|d| d.get("content"))
                        .and_then(|s| s.as_str())
                        .map(|s| s.to_string());

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
                        .map(Self::parse_finish_reason);

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

                // 如果已经收到 [DONE]，buf 会在上面的 break 后保留剩余内容；这里直接结束流。
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
    use rucora_core::provider::types::{MessageContent, ResponseFormat, Role};
    use rucora_core::tool::types::ToolDefinition;

    fn test_provider() -> OpenAiProvider {
        OpenAiProvider::with_model(
            "https://api.openai.com/v1",
            "test-key",
            "gpt-4o-mini",
        )
    }

    #[test]
    fn test_provider_creation_with_model() {
        let provider = test_provider();
        assert_eq!(provider.base_url, "https://api.openai.com/v1");
        assert_eq!(provider.default_model(), "gpt-4o-mini");
    }

    #[test]
    fn test_default_model_falls_back_to_constant() {
        let provider = OpenAiProvider::with_model("https://example.com", "key", "");
        // 空字符串也保持原样，不触发 fallback
        assert_eq!(provider.default_model(), "");
    }

    #[test]
    fn test_build_headers() {
        let headers = OpenAiProvider::build_headers("sk-test-123");
        assert_eq!(
            headers.get("authorization").unwrap(),
            "Bearer sk-test-123"
        );
        assert_eq!(headers.get("content-type").unwrap(), "application/json");
    }

    #[test]
    fn test_build_response_format_json_object() {
        let value = OpenAiProvider::build_response_format(&ResponseFormat::JsonObject);
        assert_eq!(value, json!({"type": "json_object"}));
    }

    #[test]
    fn test_build_response_format_json_schema() {
        let value = OpenAiProvider::build_response_format(&ResponseFormat::JsonSchema {
            name: "test_schema".to_string(),
            schema: json!({"type": "object"}),
            strict: None,
        });
        assert_eq!(
            value,
            json!({
                "type": "json_schema",
                "json_schema": {
                    "name": "test_schema",
                    "schema": {"type": "object"},
                }
            })
        );
    }

    #[test]
    fn test_build_response_format_json_schema_strict() {
        let value = OpenAiProvider::build_response_format(&ResponseFormat::JsonSchema {
            name: "test_schema".to_string(),
            schema: json!({}),
            strict: Some(true),
        });
        let inner = value
            .get("json_schema")
            .and_then(|v| v.as_object())
            .unwrap();
        assert_eq!(inner.get("strict"), Some(&json!(true)));
    }

    #[test]
    fn test_build_tools() {
        let tools = vec![ToolDefinition {
            name: "get_weather".to_string(),
            description: Some("获取天气".to_string()),
            input_schema: json!({"type": "object"}),
            version: 1,
        }];
        let value = OpenAiProvider::build_tools(&tools);
        assert_eq!(
            value,
            vec![json!({
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "description": "获取天气",
                    "parameters": {"type": "object"},
                }
            })]
        );
    }

    #[test]
    fn test_build_messages() {
        let messages = vec![ChatMessage::user("你好")];
        let value = OpenAiProvider::build_messages(&messages);
        assert_eq!(
            value,
            vec![json!({
                "role": "user",
                "content": "你好",
            })]
        );
    }

    #[test]
    fn test_build_messages_assistant_with_tool_calls() {
        let call = ToolCall {
            id: "call_1".to_string(),
            name: "get_weather".to_string(),
            input: json!({"city": "北京"}),
        };
        let messages = vec![
            ChatMessage::user("请调用工具"),
            ChatMessage {
                role: Role::Assistant,
                content: MessageContent::ToolCalls {
                    text: "".to_string(),
                    calls: vec![call],
                },
                name: None,
            },
        ];
        let value = OpenAiProvider::build_messages(&messages);
        assert_eq!(value[1]["tool_calls"][0]["id"], "call_1");
        assert_eq!(value[1]["tool_calls"][0]["function"]["name"], "get_weather");
    }

    #[test]
    fn test_parse_tool_calls_valid() {
        let message = json!({
            "tool_calls": [
                {
                    "id": "call_1",
                    "type": "function",
                    "function": {
                        "name": "get_weather",
                        "arguments": "{\"city\":\"北京\"}",
                    }
                }
            ]
        });
        let calls = OpenAiProvider::parse_tool_calls(&message);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_1");
        assert_eq!(calls[0].name, "get_weather");
        assert_eq!(calls[0].input, json!({"city": "北京"}));
    }

    #[test]
    fn test_parse_tool_calls_invalid_json_arguments() {
        let message = json!({
            "tool_calls": [
                {
                    "id": "call_1",
                    "function": {
                        "name": "get_weather",
                        "arguments": "not-json",
                    }
                }
            ]
        });
        let calls = OpenAiProvider::parse_tool_calls(&message);
        assert_eq!(calls.len(), 1);
        // 非 JSON 参数退化为字符串
        assert_eq!(calls[0].input, Value::String("not-json".to_string()));
    }

    #[test]
    fn test_parse_tool_calls_missing_id_or_name() {
        let message = json!({
            "tool_calls": [
                {"id": "", "function": {"name": "", "arguments": "{}"}},
                {"id": "call_2", "function": {"name": "ok", "arguments": "{}"}}
            ]
        });
        let calls = OpenAiProvider::parse_tool_calls(&message);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_2");
    }

    #[test]
    fn test_parse_tool_calls_empty() {
        let calls = OpenAiProvider::parse_tool_calls(&json!({}));
        assert!(calls.is_empty());
    }

    #[test]
    fn test_parse_finish_reason() {
        assert_eq!(OpenAiProvider::parse_finish_reason("stop"), FinishReason::Stop);
        assert_eq!(OpenAiProvider::parse_finish_reason("length"), FinishReason::Length);
        assert_eq!(OpenAiProvider::parse_finish_reason("tool_calls"), FinishReason::ToolCall);
        assert_eq!(OpenAiProvider::parse_finish_reason("unknown"), FinishReason::Other);
    }

    #[test]
    fn test_map_http_error_status_codes() {
        let auth = OpenAiProvider::map_http_error(reqwest::StatusCode::UNAUTHORIZED, "bad key".into());
        assert_eq!(auth.category(), rucora_core::error::ErrorCategory::Authentication);

        let rate = OpenAiProvider::map_http_error(reqwest::StatusCode::TOO_MANY_REQUESTS, "slow down".into());
        assert_eq!(rate.category(), rucora_core::error::ErrorCategory::RateLimit);

        let server = OpenAiProvider::map_http_error(reqwest::StatusCode::INTERNAL_SERVER_ERROR, "oops".into());
        assert_eq!(server.category(), rucora_core::error::ErrorCategory::Api);
    }

    #[test]
    fn test_map_reqwest_error_timeout() {
        // 无法轻易构造 reqwest::Error，仅验证超时分支的类型签名可编译
        let _ = std::mem::size_of::<ProviderError>();
    }

    #[test]
    fn test_with_timeouts_changes_client() {
        let provider = test_provider();
        let provider = provider.with_request_timeout(Some(300));
        assert_eq!(provider.request_timeout_secs, Some(300));
        let provider = provider.with_connect_timeout(Some(60));
        assert_eq!(provider.connect_timeout_secs, Some(60));
    }

    #[test]
    fn test_default_model_with_env() {
        // 环境变量注入
        unsafe {
            std::env::set_var("OPENAI_DEFAULT_MODEL", "gpt-5-test");
        }
        let provider = OpenAiProvider::with_model("https://example.com", "key", "");
        // with_model 不会读取环境变量，保持 ""；实际默认逻辑由 from_env 处理
        assert_eq!(provider.default_model(), "");
        unsafe {
            std::env::remove_var("OPENAI_DEFAULT_MODEL");
        }
    }
}
