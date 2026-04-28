//! Channel（通信渠道）相关的类型定义
//!
//! # 概述
//!
//! 本模块定义了 rucora 统一的事件模型，用于在组件之间传递消息。
//! 所有事件都支持序列化，便于 Trace 记录和回放。
//!
//! # 事件类型
//!
//! ## ChannelEvent 枚举
//!
//! [`ChannelEvent`] 是统一的事件类型，包含以下变体：
//!
//! ```text
//! ChannelEvent
//! ├── Message(ChatMessage)      - 对话消息
//! ├── TokenDelta(TokenDeltaEvent) - Token 流式输出
//! ├── ToolCall(ToolCall)        - 工具调用
//! ├── ToolResult(ToolResult)    - 工具结果
//! ├── Skill(SkillEvent)         - 技能执行
//! ├── Memory(MemoryEvent)       - 记忆操作
//! ├── Debug(DebugEvent)         - 调试信息
//! ├── Error(ErrorEvent)         - 错误信息
//! └── Raw(Value)                - 原始数据
//! ```
//!
//! # 使用示例
//!
//! ## 创建事件
//!
//! ```rust
//! use rucora_core::channel::types::*;
//! use rucora_core::provider::types::{ChatMessage, Role};
//! use serde_json::json;
//!
//! // 消息事件
//! let message = ChannelEvent::Message(ChatMessage {
//!     role: Role::Assistant,
//!     content: "你好！".to_string(),
//!     name: None,
//! });
//!
//! // Token 流式事件
//! let token = ChannelEvent::TokenDelta(TokenDeltaEvent {
//!     delta: "你".to_string(),
//! });
//!
//! // 调试事件
//! let debug = ChannelEvent::Debug(DebugEvent {
//!     message: "step.start".to_string(),
//!     data: Some(json!({"step": 1})),
//! });
//! ```
//!
//! ## 事件匹配
//!
//! ```rust
//! use rucora_core::channel::types::*;
//!
//! fn handle_event(event: ChannelEvent) {
//!     match event {
//!         ChannelEvent::TokenDelta(delta) => {
//!             print!("{}", delta.delta);
//!         }
//!         ChannelEvent::Message(msg) => {
//!             println!("消息：{}", msg.content);
//!         }
//!         ChannelEvent::Error(err) => {
//!             eprintln!("错误：{}", err.message);
//!         }
//!         _ => {}
//!     }
//! }
//! ```
//!
//! ## 事件序列化
//!
//! ```rust
//! use rucora_core::channel::types::*;
//! use serde_json;
//!
//! let event = ChannelEvent::Debug(DebugEvent {
//!     message: "test".to_string(),
//!     data: None,
//! });
//!
//! // 序列化
//! let json = serde_json::to_string(&event).unwrap();
//!
//! // 反序列化
//! let parsed: ChannelEvent = serde_json::from_str(&json).unwrap();
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    provider::types::ChatMessage,
    tool::types::{ToolCall, ToolResult},
};

/// Token 流式输出事件
///
/// 用于流式 provider 输出 delta/token。
///
/// # 字段说明
///
/// - `delta`: 增量文本（每次输出的 token）
///
/// # 示例
///
/// ```rust
/// use rucora_core::channel::types::TokenDeltaEvent;
///
/// let event = TokenDeltaEvent {
///     delta: "你".to_string(),
/// };
///
/// assert_eq!(event.delta, "你");
/// ```
///
/// # 流式输出流程
///
/// ```text
/// LLM 响应
///    │
///    ▼
/// "你" ──► TokenDeltaEvent { delta: "你" }
///    │
///    ▼
/// "好" ──► TokenDeltaEvent { delta: "好" }
///    │
///    ▼
/// "！" ──► TokenDeltaEvent { delta: "！" }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenDeltaEvent {
    /// 增量文本（delta）
    pub delta: String,
}

/// Skill 执行事件
///
/// 用于记录 Skill 的执行过程，便于 GUI/CLI 做统一可视化。
///
/// # 字段说明
///
/// - `name`: Skill 名称
/// - `phase`: 执行阶段（start/end/error 等）
/// - `data`: 可选附加数据（入参/出参/耗时/错误详情等）
///
/// # 示例
///
/// ```rust
/// use rucora_core::channel::types::SkillEvent;
/// use serde_json::json;
///
/// // Skill 开始执行
/// let start = SkillEvent {
///     name: "weather".to_string(),
///     phase: "start".to_string(),
///     data: Some(json!({"location": "Beijing"})),
/// };
///
/// // Skill 执行完成
/// let end = SkillEvent {
///     name: "weather".to_string(),
///     phase: "end".to_string(),
///     data: Some(json!({"result": "晴朗", "temperature": 25})),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillEvent {
    /// Skill 名称
    pub name: String,
    /// 执行阶段：start/end/error 等
    pub phase: String,
    /// 可选附加数据（入参/出参/耗时/错误详情等）
    #[serde(default)]
    pub data: Option<Value>,
}

/// Memory 相关事件
///
/// 用于记录 Memory 的操作过程（写入/检索/命中等）。
///
/// # 字段说明
///
/// - `phase`: 操作阶段（add/query/hit/error 等）
/// - `data`: 可选附加数据
///
/// # 示例
///
/// ```rust
/// use rucora_core::channel::types::MemoryEvent;
/// use serde_json::json;
///
/// // 记忆写入
/// let add = MemoryEvent {
///     phase: "add".to_string(),
///     data: Some(json!({"key": "user_name", "value": "Alice"})),
/// };
///
/// // 记忆检索
/// let query = MemoryEvent {
///     phase: "query".to_string(),
///     data: Some(json!({"key": "user_name"})),
/// };
///
/// // 记忆命中
/// let hit = MemoryEvent {
///     phase: "hit".to_string(),
///     data: Some(json!({"key": "user_name", "value": "Alice"})),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryEvent {
    /// 操作阶段：add/query/hit/error 等
    pub phase: String,
    /// 可选附加数据
    #[serde(default)]
    pub data: Option<Value>,
}

/// 调试事件
///
/// 用于输出 runtime 内部状态，可被 trace/replay 记录。
///
/// # 字段说明
///
/// - `message`: 调试消息
/// - `data`: 可选结构化字段（如 step 计数、消息数量等）
///
/// # 示例
///
/// ```rust
/// use rucora_core::channel::types::DebugEvent;
/// use serde_json::json;
///
/// let event = DebugEvent {
///     message: "step.start".to_string(),
///     data: Some(json!({
///         "step": 1,
///         "messages_len": 5
///     })),
/// };
///
/// assert_eq!(event.message, "step.start");
/// ```
///
/// # 常见调试消息
///
/// - `step.start`: 步骤开始
/// - `step.end`: 步骤结束
/// - `step.end(no_tool_calls)`: 无工具调用，结束
/// - `tool_call.start`: 工具调用开始
/// - `tool_call.done`: 工具调用完成
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebugEvent {
    /// 调试消息
    pub message: String,
    /// 可选结构化字段
    #[serde(default)]
    pub data: Option<Value>,
}

/// 错误事件
///
/// 用于统一输出 provider/tool/runtime/skill/memory 等错误。
///
/// # 字段说明
///
/// - `kind`: 错误来源分类（provider/tool/runtime/skill/memory/...）
/// - `message`: 人类可读错误信息
/// - `data`: 可选结构化字段（如错误详情、堆栈跟踪等）
///
/// # 示例
///
/// ```rust
/// use rucora_core::channel::types::ErrorEvent;
/// use serde_json::json;
///
/// // Tool 错误
/// let tool_error = ErrorEvent {
///     kind: "tool".to_string(),
///     message: "工具执行失败：文件不存在".to_string(),
///     data: Some(json!({"tool_name": "file_read"})),
/// };
///
/// // Provider 错误
/// let provider_error = ErrorEvent {
///     kind: "provider".to_string(),
///     message: "API 调用失败：连接超时".to_string(),
///     data: None,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorEvent {
    /// 错误来源分类（provider/tool/runtime/skill/memory/...）
    pub kind: String,
    /// 人类可读错误信息
    pub message: String,
    /// 可选结构化字段
    #[serde(default)]
    pub data: Option<Value>,
}

/// Channel 层传递的事件类型
///
/// 这是 rucora 的统一事件模型，用于在组件之间传递消息。
/// 所有事件都支持序列化，便于 Trace 记录和回放。
///
/// # 变体说明
///
/// - `Message(ChatMessage)`: 对话消息事件
/// - `TokenDelta(TokenDeltaEvent)`: Token 流式输出事件
/// - `ToolCall(ToolCall)`: 工具调用事件
/// - `ToolResult(ToolResult)`: 工具结果事件
/// - `Skill(SkillEvent)`: Skill 相关事件
/// - `Memory(MemoryEvent)`: Memory 相关事件
/// - `Debug(DebugEvent)`: 调试事件
/// - `Error(ErrorEvent)`: 错误事件
/// - `Raw(Value)`: 原始事件（用于透传自定义结构）
///
/// # 示例
///
/// ```rust
/// use rucora_core::channel::types::*;
/// use rucora_core::provider::types::{ChatMessage, Role};
///
/// // 消息事件
/// let event = ChannelEvent::Message(ChatMessage {
///     role: Role::Assistant,
///     content: "你好！".to_string(),
///     name: None,
/// });
///
/// // Token 流式事件
/// let event = ChannelEvent::TokenDelta(TokenDeltaEvent {
///     delta: "你".to_string(),
/// });
///
/// // 错误事件
/// let event = ChannelEvent::Error(ErrorEvent {
///     kind: "tool".to_string(),
///     message: "工具执行失败".to_string(),
///     data: None,
/// });
/// ```
///
/// # 序列化
///
/// 所有事件都支持序列化为 JSON，便于 Trace 记录：
///
/// ```rust
/// use rucora_core::channel::types::*;
/// use serde_json;
///
/// let event = ChannelEvent::Debug(DebugEvent {
///     message: "test".to_string(),
///     data: None,
/// });
///
/// let json = serde_json::to_string(&event).unwrap();
/// // {"Debug":{"message":"test","data":null}}
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ChannelEvent {
    /// 对话消息事件
    Message(ChatMessage),

    /// Token 流式输出事件（delta）
    TokenDelta(TokenDeltaEvent),

    /// 工具调用事件
    ToolCall(ToolCall),

    /// 工具结果事件
    ToolResult(ToolResult),

    /// Skill 相关事件
    Skill(SkillEvent),

    /// Memory 相关事件
    Memory(MemoryEvent),

    /// 调试事件
    Debug(DebugEvent),

    /// 错误事件
    Error(ErrorEvent),

    /// 原始事件（用于透传实现层自定义结构）
    Raw(Value),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_delta_event_serialization() {
        let event = TokenDeltaEvent {
            delta: "test".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: TokenDeltaEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn test_debug_event_serialization() {
        let event = DebugEvent {
            message: "test".to_string(),
            data: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: DebugEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn test_channel_event_serialization() {
        let event = ChannelEvent::Debug(DebugEvent {
            message: "test".to_string(),
            data: None,
        });
        let json = serde_json::to_string(&event).unwrap();
        let parsed: ChannelEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, parsed);
    }
}
