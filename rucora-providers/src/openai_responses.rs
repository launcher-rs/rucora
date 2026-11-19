//! OpenAI Responses API Provider 实现。
//!
//! Responses API 是 OpenAI 面向 Agent 场景的核心原语（`POST /v1/responses`），
//! 相比 Chat Completions 的主要差异：
//! - 会话历史通过 `input` 数组表达，工具调用是顶层的 `function_call` / `function_call_output` 项
//! - system 提示词走顶层 `instructions` 字段
//! - 工具定义为扁平结构（不再嵌套在 `function` 对象下）
//! - 支持服务端会话状态（`previous_response_id`）、内置工具（web_search / file_search 等）、
//!   Background Mode 与 `reasoning` 参数——这些均可通过 `extra` 顶层透传
//!
//! 约定：
//! - API Key 从 `OPENAI_API_KEY` 环境变量读取
//! - Base URL 默认 `https://api.openai.com/v1`，也可通过 `OPENAI_BASE_URL` 覆盖
//! - 默认模型优先级：1) 手动设置 `with_default_model()` 2) `OPENAI_DEFAULT_MODEL` 环境变量
//!   3) 内置默认值 `gpt-5-mini`

use std::{collections::BTreeMap, env};

use crate::{
    helpers::{IncrementalUtf8Decoder, map_http_error, map_reqwest_error},
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

/// OpenAI Responses API 默认模型（当未指定时使用）。
pub const OPENAI_RESPONSES_DEFAULT_MODEL: &str = "gpt-5-mini";

/// OpenAI Responses API Provider。
///
/// 与 [`OpenAiProvider`](crate::openai::OpenAiProvider)（Chat Completions）并存的
/// Responses API 实现，适合需要内置工具、服务端会话状态或 reasoning 参数的场景。
///
/// # 默认模型
///
/// 默认模型的优先级顺序：
/// 1. 手动调用 `with_default_model()` 设置的值
/// 2. `OPENAI_DEFAULT_MODEL` 环境变量
/// 3. 内置默认值 `gpt-5-mini`
///
/// # 超时配置
///
/// 默认请求超时 120 秒，连接超时 15 秒。可通过 `with_request_timeout` /
/// `with_connect_timeout` / `with_client` 自定义。
///
/// # extra 顶层透传
///
/// Responses API 的特有参数（如 `reasoning`、`store`、`background`、
/// `previous_response_id`、`previous_response_id` 关联的服务端状态、内置工具
/// `tools: [{type: "web_search"}]`）可通过 `ChatRequest` 的 `extra` 字段顶层合并：
///
/// ```rust,no_run
/// use rucora_providers::OpenAiResponsesProvider;
/// use rucora_core::provider::types::{ChatMessage, ChatRequest};
/// use serde_json::json;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let provider = OpenAiResponsesProvider::from_env()?;
/// let mut request = ChatRequest::new(vec![ChatMessage::user("今天旧金山天气如何？".to_string())]);
/// // Responses API 特有参数通过 extra 顶层透传
/// request.params.extra = Some(json!({"reasoning": {"effort": "medium"}}));
/// // let response = provider.chat(request).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct OpenAiResponsesProvider {
    client: reqwest::Client,
    headers: HeaderMap,
    base_url: String,
    default_model: String,
    request_timeout_secs: Option<u64>,
    connect_timeout_secs: Option<u64>,
}

impl OpenAiResponsesProvider {
    /// 从环境变量创建 Provider。
    ///
    /// 默认模型来源（按优先级）：
    /// 1. `OPENAI_DEFAULT_MODEL` 环境变量
    /// 2. 内置默认值 `gpt-5-mini`
    pub fn from_env() -> Result<Self, ProviderError> {
        let api_key = env::var("OPENAI_API_KEY")
            .map_err(|_| ProviderError::Message("缺少环境变量 OPENAI_API_KEY".to_string()))?;
        let base_url =
            env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let default_model = env::var("OPENAI_DEFAULT_MODEL")
            .unwrap_or_else(|_| OPENAI_RESPONSES_DEFAULT_MODEL.to_string());

        Ok(Self::with_model(base_url, api_key, default_model))
    }

    /// 创建 Provider（使用内置默认模型 `gpt-5-mini`）。
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self::with_model(base_url, api_key, OPENAI_RESPONSES_DEFAULT_MODEL.to_string())
    }

    /// 创建 Provider（指定默认模型）。
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

    /// 设置请求超时时间（秒）。`None` 恢复默认值。
    pub fn with_request_timeout(mut self, secs: Option<u64>) -> Self {
        self.request_timeout_secs = secs;
        self.client = Self::build_http_client(&self.headers, self.request_timeout_secs, self.connect_timeout_secs);
        self
    }

    /// 设置连接超时时间（秒）。`None` 恢复默认值。
    pub fn with_connect_timeout(mut self, secs: Option<u64>) -> Self {
        self.connect_timeout_secs = secs;
        self.client = Self::build_http_client(&self.headers, self.request_timeout_secs, self.connect_timeout_secs);
        self
    }

    /// 设置自定义 HTTP 客户端（完全控制代理、TLS、超时等配置）。
    pub fn with_client(mut self, client: reqwest::Client) -> Self {
        self.client = client;
        self
    }

    /// 获取当前配置的默认模型。
    pub fn default_model(&self) -> &str {
        &self.default_model
    }

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

    /// 将消息历史转换为 Responses API 的 `input` 数组。
    ///
    /// - System 消息不入 `input`（由调用方汇总进顶层 `instructions`）
    /// - Tool 消息 → `function_call_output` 项
    /// - 带工具调用的 assistant 消息 → `function_call` 项（先输出文本）
    fn build_input(messages: &[ChatMessage]) -> Vec<Value> {
        let mut out = Vec::new();
        for m in messages {
            match &m.role {
                Role::System => {}
                Role::Tool => {
                    let call_id = m.tool_call_id().unwrap_or_default();
                    out.push(json!({
                        "type": "function_call_output",
                        "call_id": call_id,
                        "output": m.content_text(),
                    }));
                }
                Role::Assistant if !m.tool_calls().is_empty() => {
                    let text = m.content_text();
                    if !text.trim().is_empty() {
                        out.push(json!({
                            "role": "assistant",
                            "content": [{"type": "output_text", "text": text}],
                        }));
                    }
                    for call in m.tool_calls() {
                        out.push(json!({
                            "type": "function_call",
                            "call_id": call.id,
                            "name": call.name,
                            "arguments": call.input.to_string(),
                        }));
                    }
                }
                role => {
                    // User → input_text；无工具调用的 assistant → output_text
                    let (role_name, text_type) = match role {
                        Role::User => ("user", "input_text"),
                        _ => ("assistant", "output_text"),
                    };
                    out.push(json!({
                        "role": role_name,
                        "content": [{"type": text_type, "text": m.content_text()}],
                    }));
                }
            }
        }
        out
    }

    /// 构建扁平结构的函数工具定义。
    fn build_tools(tools: &[ToolDefinition]) -> Vec<Value> {
        tools
            .iter()
            .map(|t| {
                let mut def = json!({
                    "type": "function",
                    "name": t.name,
                    "parameters": t.input_schema,
                });
                // description 为可选字段，避免序列化为 null
                if let Some(desc) = &t.description
                    && let Some(obj) = def.as_object_mut()
                {
                    obj.insert("description".to_string(), json!(desc));
                }
                def
            })
            .collect()
    }

    /// 将 `ResponseFormat` 映射为 Responses API 的顶层 `text.format` 结构。
    fn build_text_format(fmt: &ResponseFormat) -> Value {
        match fmt {
            ResponseFormat::JsonObject => json!({"format": {"type": "json_object"}}),
            ResponseFormat::JsonSchema { name, schema, strict } => {
                let mut format = json!({"type": "json_schema", "name": name, "schema": schema});
                if let Some(strict) = strict {
                    format["strict"] = json!(strict);
                }
                json!({"format": format})
            }
        }
    }

    /// 构建请求体（chat 与 stream_chat 共用）。
    fn build_body(&self, request: &ChatRequest, model: &str, stream: bool) -> Value {
        // System 消息汇总为顶层 instructions
        let system_parts: Vec<&str> = request
            .messages
            .iter()
            .filter(|m| m.role == Role::System)
            .map(|m| m.content_text())
            .collect();

        let mut body = json!({
            "model": model,
            "input": Self::build_input(&request.messages),
            "stream": stream,
        });

        if let Some(map) = body.as_object_mut() {
            if !system_parts.is_empty() {
                map.insert("instructions".to_string(), json!(system_parts.join("\n\n")));
            }
            if let Some(tools) = request.tools.as_ref()
                && !tools.is_empty()
            {
                map.insert("tools".to_string(), Value::Array(Self::build_tools(tools)));
            }
            if let Some(t) = request.temperature {
                map.insert("temperature".to_string(), json!(t));
            }
            if let Some(v) = request.top_p {
                map.insert("top_p".to_string(), json!(v));
            }
            if let Some(max_tokens) = request.max_tokens {
                map.insert("max_output_tokens".to_string(), json!(max_tokens));
            }
            if let Some(fmt) = request.response_format.as_ref() {
                map.insert("text".to_string(), Self::build_text_format(fmt));
            }
            // extra 顶层合并：reasoning / store / background / previous_response_id / 内置工具等
            if let Some(Value::Object(extra_map)) = request.extra.as_ref() {
                for (key, value) in extra_map {
                    map.insert(key.clone(), value.clone());
                }
            }
        }

        body
    }

    /// 从 `output` 数组提取文本与工具调用。
    fn parse_output(output: &Value) -> (String, Vec<ToolCall>) {
        let mut text = String::new();
        let mut calls = Vec::new();
        let Some(items) = output.as_array() else {
            return (text, calls);
        };

        for item in items {
            match item.get("type").and_then(|t| t.as_str()) {
                Some("message") => {
                    if let Some(content) = item.get("content").and_then(|c| c.as_array()) {
                        for part in content {
                            if part.get("type").and_then(|t| t.as_str()) == Some("output_text")
                                && let Some(t) = part.get("text").and_then(|x| x.as_str())
                            {
                                text.push_str(t);
                            }
                        }
                    }
                }
                Some("function_call") => {
                    let call_id = item.get("call_id").and_then(|v| v.as_str()).unwrap_or("");
                    let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let args_raw = item.get("arguments").and_then(|v| v.as_str()).unwrap_or("{}");
                    let input: Value = serde_json::from_str(args_raw)
                        .unwrap_or_else(|_| Value::String(args_raw.to_string()));
                    if !call_id.is_empty() && !name.is_empty() {
                        calls.push(ToolCall {
                            id: call_id.to_string(),
                            name: name.to_string(),
                            input,
                        });
                    }
                }
                _ => {}
            }
        }

        (text, calls)
    }

    /// 结合响应 `status` 与是否存在工具调用映射 FinishReason。
    fn parse_finish(response: &Value, has_tool_calls: bool) -> FinishReason {
        let status = response.get("status").and_then(|v| v.as_str()).unwrap_or("");
        match status {
            "completed" if has_tool_calls => FinishReason::ToolCall,
            "completed" => FinishReason::Stop,
            "incomplete" => {
                let reason = response
                    .get("incomplete_details")
                    .and_then(|d| d.get("reason"))
                    .and_then(|r| r.as_str())
                    .unwrap_or("");
                if reason == "max_output_tokens" {
                    FinishReason::Length
                } else {
                    FinishReason::Other
                }
            }
            _ => FinishReason::Other,
        }
    }

    /// 解析 Responses API 的 usage 结构（input_tokens / output_tokens / total_tokens）。
    fn parse_usage(usage: &Value) -> Usage {
        Usage {
            prompt_tokens: usage.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            completion_tokens: usage.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            total_tokens: usage.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAiResponsesProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let model = request
            .model
            .clone()
            .unwrap_or_else(|| self.default_model.clone());

        let url = format!("{}/responses", self.base_url.trim_end_matches('/'));
        let body = self.build_body(&request, &model, false);

        debug!(
            provider = "openai_responses",
            url = %url,
            model = %model,
            messages_len = request.messages.len(),
            tools_len = request.tools.as_ref().map_or(0, |t| t.len()),
            "provider.chat.start"
        );
        debug!(
            provider = "openai_responses",
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
        // 先读取文本再解析 JSON：错误响应可能不是 JSON，先检查状态码避免掩盖真实错误
        let text = resp
            .text()
            .await
            .map_err(|e| map_reqwest_error(e, start.elapsed()))?;

        debug!(
            provider = "openai_responses",
            status = %status,
            elapsed_ms = start.elapsed().as_millis() as u64,
            "provider.chat.http.done"
        );

        if !status.is_success() {
            return Err(map_http_error(
                status,
                format!("OpenAI Responses 请求失败：status={status} body={text}"),
            ));
        }

        let data: Value = serde_json::from_str(&text).map_err(|e| {
            ProviderError::Message(format!(
                "解析响应 JSON 失败：{e}。响应内容：{}",
                preview(&text, 500)
            ))
        })?;

        let (content, tool_calls) = Self::parse_output(data.get("output").unwrap_or(&Value::Null));
        let finish_reason = Some(Self::parse_finish(&data, !tool_calls.is_empty()));
        let usage = data.get("usage").filter(|u| u.is_object()).map(Self::parse_usage);

        if !tool_calls.is_empty() {
            let names: Vec<&str> = tool_calls.iter().map(|c| c.name.as_str()).collect();
            debug!(
                provider = "openai_responses",
                tool_calls_len = tool_calls.len(),
                tool_call_names = ?names,
                "provider.chat.tool_calls"
            );
        }
        debug!(
            provider = "openai_responses",
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
        let model = request
            .model
            .clone()
            .unwrap_or_else(|| self.default_model.clone());

        let url = format!("{}/responses", self.base_url.trim_end_matches('/'));
        let body = self.build_body(&request, &model, true);

        debug!(
            provider = "openai_responses",
            model = %model,
            "provider.stream_chat.start"
        );

        let client = self.client.clone();
        let stream = async_stream::try_stream! {
            let start = std::time::Instant::now();
            let resp = client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| map_reqwest_error(e, start.elapsed()))?;

            let status = resp.status();
            if !status.is_success() {
                Err(map_http_error(
                    status,
                    format!("OpenAI Responses 流式请求失败：status={status}"),
                ))?;
            }

            debug!(
                provider = "openai_responses",
                status = %status,
                elapsed_ms = start.elapsed().as_millis() as u64,
                "provider.stream_chat.http.started"
            );

            let mut buf = String::new();
            let mut decoder = IncrementalUtf8Decoder::new();
            let mut bytes_stream = resp.bytes_stream();

            // 按条目 id 累积流式工具调用（call_id / name / arguments）
            let mut tool_accs: BTreeMap<String, (String, String, String)> = BTreeMap::new();

            'outer: while let Some(item) = bytes_stream.next().await {
                let bytes = item.map_err(|e| map_reqwest_error(e, start.elapsed()))?;
                // 增量解码：保留被分块边界切断的多字节字符，避免中文乱码
                buf.push_str(&decoder.feed(&bytes));

                while let Some(idx) = buf.find("\r\n\r\n").or_else(|| buf.find("\n\n")) {
                    let sep_len = if buf[idx..].starts_with("\r\n\r\n") { 4 } else { 2 };
                    // 用 drain 避免整段重新分配
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
                        break 'outer;
                    }

                    let v: Value = serde_json::from_str(&data)
                        .map_err(|e| ProviderError::Message(format!("SSE JSON 解析失败: {e} data={data}")))?;

                    let event_type = v.get("type").and_then(|t| t.as_str()).unwrap_or("");

                    match event_type {
                        // 文本增量
                        "response.output_text.delta" => {
                            if let Some(delta) = v.get("delta").and_then(|d| d.as_str()) {
                                yield ChatStreamChunk {
                                    delta: Some(delta.to_string()),
                                    tool_calls: vec![],
                                    usage: None,
                                    finish_reason: None,
                                };
                            }
                        }
                        // 工具调用条目开始：登记 call_id 与 name
                        "response.output_item.added" => {
                            let item = v.get("item");
                            if item.and_then(|i| i.get("type")).and_then(|t| t.as_str()) == Some("function_call") {
                                let item_id = item.and_then(|i| i.get("id")).and_then(|x| x.as_str()).unwrap_or("").to_string();
                                let call_id = item.and_then(|i| i.get("call_id")).and_then(|x| x.as_str()).unwrap_or("").to_string();
                                let name = item.and_then(|i| i.get("name")).and_then(|x| x.as_str()).unwrap_or("").to_string();
                                if !item_id.is_empty() {
                                    tool_accs.insert(item_id, (call_id, name, String::new()));
                                }
                            }
                        }
                        // 工具调用参数增量
                        "response.function_call_arguments.delta" => {
                            let item_id = v.get("item_id").and_then(|d| d.as_str()).unwrap_or("");
                            if let Some(delta) = v.get("delta").and_then(|d| d.as_str())
                                && let Some((_, _, acc)) = tool_accs.get_mut(item_id)
                            {
                                acc.push_str(delta);
                            }
                        }
                        // 条目完成：function_call 组装完整 ToolCall
                        "response.output_item.done" => {
                            let item = v.get("item");
                            if item.and_then(|i| i.get("type")).and_then(|t| t.as_str()) == Some("function_call") {
                                // done 条目自带完整 arguments，无需依赖增量累积
                                let call_id = item.and_then(|i| i.get("call_id")).and_then(|x| x.as_str()).unwrap_or("").to_string();
                                let name = item.and_then(|i| i.get("name")).and_then(|x| x.as_str()).unwrap_or("").to_string();
                                let args_raw = item.and_then(|i| i.get("arguments")).and_then(|x| x.as_str()).unwrap_or("{}").to_string();
                                let input: Value = serde_json::from_str(&args_raw)
                                    .unwrap_or_else(|_| Value::String(args_raw.clone()));
                                let item_id = item.and_then(|i| i.get("id")).and_then(|x| x.as_str()).unwrap_or("").to_string();
                                tool_accs.remove(&item_id);
                                if !call_id.is_empty() && !name.is_empty() {
                                    yield ChatStreamChunk {
                                        delta: None,
                                        tool_calls: vec![ToolCall { id: call_id, name, input }],
                                        usage: None,
                                        finish_reason: None,
                                    };
                                }
                            }
                        }
                        // 响应完成 / 不完整：携带 usage 与最终状态
                        "response.completed" | "response.incomplete" => {
                            let response = v.get("response");
                            let usage = response
                                .and_then(|r| r.get("usage"))
                                .filter(|u| u.is_object())
                                .map(Self::parse_usage);
                            let has_calls = response
                                .and_then(|r| r.get("output"))
                                .and_then(|o| o.as_array())
                                .is_some_and(|arr| {
                                    arr.iter().any(|i| i.get("type").and_then(|t| t.as_str()) == Some("function_call"))
                                });
                            let finish_reason = response
                                .map_or(FinishReason::Other, |r| Self::parse_finish(r, has_calls));
                            yield ChatStreamChunk {
                                delta: None,
                                tool_calls: vec![],
                                usage,
                                finish_reason: Some(finish_reason),
                            };
                            break 'outer;
                        }
                        // 响应失败：服务端错误事件
                        "response.failed" | "error" => {
                            let message = v.pointer("/response/error/message")
                                .or_else(|| v.get("error").and_then(|e| e.get("message")))
                                .and_then(|m| m.as_str())
                                .unwrap_or("OpenAI Responses 流式请求失败");
                            Err(ProviderError::Message(format!("OpenAI Responses 流式错误：{message}")))?;
                        }
                        _ => {}
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
    use rucora_core::tool::types::ToolCall;

    fn test_provider() -> OpenAiResponsesProvider {
        OpenAiResponsesProvider::with_model(
            "https://api.openai.com/v1",
            "test-key",
            "gpt-5-mini",
        )
    }

    #[test]
    fn test_provider_creation_with_model() {
        let provider = test_provider();
        assert_eq!(provider.base_url, "https://api.openai.com/v1");
        assert_eq!(provider.default_model(), "gpt-5-mini");
    }

    #[test]
    fn test_build_headers() {
        let headers = OpenAiResponsesProvider::build_headers("sk-test-123");
        assert_eq!(headers.get("authorization").unwrap(), "Bearer sk-test-123");
        assert_eq!(headers.get("content-type").unwrap(), "application/json");
    }

    #[test]
    fn test_build_input_user_and_system() {
        let messages = vec![
            ChatMessage::system("你是有用的助手".to_string()),
            ChatMessage::user("你好".to_string()),
        ];
        let input = OpenAiResponsesProvider::build_input(&messages);
        // system 不进 input（走 instructions）
        assert_eq!(input.len(), 1);
        assert_eq!(input[0]["role"], "user");
        assert_eq!(input[0]["content"][0]["type"], "input_text");
        assert_eq!(input[0]["content"][0]["text"], "你好");
    }

    #[test]
    fn test_build_input_tool_cycle() {
        let call = ToolCall {
            id: "call_123".to_string(),
            name: "get_weather".to_string(),
            input: json!({"city": "旧金山"}),
        };
        let messages = vec![
            ChatMessage::assistant_with_tool_calls("让我查一下".to_string(), vec![call]),
            ChatMessage::tool_result("get_weather", "call_123", "晴天 25 度"),
        ];
        let input = OpenAiResponsesProvider::build_input(&messages);
        assert_eq!(input.len(), 3);
        // assistant 文本 + function_call 项
        assert_eq!(input[0]["role"], "assistant");
        assert_eq!(input[0]["content"][0]["type"], "output_text");
        assert_eq!(input[1]["type"], "function_call");
        assert_eq!(input[1]["call_id"], "call_123");
        assert_eq!(input[1]["name"], "get_weather");
        // JSON 序列化的 arguments 可反序列化回原结构
        let args: Value = serde_json::from_str(input[1]["arguments"].as_str().unwrap()).unwrap();
        assert_eq!(args["city"], "旧金山");
        // 工具结果 → function_call_output
        assert_eq!(input[2]["type"], "function_call_output");
        assert_eq!(input[2]["call_id"], "call_123");
    }

    #[test]
    fn test_build_input_function_call_output() {
        let messages = vec![ChatMessage::tool_result(
            "get_weather",
            "call_456",
            "晴天 25 度",
        )];
        let input = OpenAiResponsesProvider::build_input(&messages);
        assert_eq!(input.len(), 1);
        assert_eq!(input[0]["type"], "function_call_output");
        assert_eq!(input[0]["call_id"], "call_456");
        assert_eq!(input[0]["output"], "晴天 25 度");
    }

    #[test]
    fn test_build_tools_flat_format() {
        let tools = vec![ToolDefinition {
            name: "get_weather".to_string(),
            description: Some("查询天气".to_string()),
            input_schema: json!({"type": "object"}),
            version: 1,
        }];
        let built = OpenAiResponsesProvider::build_tools(&tools);
        assert_eq!(built.len(), 1);
        assert_eq!(built[0]["type"], "function");
        assert_eq!(built[0]["name"], "get_weather");
        assert_eq!(built[0]["parameters"]["type"], "object");
        // 扁平结构：没有嵌套的 function 对象
        assert!(built[0].get("function").is_none());
    }

    #[test]
    fn test_build_text_format() {
        let obj = OpenAiResponsesProvider::build_text_format(&ResponseFormat::JsonObject);
        assert_eq!(obj["format"]["type"], "json_object");

        let schema = OpenAiResponsesProvider::build_text_format(&ResponseFormat::JsonSchema {
            name: "out".to_string(),
            schema: json!({"type": "object"}),
            strict: Some(true),
        });
        assert_eq!(schema["format"]["type"], "json_schema");
        assert_eq!(schema["format"]["name"], "out");
        assert_eq!(schema["format"]["strict"], true);
    }

    #[test]
    fn test_build_body() {
        let provider = test_provider();
        let mut request = ChatRequest::new(vec![
            ChatMessage::system("系统提示".to_string()),
            ChatMessage::user("你好".to_string()),
        ]);
        request.model = Some("gpt-5.1".to_string());
        request.max_tokens = Some(2048);
        request.temperature = Some(0.5);

        let body = provider.build_body(&request, "gpt-5.1", false);
        assert_eq!(body["model"], "gpt-5.1");
        assert_eq!(body["stream"], false);
        assert_eq!(body["instructions"], "系统提示");
        assert_eq!(body["max_output_tokens"], 2048);
        assert_eq!(body["temperature"], 0.5);
    }

    #[test]
    fn test_build_body_extra_merge() {
        let provider = test_provider();
        let mut request =
            ChatRequest::new(vec![ChatMessage::user("查询".to_string())]);
        request.params.extra = Some(json!({"reasoning": {"effort": "high"}, "store": false}));

        let body = provider.build_body(&request, "gpt-5-mini", true);
        assert_eq!(body["stream"], true);
        assert_eq!(body["reasoning"]["effort"], "high");
        assert_eq!(body["store"], false);
    }

    #[test]
    fn test_parse_output_message_and_function_call() {
        let output = json!([
            {"type": "reasoning", "id": "rs_1", "summary": []},
            {
                "type": "message",
                "role": "assistant",
                "content": [
                    {"type": "output_text", "text": "第一段"},
                    {"type": "output_text", "text": "第二段"}
                ]
            },
            {
                "type": "function_call",
                "call_id": "call_789",
                "name": "search",
                "arguments": "{\"q\": \"天气\"}"
            }
        ]);
        let (text, calls) = OpenAiResponsesProvider::parse_output(&output);
        assert_eq!(text, "第一段第二段");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_789");
        assert_eq!(calls[0].name, "search");
        assert_eq!(calls[0].input["q"], "天气");
    }

    #[test]
    fn test_parse_finish() {
        let completed = json!({"status": "completed"});
        assert_eq!(
            OpenAiResponsesProvider::parse_finish(&completed, false),
            FinishReason::Stop
        );
        assert_eq!(
            OpenAiResponsesProvider::parse_finish(&completed, true),
            FinishReason::ToolCall
        );

        let length_capped = json!({
            "status": "incomplete",
            "incomplete_details": {"reason": "max_output_tokens"}
        });
        assert_eq!(
            OpenAiResponsesProvider::parse_finish(&length_capped, false),
            FinishReason::Length
        );

        let other = json!({"status": "incomplete", "incomplete_details": {"reason": "content_filter"}});
        assert_eq!(
            OpenAiResponsesProvider::parse_finish(&other, false),
            FinishReason::Other
        );
    }

    #[test]
    fn test_parse_usage() {
        let usage = OpenAiResponsesProvider::parse_usage(&json!({
            "input_tokens": 100,
            "output_tokens": 50,
            "total_tokens": 150
        }));
        assert_eq!(usage.prompt_tokens, 100);
        assert_eq!(usage.completion_tokens, 50);
        assert_eq!(usage.total_tokens, 150);
    }
}
