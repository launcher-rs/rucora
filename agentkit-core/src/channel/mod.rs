//! Channel（通信渠道）抽象模块
//!
//! # 概述
//!
//! Channel（通信渠道）用于在 Agent 的各个组件之间传递事件，以及将事件对接到外部系统：
//! - CLI（命令行界面）
//! - HTTP/WebSocket（Web 服务）
//! - IM/机器人平台（如钉钉、企业微信）
//! - Trace 记录（调试和审计）
//!
//! 在 core 层，我们只定义事件的类型和接口，不绑定具体的传输实现。
//!
//! # 核心类型
//!
//! ## ChannelEvent
//!
//! [`types::ChannelEvent`] 是统一的事件类型，支持：
//!
//! - `Message`: 对话消息事件
//! - `TokenDelta`: Token 流式输出事件
//! - `ToolCall`: 工具调用事件
//! - `ToolResult`: 工具结果事件
//! - `Skill`: 技能相关事件
//! - `Memory`: 记忆相关事件
//! - `Debug`: 调试事件
//! - `Error`: 错误事件
//! - `Raw`: 原始事件（用于透传自定义数据）
//!
//! ## ChannelObserver trait
//!
//! [`ChannelObserver`] trait 用于观测渠道中的事件：
//!
//! ```rust
//! use agentkit_core::channel::{ChannelObserver, ChannelEvent};
//!
//! struct LoggingObserver;
//!
//! impl ChannelObserver for LoggingObserver {
//!     fn on_event(&self, event: ChannelEvent) {
//!         println!("收到事件：{:?}", event);
//!     }
//! }
//! ```
//!
//! ## Channel trait
//!
//! [`Channel`] trait 定义了事件发送和订阅的接口（可选实现）。
//!
//! # 事件流转
//!
//! ```text
//! ┌──────────────┐
//! │  User Input  │
//! └──────┬───────┘
//!        │
//!        ▼
//! ┌─────────────────────────────────────────┐
//! │           Agent                         │
//! │                                         │
//! │  ┌─────────────────────────────────┐   │
//! │  │  Provider (LLM API)             │   │
//! │  └────────────┬────────────────────┘   │
//! │               │                         │
//! │               ▼                         │
//! │  ┌─────────────────────────────────┐   │
//! │  │  Tool Execution                 │   │
//! │  └────────────┬────────────────────┘   │
//! │               │                         │
//! └───────────────┼─────────────────────────┘
//!                 │
//!                 ▼
//!      ┌──────────────────────┐
//!      │   Channel Events     │
//!      │                      │
//!      │  - TokenDelta        │──► 流式输出到 UI
//!      │  - ToolCall          │──► 审计日志
//!      │  - ToolResult        │──► Trace 记录
//!      │  - Message           │──► 对话历史
//!      │  - Error             │──► 错误处理
//!      └──────────────────────┘
//! ```
//!
//! # 使用示例
//!
//! ## 创建和发送事件
//!
//! ```rust
//! use agentkit_core::channel::types::*;
//! use agentkit_core::provider::types::{ChatMessage, Role};
//! use serde_json::json;
//!
//! // 创建消息事件
//! let message_event = ChannelEvent::Message(ChatMessage {
//!     role: Role::Assistant,
//!     content: "你好！".to_string(),
//!     name: None,
//! });
//!
//! // 创建 Token 流式事件
//! let token_event = ChannelEvent::TokenDelta(TokenDeltaEvent {
//!     delta: "你".to_string(),
//! });
//!
//! // 创建调试事件
//! let debug_event = ChannelEvent::Debug(DebugEvent {
//!     message: "step.start".to_string(),
//!     data: Some(json!({"step": 1})),
//! });
//!
//! // 创建错误事件
//! let error_event = ChannelEvent::Error(ErrorEvent {
//!     kind: "tool".to_string(),
//!     message: "工具执行失败".to_string(),
//!     data: None,
//! });
//! ```
//!
//! ## 事件匹配处理
//!
//! ```rust
//! use agentkit_core::channel::types::*;
//!
//! fn handle_event(event: ChannelEvent) {
//!     match event {
//!         ChannelEvent::TokenDelta(delta) => {
//!             print!("{}", delta.delta);
//!         }
//!         ChannelEvent::ToolCall(call) => {
//!             println!("调用工具：{}", call.name);
//!         }
//!         ChannelEvent::ToolResult(result) => {
//!             println!("工具结果：{}", result.output);
//!         }
//!         ChannelEvent::Message(msg) => {
//!             println!("消息：{}", msg.content);
//!         }
//!         ChannelEvent::Error(err) => {
//!             eprintln!("错误 [{}]: {}", err.kind, err.message);
//!         }
//!         _ => {}
//!     }
//! }
//! ```
//!
//! ## 事件序列化（用于 Trace）
//!
//! ```rust,no_run
//! use agentkit_core::channel::types::*;
//! use serde_json;
//!
//! let event = ChannelEvent::Debug(DebugEvent {
//!     message: "test".to_string(),
//!     data: None,
//! });
//!
//! // 序列化为 JSON
//! let json = serde_json::to_string(&event).unwrap();
//!
//! // 反序列化
//! let parsed: ChannelEvent = serde_json::from_str(&json).unwrap();
//! ```
//!
//! # 事件类型说明
//!
//! ## Message
//!
//! 对话消息事件，用于传递助手或用户的消息。
//!
//! ## TokenDelta
//!
//! Token 流式输出事件，用于传递 LLM 的流式响应。
//!
//! ## ToolCall / ToolResult
//!
//! 工具调用和结果事件，用于记录和传递工具的执行过程。
//!
//! ## Skill / Memory
//!
//! 技能和记忆相关事件，用于记录高级功能的执行。
//!
//! ## Debug
//!
//! 调试事件，用于记录运行时内部状态。
//!
//! ## Error
//!
//! 错误事件，用于统一传递各组件的错误信息。
//!
//! ## Raw
//!
//! 原始事件，用于透传自定义数据。

/// Channel trait 定义（可选实现）
pub mod r#trait;

/// Channel 事件类型定义
pub mod types;

/// Hook（钩子）优先级系统
pub mod hooks;

/// 双轨指标系统（ObserverEvent 与 ObserverMetric 分离）
pub mod metrics;

/// Channel 观测器 trait
///
/// 用于观测 Agent 执行过程中的事件。
///
/// # 说明
///
/// - 统一复用 [`ChannelEvent`] 作为事件载体
/// - 采用同步方法，便于在热路径上最小开销调用
/// - 需要异步处理时，建议实现方自行把事件投递到队列/channel
#[async_trait::async_trait]
pub trait ChannelObserver: Send + Sync {
    /// 接收事件
    ///
    /// # 参数
    ///
    /// - `event`: 事件内容
    ///
    /// # 说明
    ///
    /// 该方法在热路径上被调用，应该：
    /// - 快速返回
    /// - 避免阻塞
    /// - 避免抛出异常
    fn on_event(&self, event: ChannelEvent);
}

/// 默认空实现（丢弃所有观测事件）
///
/// 用于不需要观测功能的场景。
#[derive(Debug, Default, Clone)]
pub struct NoopChannelObserver;

impl ChannelObserver for NoopChannelObserver {
    fn on_event(&self, _event: ChannelEvent) {}
}

// 为了向后兼容，将 RuntimeObserver 作为 ChannelObserver 的别名
// 注意：这是一个类型别名，实际使用时应该使用 dyn ChannelObserver
#[deprecated(since = "0.2.0", note = "使用 ChannelObserver 代替")]
pub trait RuntimeObserver: ChannelObserver {}

// 为所有实现 ChannelObserver 的类型自动实现 RuntimeObserver
#[allow(deprecated)]
impl<T: ChannelObserver> RuntimeObserver for T {}

#[deprecated(since = "0.2.0", note = "使用 NoopChannelObserver 代替")]
pub type NoopRuntimeObserver = NoopChannelObserver;

/// 重新导出 channel 相关 trait，方便 `agentkit_core::channel::*` 使用
pub use r#trait::*;

/// 重新导出 channel 相关类型，方便使用
pub use types::*;

/// 重新导出 hook 相关类型
pub use hooks::{
    CombinedHook, HookPriority, HookRegistry, HookResult, LoggingVoidHook, ModifyingHook,
    ValidationModifyingHook, VoidHook,
};

/// 重新导出双轨指标相关类型
pub use metrics::{
    DualTrackObserver, EventBuilder, LogLevel, LoggingObserver, MetricAggregator, MetricLabels,
    MultiObserver, ObserverEvent, ObserverMetric, VerboseObserver,
};
