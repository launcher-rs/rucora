//! Agent（智能体）相关的类型定义
//!
//! # 概述
//!
//! 本模块定义了 Agent 的输入和输出类型，用于在运行时和外部系统之间传递数据。
//!
//! # 核心类型
//!
//! ## AgentInput
//!
//! [`AgentInput`] 是 Agent 的输入类型，包含：
//! - `messages`: 消息历史（对话历史或任务描述）
//! - `metadata`: 透传元数据（可选）
//!
//! ## AgentOutput
//!
//! [`AgentOutput`] 是 Agent 的输出类型，包含：
//! - `message`: 最终回复消息
//! - `tool_results`: 工具执行结果列表（如果运行时支持工具）
//!
//! # 数据流
//!
//! ```text
//! 用户输入
//!    │
//!    ▼
//! ┌─────────────────┐
//! │   AgentInput    │
//! │  - messages     │
//! │  - metadata     │
//! └────────┬────────┘
//!          │
//!          ▼
//!   ┌──────────────┐
//!   │   Runtime    │
//!   │   (执行)     │
//!   └──────────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │   AgentOutput   │
//! │  - message      │
//! │  - tool_results │
//! └────────┬────────┘
//!          │
//!          ▼
//!      用户回复
//! ```
//!
//! # 使用示例
//!
//! ## 创建输入
//!
//! ```rust
//! use agentkit_core::agent::types::{AgentInput, AgentOutput};
//! use agentkit_core::provider::types::{ChatMessage, Role};
//! use serde_json::json;
//!
//! let input = AgentInput {
//!     messages: vec![
//!         ChatMessage {
//!             role: Role::System,
//!             content: "你是一个有用的助手".to_string(),
//!             name: None,
//!         },
//!         ChatMessage {
//!             role: Role::User,
//!             content: "你好".to_string(),
//!             name: None,
//!         },
//!     ],
//!     metadata: Some(json!({
//!         "user_id": "123",
//!         "session_id": "abc"
//!     })),
//! };
//! ```
//!
//! ## 处理输出
//!
//! ```rust
//! use agentkit_core::agent::types::AgentOutput;
//! use agentkit_core::provider::types::{ChatMessage, Role};
//!
//! fn handle_output(output: AgentOutput) {
//!     // 处理最终回复
//!     println!("助手回复：{}", output.message.content);
//!
//!     // 处理工具结果
//!     for result in output.tool_results {
//!         println!("工具结果：{}", result.output);
//!     }
//! }
//! ```
//!
//! ## 序列化/反序列化
//!
//! ```rust
//! use agentkit_core::agent::types::{AgentInput, AgentOutput};
//! use serde_json;
//!
//! # fn example(input: AgentInput, output: AgentOutput) {
//! // 序列化
//! let input_json = serde_json::to_string(&input).unwrap();
//! let output_json = serde_json::to_string(&output).unwrap();
//!
//! // 反序列化
//! let parsed_input: AgentInput = serde_json::from_str(&input_json).unwrap();
//! let parsed_output: AgentOutput = serde_json::from_str(&output_json).unwrap();
//! # }
//! ```
//!
//! # 与 Runtime 的关系
//!
//! [`AgentInput`] 和 [`AgentOutput`] 是 [`Runtime`](crate::runtime::Runtime) trait 的统一输入输出类型：
//!
//! ```rust,no_run
//! use agentkit_core::runtime::Runtime;
//! use agentkit_core::agent::types::{AgentInput, AgentOutput};
//!
//! # async fn example(runtime: &dyn Runtime) -> Result<(), Box<dyn std::error::Error>> {
//! let input = AgentInput {
//!     messages: vec![],
//!     metadata: None,
//! };
//!
//! let output = runtime.run(input).await?;
//! # Ok(())
//! # }
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{provider::types::ChatMessage, tool::types::ToolResult};

/// Agent 输入类型
///
/// 用于向运行时传递用户输入和上下文信息。
///
/// # 字段说明
///
/// - `messages`: 输入消息列表，通常为对话历史或任务描述
/// - `metadata`: 透传元数据，可选，用于传递额外信息（如用户 ID、会话 ID 等）
///
/// # 示例
///
/// ```rust
/// use agentkit_core::agent::types::AgentInput;
/// use agentkit_core::provider::types::{ChatMessage, Role};
/// use serde_json::json;
///
/// // 简单输入
/// let input = AgentInput {
///     messages: vec![
///         ChatMessage {
///             role: Role::User,
///             content: "你好".to_string(),
///             name: None,
///         },
///     ],
///     metadata: None,
/// };
///
/// // 带元数据的输入
/// let input = AgentInput {
///     messages: vec![],
///     metadata: Some(json!({
///         "user_id": "123",
///         "language": "zh-CN"
///     })),
/// };
/// ```
///
/// # 消息历史格式
///
/// 消息历史通常按以下顺序组织：
///
/// 1. 系统提示词（可选）
/// 2. 用户消息
/// 3. 助手回复
/// 4. 用户消息
/// 5. ...
///
/// ```rust
/// use agentkit_core::provider::types::{ChatMessage, Role};
///
/// let messages = vec![
///     // 1. 系统提示词
///     ChatMessage {
///         role: Role::System,
///         content: "你是一个有用的助手".to_string(),
///         name: None,
///     },
///     // 2. 用户消息
///     ChatMessage {
///         role: Role::User,
///         content: "你好".to_string(),
///         name: None,
///     },
/// ];
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentInput {
    /// 输入消息列表
    ///
    /// 通常为对话历史或任务描述。
    /// 消息按时间顺序排列，最新的消息在最后。
    pub messages: Vec<ChatMessage>,

    /// 透传元数据
    ///
    /// 可选字段，用于传递额外信息。
    /// 常见用途：
    /// - 用户 ID
    /// - 会话 ID
    /// - 请求 ID
    /// - 自定义配置
    ///
    /// 该字段会被透传给 Provider，但不会影响对话内容。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// Agent 输出类型
///
/// 用于从运行时接收最终回复和工具执行结果。
///
/// # 字段说明
///
/// - `message`: 最终回复消息（助手的回答）
/// - `tool_results`: 工具执行结果列表（如果运行时支持工具）
///
/// # 示例
///
/// ```rust
/// use agentkit_core::agent::types::AgentOutput;
/// use agentkit_core::provider::types::{ChatMessage, Role};
///
/// let output = AgentOutput {
///     message: ChatMessage {
///         role: Role::Assistant,
///         content: "你好！有什么可以帮助你的？".to_string(),
///         name: None,
///     },
///     tool_results: vec![],
/// };
///
/// assert_eq!(output.message.role, Role::Assistant);
/// ```
///
/// # 带工具结果的输出
///
/// 当运行时执行了工具调用时，`tool_results` 会包含所有工具的执行结果：
///
/// ```rust
/// use agentkit_core::agent::types::AgentOutput;
/// use agentkit_core::provider::types::{ChatMessage, Role};
/// use agentkit_core::tool::types::ToolResult;
/// use serde_json::json;
///
/// let output = AgentOutput {
///     message: ChatMessage {
///         role: Role::Assistant,
///         content: "已为你查询了天气".to_string(),
///         name: None,
///     },
///     tool_results: vec![
///         ToolResult {
///             tool_call_id: "call_123".to_string(),
///             output: json!({
///                 "location": "Beijing",
///                 "temperature": 25,
///                 "condition": "晴朗"
///             }),
///         },
///     ],
/// };
/// ```
///
/// # 处理输出
///
/// ```rust
/// use agentkit_core::agent::types::AgentOutput;
///
/// fn handle_output(output: AgentOutput) {
///     // 处理最终回复
///     println!("助手回复：{}", output.message.content);
///
///     // 处理工具结果
///     for result in &output.tool_results {
///         println!("工具结果：{}", result.output);
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentOutput {
    /// 最终回复消息
    ///
    /// 这是助手对用户的最终回答。
    /// 角色通常为 `Role::Assistant`。
    pub message: ChatMessage,

    /// 工具执行结果列表
    ///
    /// 如果运行时支持工具调用，该字段会包含所有工具的执行结果。
    /// 如果运行时不支持工具或未执行工具，该字段为空。
    ///
    /// 该字段主要用于：
    /// - 调试和日志记录
    /// - 向用户展示工具执行过程
    /// - 审计和追踪
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_results: Vec<ToolResult>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::types::Role;

    #[test]
    fn test_agent_input_serialization() {
        let input = AgentInput {
            messages: vec![ChatMessage {
                role: Role::User,
                content: "你好".to_string(),
                name: None,
            }],
            metadata: None,
        };

        let json = serde_json::to_string(&input).unwrap();
        let parsed: AgentInput = serde_json::from_str(&json).unwrap();
        assert_eq!(input, parsed);
    }

    #[test]
    fn test_agent_input_with_metadata() {
        let input = AgentInput {
            messages: vec![],
            metadata: Some(serde_json::json!({"key": "value"})),
        };

        assert!(input.metadata.is_some());
    }

    #[test]
    fn test_agent_output_serialization() {
        let output = AgentOutput {
            message: ChatMessage {
                role: Role::Assistant,
                content: "你好".to_string(),
                name: None,
            },
            tool_results: vec![],
        };

        let json = serde_json::to_string(&output).unwrap();
        let parsed: AgentOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(output, parsed);
    }

    #[test]
    fn test_agent_output_empty_tool_results() {
        let output = AgentOutput {
            message: ChatMessage {
                role: Role::Assistant,
                content: "你好".to_string(),
                name: None,
            },
            tool_results: vec![],
        };

        // 空 tool_results 应该被跳过序列化
        let json = serde_json::to_string(&output).unwrap();
        assert!(!json.contains("tool_results"));
    }
}
