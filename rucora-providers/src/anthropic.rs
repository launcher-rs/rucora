//! Anthropic Provider 实现。
//!
//! 约定：
//! - API Key 从 `ANTHROPIC_API_KEY` 环境变量读取
//! - Base URL 默认 `https://api.anthropic.com/v1`
//! - 使用 Anthropic Messages API 格式
//! - 支持 Claude 系列模型（Claude 2, Claude 3 等）

use std::env;

use crate::{
    helpers::{IncrementalUtf8Decoder, map_http_error, map_reqwest_error, parse_finish_reason},
    http_config::{build_client, build_client_with_timeout, DEFAULT_CONNECT_TIMEOUT_SECS, DEFAULT_REQUEST_TIMEOUT_SECS},
    preview,
};
use async_trait::async_trait;
use futures_util::{StreamExt, stream::BoxStream};
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use rucora_core::{
    error::ProviderError,
    provider::{
        LlmProvider,
        types::{ChatMessage, ChatRequest, ChatResponse, ChatStreamChunk, Role, Usage},
    },
    tool::types::{ToolCall, ToolDefinition},
};
use serde_json::{Value, json};
use tracing::debug;

/// Anthropic 默认模型。
pub const ANTHROPIC_DEFAULT_MODEL: &str = "claude-sonnet-4-5";

/// Anthropic Claude Provider。
///
/// 支持 Claude 系列模型：
/// - claude-sonnet-4-5 (默认，别名自动指向最新快照)
/// - claude-haiku-4-5
/// - claude-opus-4-1
/// - claude-3-5-sonnet 等旧版模型仍可显式指定
///
/// # 默认模型优先级
///
/// 1. **手动设置**：通过 `with_model()` 或 `with_default_model()` 显式设置
/// 2. **环境变量**：`ANTHROPIC_DEFAULT_MODEL` 环境变量
/// 3. **内置默认值**：`claude-sonnet-4-5`
///
/// # 使用示例
///
/// ```rust,no_run
/// use rucora_providers::AnthropicProvider;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // 从环境变量加载（包括 ANTHROPIC_DEFAULT_MODEL）
/// let provider = AnthropicProvider::from_env()?;
///
/// // 或手动配置（使用内置默认模型）
/// let provider = AnthropicProvider::with_api_key("sk-ant-...");
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
/// | `ANTHROPIC_DEFAULT_MODEL` | 默认模型 | `claude-sonnet-4-5` |
#[derive(Clone)]
pub struct AnthropicProvider {
    client: reqwest::Client,
    headers: HeaderMap,
    base_url: String,
    default_model: String,
    api_version: String,
    request_timeout_secs: Option<u64>,
    connect_timeout_secs: Option<u64>,
    prompt_caching: bool,
}

impl AnthropicProvider {
    /// 从环境变量创建 Provider。
    ///
    /// 从以下环境变量加载配置：
    /// - `ANTHROPIC_API_KEY`: API Key（必需）
    /// - `ANTHROPIC_BASE_URL`: Base URL（可选，默认 `https://api.anthropic.com/v1`）
    /// - `ANTHROPIC_DEFAULT_MODEL`: 默认模型（可选，默认 `claude-sonnet-4-5`）
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
        let headers = Self::build_headers(&api_key);
        let client = Self::build_http_client(&headers, None, None);

        Self {
            client,
            headers,
            base_url: base_url.into(),
            default_model: default_model.into(),
            api_version: "2023-06-01".to_string(),
            request_timeout_secs: None,
            connect_timeout_secs: None,
            prompt_caching: false,
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

    /// 是否启用 Prompt Caching。
    ///
    /// 开启后：
    /// - system 提示词改写为带 `cache_control: {type: ephemeral}` 的内容块
    /// - 最后一个工具定义附加 `cache_control`（Anthropic 建议缓存点放在工具列表末尾）
    ///
    /// 命中缓存可大幅降低重复前缀的输入成本（最长 90%）。
    /// 默认关闭。
    pub fn with_prompt_caching(mut self, enabled: bool) -> Self {
        self.prompt_caching = enabled;
        self
    }

    /// 获取默认模型。
    pub fn default_model(&self) -> &str {
        &self.default_model
    }

    /// 设置 API 版本。
    ///
    /// 默认 `2023-06-01`。设置后会同步重建请求头与 HTTP 客户端，立即生效。
    pub fn with_api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = version.into();
        // 请求头在 with_model 时已构建完成，这里必须同步更新才能生效
        if let Ok(v) = HeaderValue::from_str(&self.api_version) {
            self.headers.insert("anthropic-version", v);
            self.client = Self::build_http_client(
                &self.headers,
                self.request_timeout_secs,
                self.connect_timeout_secs,
            );
        }
        self
    }

    fn build_headers(api_key: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Ok(v) = HeaderValue::from_str(api_key) {
            headers.insert("x-api-key", v);
        }
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
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

    fn build_system_prompt(messages: &[ChatMessage]) -> Option<String> {
        let system_parts: Vec<&str> = messages
            .iter()
            .filter(|m| m.role == Role::System)
            .map(|m| m.content_text())
            .collect();
        if system_parts.is_empty() {
            None
        } else {
            Some(system_parts.join("\n\n"))
        }
    }

    fn build_messages(messages: &[ChatMessage]) -> Vec<Value> {
        messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| match m.role {
                Role::Tool => {
                    let tool_call_id = m.tool_call_id().unwrap_or_default();
                    json!({
                        "role": "user",
                        "content": [{
                            "type": "tool_result",
                            "tool_use_id": tool_call_id,
                            "content": m.content_text()
                        }]
                    })
                }
                Role::Assistant if !m.tool_calls().is_empty() => {
                    let mut content = Vec::new();
                    let text = m.content_text();
                    if !text.trim().is_empty() {
                        content.push(json!({
                            "type": "text",
                            "text": text,
                        }));
                    }
                    for call in m.tool_calls() {
                        content.push(json!({
                            "type": "tool_use",
                            "id": call.id,
                            "name": call.name,
                            "input": call.input,
                        }));
                    }
                    json!({
                        "role": "assistant",
                        "content": content,
                    })
                }
                _ => {
                    let role = match m.role {
                        Role::User => "user",
                        Role::Assistant => "assistant",
                        _ => "user",
                    };
                    json!({
                        "role": role,
                        "content": m.content_text(),
                    })
                }
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
                if let Some(tool_type) = item.get("type").and_then(|v| v.as_str())
                    && tool_type == "tool_use"
                {
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
                if let Some(tool_type) = item.get("type").and_then(|v| v.as_str())
                    && tool_type == "text"
                    && let Some(text) = item.get("text").and_then(|v| v.as_str())
                {
                    texts.push(text);
                }
            }
            if !texts.is_empty() {
                return texts.join("\n");
            }
        }

        String::new()
    }

    /// 构建 Messages API 请求体（chat 与 stream_chat 共用）。
    ///
    /// 统一处理 system 提示词、工具定义与采样参数；
    /// Prompt Caching 开启时为 system 与最后一个工具附加 `cache_control`。
    fn build_request_body(&self, request: &ChatRequest, model: &str, stream: bool) -> Value {
        let system_prompt = Self::build_system_prompt(&request.messages);

        let mut body = json!({
            "model": model,
            "messages": Self::build_messages(&request.messages),
            "stream": stream,
        });

        if let Some(map) = body.as_object_mut() {
            if let Some(system) = system_prompt.as_deref() {
                // Anthropic 支持 system prompt 作为顶层字段；
                // Prompt Caching 开启时改写为带 cache_control 的内容块
                let system_value = if self.prompt_caching {
                    json!([{
                        "type": "text",
                        "text": system,
                        "cache_control": {"type": "ephemeral"},
                    }])
                } else {
                    json!(system)
                };
                map.insert("system".to_string(), system_value);
            }

            if let Some(tools) = request.tools.as_ref() {
                let mut tool_values = Self::build_tools(tools);
                // Prompt Caching：缓存点附加到最后一个工具定义（官方建议做法）
                if self.prompt_caching
                    && let Some(last) = tool_values.last_mut()
                    && let Some(obj) = last.as_object_mut()
                {
                    obj.insert("cache_control".to_string(), json!({"type": "ephemeral"}));
                }
                map.insert("tools".to_string(), Value::Array(tool_values));
            }

            if let Some(max_tokens) = request.max_tokens {
                map.insert("max_tokens".to_string(), json!(max_tokens));
            } else {
                // Anthropic 要求必须指定 max_tokens
                map.insert("max_tokens".to_string(), json!(4096));
            }
            if let Some(t) = request.temperature {
                map.insert("temperature".to_string(), json!(t));
            }
            if let Some(v) = request.top_p {
                map.insert("top_p".to_string(), json!(v));
            }
            if let Some(stop) = request.params.stop.as_ref()
                && !stop.is_empty()
            {
                map.insert("stop_sequences".to_string(), json!(stop));
            }
            // 额外参数（provider 特定参数）
            if let Some(Value::Object(extra_map)) = request.extra.as_ref() {
                for (key, value) in extra_map {
                    map.insert(key.clone(), value.clone());
                }
            }
        }

        body
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

        let messages = Self::build_messages(&request.messages);

        let last_user_preview = request
            .messages
            .iter()
            .rev()
            .find(|m| m.role == Role::User)
            .map(|m| preview(m.content_text(), 600));

        debug!(
            provider = "anthropic",
            url = %url,
            model = %model,
            messages_len = messages.len(),
            tools_len = request.tools.as_ref().map_or(0, |t| t.len()),
            last_user = last_user_preview.as_deref().unwrap_or(""),
            "provider.chat.start"
        );

        let body = self.build_request_body(&request, &model, false);

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
            .map_err(|e| map_reqwest_error(e, start.elapsed()))?;

        let status = resp.status();
        // 先读取文本再解析 JSON：错误响应可能不是 JSON（如代理返回 HTML），
        // 先检查状态码可以避免反序列化错误掩盖真实的 HTTP 错误
        let text = resp
            .text()
            .await
            .map_err(|e| map_reqwest_error(e, start.elapsed()))?;

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
            body = %preview(&text, 1200),
            "provider.chat.response_body"
        );

        if !status.is_success() {
            return Err(map_http_error(
                status,
                format!("Anthropic 请求失败：status={status} body={text}"),
            ));
        }

        let data: Value = serde_json::from_str(&text).map_err(|e| {
            ProviderError::Message(format!(
                "解析响应 JSON 失败：{e}。响应内容：{}",
                preview(&text, 500)
            ))
        })?;

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
            message: ChatMessage::assistant_with_tool_calls(text_content, tool_calls),
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

        let body = self.build_request_body(&request, &model, true);

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
                    format!("Anthropic 流式请求失败：status={status}"),
                ))?;
            }

            let mut buf = String::new();
            let mut decoder = IncrementalUtf8Decoder::new();
            let mut bytes_stream = resp.bytes_stream();

            // 按 content_block index 累积的流式工具调用（id/name/partial_json）
            let mut tool_accs: std::collections::BTreeMap<usize, (String, String, String)> =
                std::collections::BTreeMap::new();
            // 累积 usage：message_start 携带 input_tokens，message_delta 携带 output_tokens
            let mut usage_input_tokens: u32 = 0;
            let mut usage_output_tokens: u32 = 0;

            while let Some(item) = bytes_stream.next().await {
                let bytes = item.map_err(|e| ProviderError::Message(e.to_string()))?;
                // 增量解码：保留被分块边界切断的多字节字符，避免中文乱码
                buf.push_str(&decoder.feed(&bytes));

                while let Some(idx) = buf.find("\r\n\r\n").or_else(|| buf.find("\n\n")) {
                    let sep_len = if buf[idx..].starts_with("\r\n\r\n") { 4 } else { 2 };
                    // 用 drain 避免整段重新分配（O(n²) → O(n)）
                    let event: String = buf.drain(..idx + sep_len).collect();
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
                        break;
                    }

                    let json: Value = serde_json::from_str(&data)
                        .map_err(|e| ProviderError::Message(format!("SSE 解析失败：{e} data={data}")))?;

                    let event_type = json
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    if event_type == "content_block_start" {
                        // tool_use content block 开始
                        if json.get("content_block")
                            .and_then(|b| b.get("type"))
                            .and_then(|t| t.as_str())
                            == Some("tool_use")
                        {
                            let index = json.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                            let tool_call_id = json
                                .get("content_block")
                                .and_then(|b| b.get("id"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let tool_name = json
                                .get("content_block")
                                .and_then(|b| b.get("name"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            // start 事件不含 input；input 由后续 input_json_delta 分片拼装
                            tool_accs.insert(index, (tool_call_id, tool_name, String::new()));
                        }
                    }

                    if event_type == "content_block_delta" {
                        let delta = json.get("delta");
                        // 文本增量
                        if let Some(text) = delta
                            .and_then(|d| d.get("text"))
                            .and_then(|v| v.as_str())
                        {
                            yield ChatStreamChunk {
                                delta: Some(text.to_string()),
                                tool_calls: vec![],
                                usage: None,
                                finish_reason: None,
                            };
                        }
                        // 工具调用参数的 JSON 增量
                        if let Some(partial_json) = delta
                            .and_then(|d| d.get("input_json_delta"))
                            .and_then(|v| v.as_str())
                        {
                            let index = json.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                            if let Some((_, _, acc)) = tool_accs.get_mut(&index) {
                                acc.push_str(partial_json);
                            }
                        }
                    }

                    if event_type == "content_block_stop" {
                        // tool_use 块结束，组装完整 input 并 yield
                        let index = json.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                        if let Some((tool_call_id, tool_name, acc)) = tool_accs.remove(&index)
                            && !tool_call_id.is_empty()
                            && !tool_name.is_empty()
                        {
                            let input: Value = serde_json::from_str(acc.as_str()).unwrap_or(Value::String(acc));
                            yield ChatStreamChunk {
                                delta: None,
                                tool_calls: vec![ToolCall {
                                    id: tool_call_id,
                                    name: tool_name,
                                    input,
                                }],
                                usage: None,
                                finish_reason: None,
                            };
                        }
                    }

                    if event_type == "message_start" {
                        // message_start.message.usage.input_tokens
                        usage_input_tokens = json
                            .get("message")
                            .and_then(|m| m.get("usage"))
                            .and_then(|u| u.get("input_tokens"))
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as u32;
                    }

                    if event_type == "message_delta" {
                        let finish_reason = json
                            .get("delta")
                            .and_then(|d| d.get("stop_reason"))
                            .and_then(|v| v.as_str())
                            .map(parse_finish_reason);
                        // message_delta.usage.output_tokens；input_tokens 已从 message_start 获知
                        if let Some(out) = json
                            .get("usage")
                            .and_then(|u| u.get("output_tokens"))
                            .and_then(|v| v.as_u64())
                        {
                            usage_output_tokens = out as u32;
                        }
                        yield ChatStreamChunk {
                            delta: None,
                            tool_calls: vec![],
                            usage: Some(Usage {
                                prompt_tokens: usage_input_tokens,
                                completion_tokens: usage_output_tokens,
                                total_tokens: usage_input_tokens + usage_output_tokens,
                            }),
                            finish_reason,
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
    fn test_anthropic_provider_creation() {
        let provider = AnthropicProvider::with_api_key("test-key");
        assert_eq!(provider.base_url, "https://api.anthropic.com/v1");
        assert_eq!(provider.default_model(), ANTHROPIC_DEFAULT_MODEL);
    }

    #[test]
    fn test_anthropic_provider_with_custom_model() {
        let provider = AnthropicProvider::new("https://api.anthropic.com/v1", "test-key")
            .with_default_model("claude-sonnet-4-5");
        assert_eq!(provider.default_model(), "claude-sonnet-4-5");
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
        assert_eq!(ANTHROPIC_DEFAULT_MODEL, "claude-sonnet-4-5");
    }
}
