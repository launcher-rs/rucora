//! Provider 辅助函数模块

use rucora_core::provider::types::{ChatMessage, FinishReason, Role};
use serde_json::{Map, Value, json};

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
                if m.role == Role::Tool {
                    if let Some(id) = m.tool_call_id() {
                        map.insert("tool_call_id".to_string(), Value::String(id.to_string()));
                    }
                }
            }
            obj
        })
        .collect()
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
