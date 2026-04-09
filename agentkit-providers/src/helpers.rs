//! Provider 辅助函数模块

use agentkit_core::provider::types::FinishReason;

/// 将 provider 特定的 finish_reason 字符串转换为 FinishReason 枚举。
pub fn parse_finish_reason(reason: &str) -> FinishReason {
    match reason.to_lowercase().as_str() {
        "stop" | "end_turn" => FinishReason::Stop,
        "length" | "max_tokens" => FinishReason::Length,
        "tool_calls" | "function_call" | "tool_use" => FinishReason::ToolCall,
        _ => FinishReason::Other,
    }
}


