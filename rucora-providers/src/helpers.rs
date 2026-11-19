//! Provider 辅助函数模块

use rucora_core::{
    error::ProviderError,
    provider::types::{ChatMessage, FinishReason, Role},
};
use serde_json::{Map, Value, json};

/// 将 reqwest 传输层错误映射为 `ProviderError`。
///
/// 超时 → `Timeout`，连接/请求失败 → 可重试 `Network`，其余 → `Message`。
pub fn map_reqwest_error(e: reqwest::Error, elapsed: std::time::Duration) -> ProviderError {
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

/// 将 HTTP 状态码映射为 `ProviderError`。
///
/// 401/403 → `Authentication`，429 → `RateLimit`（保留 `retry_after`），
/// 其余非成功状态 → `Api`。注意：仅对错误场景调用，成功状态交给调用方处理。
pub fn map_http_error(status: reqwest::StatusCode, message: String) -> ProviderError {
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

/// 将 provider 特定的 finish_reason 字符串转换为 FinishReason 枚举。
pub fn parse_finish_reason(reason: &str) -> FinishReason {
    match reason.to_lowercase().as_str() {
        "stop" | "end_turn" => FinishReason::Stop,
        "length" | "max_tokens" => FinishReason::Length,
        "tool_calls" | "function_call" | "tool_use" => FinishReason::ToolCall,
        _ => FinishReason::Other,
    }
}

/// 将 `Role` 映射为 OpenAI 兼容的 role 字符串。
pub fn map_role(role: &Role) -> &'static str {
    match role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "tool",
    }
}

/// 构建 OpenAI 兼容格式的消息数组。
///
/// 处理 assistant 的 tool_calls（`message.tool_calls()`）和 tool 的 tool_call_id。
/// 统一所有 OpenAI 兼容 provider 的消息转换逻辑。
///
/// # 参数
///
/// - `messages`: 内部 `ChatMessage` 列表
///
/// # 返回
///
/// OpenAI 兼容的 `Vec<Value>`，每个元素是一个消息对象。
pub fn build_openai_messages(messages: &[ChatMessage]) -> Vec<Value> {
    messages
        .iter()
        .map(|m| {
            let mut obj = json!({
                "role": map_role(&m.role),
                "content": m.content_text(),
            });
            if let Some(map) = obj.as_object_mut() {
                if let Some(name) = &m.name {
                    map.insert("name".to_string(), Value::String(name.clone()));
                }
                let tool_calls = m.tool_calls();
                if m.role == Role::Assistant && !tool_calls.is_empty() {
                    let calls: Vec<Value> = tool_calls
                        .iter()
                        .map(|call| {
                            json!({
                                "id": call.id,
                                "type": "function",
                                "function": {
                                    "name": call.name,
                                    "arguments": call.input.to_string(),
                                }
                            })
                        })
                        .collect();
                    map.insert("tool_calls".to_string(), Value::Array(calls));
                }
                if m.role == Role::Tool
                    && let Some(id) = m.tool_call_id()
                {
                    map.insert("tool_call_id".to_string(), Value::String(id.to_string()));
                }
            }
            obj
        })
        .collect()
}

/// SSE 流式响应的增量 UTF-8 解码器。
///
/// 网络分块的边界可能落在多字节字符（如中文）中间，若逐块调用
/// `from_utf8_lossy`，被切断的字符会立即替换为 U+FFFD 造成乱码。
/// 此解码器保留尾部不完整的字节序列，拼接到下一个分块后再解码。
///
/// # 用法
///
/// ```rust
/// use rucora_providers::helpers::IncrementalUtf8Decoder;
///
/// let mut decoder = IncrementalUtf8Decoder::new();
/// // 模拟"你"字（3 字节）被从中间切断
/// assert_eq!(decoder.feed(&[0xE4]), "");
/// assert_eq!(decoder.feed(&[0xBD]), "");
/// assert_eq!(decoder.feed(&[0xA0, 0x21]), "你!");
/// ```
pub struct IncrementalUtf8Decoder {
    buf: Vec<u8>,
}

impl IncrementalUtf8Decoder {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    /// 喂入一个网络分块，返回本次可安全解码的字符串。
    ///
    /// 尾部不完整的多字节序列会保留在内部缓冲区，等待后续分块补齐；
    /// 真正非法的字节序列会被替换为 U+FFFD 并消费掉。
    pub fn feed(&mut self, bytes: &[u8]) -> String {
        self.buf.extend_from_slice(bytes);
        match std::str::from_utf8(&self.buf) {
            // 整个缓冲区都是合法 UTF-8
            Ok(s) => {
                let out = s.to_string();
                self.buf.clear();
                out
            }
            Err(e) => {
                let valid = e.valid_up_to();
                let mut out = String::from_utf8_lossy(&self.buf[..valid]).into_owned();
                match e.error_len() {
                    // 真正非法的字节序列：消费并插入替换符
                    Some(bad_len) => {
                        out.push('\u{FFFD}');
                        self.buf.drain(..valid + bad_len);
                    }
                    // 尾部是不完整的多字节序列：保留待下一个分块
                    None => {
                        self.buf.drain(..valid);
                    }
                }
                out
            }
        }
    }
}

impl Default for IncrementalUtf8Decoder {
    fn default() -> Self {
        Self::new()
    }
}

/// 将 ChatRequest 中的采样参数添加到 JSON 对象中。
///
/// 这是一个通用的辅助函数，用于所有 OpenAI 兼容格式的 provider。
/// 支持的参数：temperature, top_p, top_k, max_tokens, frequency_penalty, presence_penalty, stop, extra
///
/// # 参数
///
/// - `map`: 要添加参数的 JSON 对象
/// - `temperature`: 温度参数
/// - `top_p`: top_p 采样参数
/// - `top_k`: top_k 采样参数
/// - `max_tokens`: 最大生成 token 数
/// - `frequency_penalty`: 频率惩罚
/// - `presence_penalty`: 存在惩罚
/// - `stop`: 停止序列
/// - `extra`: 额外参数（provider 特定）
#[allow(clippy::too_many_arguments)]
pub fn apply_sampling_params(
    map: &mut Map<String, Value>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    top_k: Option<u32>,
    max_tokens: Option<u32>,
    frequency_penalty: Option<f32>,
    presence_penalty: Option<f32>,
    stop: Option<&Vec<String>>,
    extra: Option<&Value>,
) {
    if let Some(v) = temperature {
        map.insert("temperature".to_string(), json!(v));
    }
    if let Some(v) = top_p {
        map.insert("top_p".to_string(), json!(v));
    }
    if let Some(v) = top_k {
        map.insert("top_k".to_string(), json!(v));
    }
    if let Some(v) = max_tokens {
        map.insert("max_tokens".to_string(), json!(v));
    }
    if let Some(v) = frequency_penalty {
        map.insert("frequency_penalty".to_string(), json!(v));
    }
    if let Some(v) = presence_penalty {
        map.insert("presence_penalty".to_string(), json!(v));
    }
    if let Some(v) = stop
        && !v.is_empty()
    {
        map.insert("stop".to_string(), json!(v));
    }

    // 添加额外参数（provider 特定参数，如 Ollama 的 think、chat_template_kwargs 等）
    if let Some(Value::Object(extra_map)) = extra {
        for (key, value) in extra_map {
            map.insert(key.clone(), value.clone());
        }
    }
}
