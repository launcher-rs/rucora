//! A2A 协议模型定义
//!
//! # 概述
//!
//! 本模块定义 A2A（Agent-to-Agent）协议的核心数据结构，用于 Agent 之间的通信。
//!
//! # 核心类型
//!
//! ## AgentId
//!
//! Agent 的唯一标识符：
//!
//! ```rust
//! use agentkit::a2a::protocol::AgentId;
//!
//! let id = AgentId("agent_123".to_string());
//! ```
//!
//! ## TaskId
//!
//! 任务的唯一标识符：
//!
//! ```rust
//! use agentkit::a2a::protocol::TaskId;
//!
//! let id = TaskId("task_456".to_string());
//! ```
//!
//! ## A2aTask
//!
//! A2A 任务定义，包含：
//! - `id`: 任务 ID
//! - `from`: 发送方 Agent ID
//! - `to`: 接收方 Agent ID
//! - `payload`: 任务负载（JSON）
//!
//! ```rust
//! use agentkit::a2a::protocol::{A2aTask, AgentId, TaskId};
//! use serde_json::json;
//!
//! let task = A2aTask {
//!     id: TaskId("task_1".to_string()),
//!     from: AgentId("agent_a".to_string()),
//!     to: AgentId("agent_b".to_string()),
//!     payload: json!({"action": "process", "data": "input"}),
//! };
//! ```
//!
//! ## A2aResult
//!
//! A2A 任务结果，包含：
//! - `id`: 任务 ID
//! - `from`: 发送方 Agent ID
//! - `to`: 接收方 Agent ID
//! - `output`: 任务输出（JSON）
//!
//! ```rust
//! use agentkit::a2a::protocol::{A2aResult, AgentId, TaskId};
//! use serde_json::json;
//!
//! let result = A2aResult {
//!     id: TaskId("task_1".to_string()),
//!     from: AgentId("agent_b".to_string()),
//!     to: AgentId("agent_a".to_string()),
//!     output: json!({"status": "completed", "data": "output"}),
//! };
//! ```
//!
//! ## A2aCancel
//!
//! A2A 任务取消请求：
//!
//! ```rust
//! use agentkit::a2a::protocol::{A2aCancel, AgentId, TaskId};
//!
//! let cancel = A2aCancel {
//!     id: TaskId("task_1".to_string()),
//!     from: AgentId("agent_a".to_string()),
//!     to: AgentId("agent_b".to_string()),
//! };
//! ```
//!
//! # 通信流程
//!
//! ```text
//! Agent A                    Agent B
//!    │                          │
//!    │─── A2aTask ─────────────>│
//!    │                          │
//!    │                    处理任务
//!    │                          │
//!    │<── A2aResult ────────────│
//!    │                          │
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Agent 的唯一标识符
///
/// # 示例
///
/// ```rust
/// use agentkit::a2a::protocol::AgentId;
///
/// let id = AgentId("agent_123".to_string());
/// assert_eq!(id.0, "agent_123");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentId(pub String);

/// 任务的唯一标识符
///
/// # 示例
///
/// ```rust
/// use agentkit::a2a::protocol::TaskId;
///
/// let id = TaskId("task_456".to_string());
/// assert_eq!(id.0, "task_456");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskId(pub String);

/// A2A 任务定义
///
/// 用于在 Agent 之间传递任务请求。
///
/// # 字段说明
///
/// - `id`: 任务 ID
/// - `from`: 发送方 Agent ID
/// - `to`: 接收方 Agent ID
/// - `payload`: 任务负载（JSON 格式）
///
/// # 示例
///
/// ```rust
/// use agentkit::a2a::protocol::{A2aTask, AgentId, TaskId};
/// use serde_json::json;
///
/// let task = A2aTask {
///     id: TaskId("task_1".to_string()),
///     from: AgentId("agent_a".to_string()),
///     to: AgentId("agent_b".to_string()),
///     payload: json!({"action": "process", "data": "input"}),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct A2aTask {
    /// 任务 ID
    pub id: TaskId,
    /// 发送方 Agent ID
    pub from: AgentId,
    /// 接收方 Agent ID
    pub to: AgentId,
    /// 任务负载（JSON）
    pub payload: Value,
}

/// A2A 任务结果
///
/// 用于在 Agent 之间传递任务执行结果。
///
/// # 字段说明
///
/// - `id`: 任务 ID
/// - `from`: 发送方 Agent ID（执行任务的 Agent）
/// - `to`: 接收方 Agent ID（请求任务的 Agent）
/// - `output`: 任务输出（JSON 格式）
///
/// # 示例
///
/// ```rust
/// use agentkit::a2a::protocol::{A2aResult, AgentId, TaskId};
/// use serde_json::json;
///
/// let result = A2aResult {
///     id: TaskId("task_1".to_string()),
///     from: AgentId("agent_b".to_string()),
///     to: AgentId("agent_a".to_string()),
///     output: json!({"status": "completed", "data": "output"}),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct A2aResult {
    /// 任务 ID
    pub id: TaskId,
    /// 发送方 Agent ID
    pub from: AgentId,
    /// 接收方 Agent ID
    pub to: AgentId,
    /// 任务输出（JSON）
    pub output: Value,
}

/// A2A 任务取消请求
///
/// 用于取消正在执行的任务。
///
/// # 字段说明
///
/// - `id`: 要取消的任务 ID
/// - `from`: 发送方 Agent ID
/// - `to`: 接收方 Agent ID
///
/// # 示例
///
/// ```rust
/// use agentkit::a2a::protocol::{A2aCancel, AgentId, TaskId};
///
/// let cancel = A2aCancel {
///     id: TaskId("task_1".to_string()),
///     from: AgentId("agent_a".to_string()),
///     to: AgentId("agent_b".to_string()),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct A2aCancel {
    /// 任务 ID
    pub id: TaskId,
    /// 发送方 Agent ID
    pub from: AgentId,
    /// 接收方 Agent ID
    pub to: AgentId,
}

/// A2A 消息类型
///
/// 用于在 Agent 之间传递各种类型的消息。
///
/// # 示例
///
/// ```rust
/// use agentkit::a2a::protocol::{A2aMessage, A2aTask, AgentId, TaskId};
/// use serde_json::json;
///
/// let task = A2aTask {
///     id: TaskId("task_1".to_string()),
///     from: AgentId("agent_a".to_string()),
///     to: AgentId("agent_b".to_string()),
///     payload: json!({"action": "process"}),
/// };
///
/// let message = A2aMessage::Task(task);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum A2aMessage {
    /// 任务请求
    Task(A2aTask),
    /// 任务结果
    Result(A2aResult),
    /// 任务取消
    Cancel(A2aCancel),
}
