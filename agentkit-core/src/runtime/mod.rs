//! Runtime（运行时）抽象模块
//!
//! # 概述
//!
//! Runtime（运行时）是 Agent 的执行引擎，负责：
//! - 调用 LLM Provider 进行对话
//! - 执行工具调用循环（Tool-Calling Loop）
//! - 管理对话历史和上下文
//! - 支持流式和非流式两种执行模式
//!
//! 本模块定义了运行时的最小抽象接口，不绑定任何具体实现。
//!
//! # 核心类型
//!
//! ## Runtime trait
//!
//! [`Runtime`] trait 定义了运行时的最小能力：
//!
//! ```rust,no_run
//! use agentkit_core::runtime::Runtime;
//! use agentkit_core::agent::types::{AgentInput, AgentOutput};
//! use agentkit_core::error::AgentError;
//!
//! # async fn example(runtime: &dyn Runtime, input: AgentInput) -> Result<(), AgentError> {
//! // 执行一次任务
//! let output = runtime.run(input).await?;
//! println!("最终回复：{}", output.message.content);
//! # Ok(())
//! # }
//! ```
//!
//! ## RuntimeObserver trait
//!
//! [`RuntimeObserver`] trait 用于观测运行时的事件：
//!
//! ```rust
//! use agentkit_core::runtime::{RuntimeObserver, NoopRuntimeObserver};
//! use agentkit_core::channel::types::ChannelEvent;
//!
//! // 使用默认的 Noop 观测器
//! let observer = NoopRuntimeObserver;
//!
//! // 或实现自定义观测器
//! struct MyObserver;
//!
//! impl RuntimeObserver for MyObserver {
//!     fn on_event(&self, event: ChannelEvent) {
//!         println!("收到事件：{:?}", event);
//!     }
//! }
//! ```
//!
//! # 运行时的职责
//!
//! Runtime 通常负责以下任务：
//!
//! ```text
//! 1. 接收用户输入（AgentInput）
//!    │
//!    ▼
//! 2. 添加系统提示词（如果有）
//!    │
//!    ▼
//! 3. 调用 LLM Provider（带工具定义）
//!    │
//!    ▼
//! 4. 检查是否有工具调用
//!    │
//!    ├─ 无工具调用 ──► 返回最终回复
//!    │
//!    ▼
//! 5. 有工具调用
//!    │
//!    ▼
//! 6. 执行策略检查（Policy Check）
//!    │
//!    ▼
//! 7. 执行工具（支持并发）
//!    │
//!    ▼
//! 8. 将工具结果添加到对话历史
//!    │
//!    ▼
//! 9. 回到步骤 3，继续循环
//!    │
//!    ▼
//! 10. 达到终止条件 ──► 返回最终回复
//! ```
//!
//! # 实现示例
//!
//! ## 简单运行时
//!
//! ```rust,no_run
//! use agentkit_core::runtime::Runtime;
//! use agentkit_core::agent::types::{AgentInput, AgentOutput};
//! use agentkit_core::error::AgentError;
//! use async_trait::async_trait;
//!
//! struct SimpleRuntime;
//!
//! #[async_trait]
//! impl Runtime for SimpleRuntime {
//!     async fn run(&self, input: AgentInput) -> Result<AgentOutput, AgentError> {
//!         // 简单实现：直接返回固定回复
//!         Ok(AgentOutput {
//!             message: agentkit_core::provider::types::ChatMessage {
//!                 role: agentkit_core::provider::types::Role::Assistant,
//!                 content: "你好！我是助手。".to_string(),
//!                 name: None,
//!             },
//!             tool_results: vec![],
//!         })
//!     }
//! }
//! ```
//!
//! ## 带工具调用的运行时
//!
//! ```rust,no_run
//! use agentkit_core::runtime::Runtime;
//! use agentkit_core::agent::types::{AgentInput, AgentOutput};
//! use agentkit_core::error::AgentError;
//! use async_trait::async_trait;
//!
//! struct ToolCallingRuntime {
//!     // provider: Arc<dyn LlmProvider>,
//!     // tools: ToolRegistry,
//!     // max_steps: usize,
//! }
//!
//! #[async_trait]
//! impl Runtime for ToolCallingRuntime {
//!     async fn run(&self, input: AgentInput) -> Result<AgentOutput, AgentError> {
//!         // 实现工具调用循环
//!         // 1. 调用 LLM
//!         // 2. 检查工具调用
//!         // 3. 执行工具
//!         // 4. 循环直到完成
//!         unimplemented!()
//!     }
//! }
//! ```
//!
//! # 观测器模式
//!
//! RuntimeObserver 用于在运行时发出事件时进行观测：
//!
//! ```rust
//! use agentkit_core::runtime::RuntimeObserver;
//! use agentkit_core::channel::types::ChannelEvent;
//! use std::sync::Arc;
//! use std::sync::Mutex;
//!
//! // 收集所有事件的观测器
//! struct EventCollector {
//!     events: Arc<Mutex<Vec<ChannelEvent>>>,
//! }
//!
//! impl RuntimeObserver for EventCollector {
//!     fn on_event(&self, event: ChannelEvent) {
//!         let mut events = self.events.lock().unwrap();
//!         events.push(event);
//!     }
//! }
//! ```
//!
//! # 与 DefaultRuntime 的关系
//!
//! `agentkit-runtime` crate 提供了 [`DefaultRuntime`](https://docs.rs/agentkit-runtime/latest/agentkit_runtime/struct.DefaultRuntime.html) 实现：
//! - 支持完整的 tool-calling loop
//! - 支持流式输出
//! - 支持工具策略
//! - 支持观测器协议
//!
//! 本模块只定义抽象接口，具体实现请参考 `agentkit-runtime`。

use async_trait::async_trait;

use crate::{
    agent::types::{AgentInput, AgentOutput},
    channel::types::ChannelEvent,
    error::AgentError,
};

/// Runtime 统一观测协议
///
/// 接收 runtime 发出的事件，用于：
/// - UI 更新
/// - Trace 记录
/// - 指标收集
/// - 审计日志
///
/// # 说明
///
/// - 统一复用 [`ChannelEvent`] 作为事件载体
/// - Runtime 在关键节点发出事件
/// - 该 trait 采用同步方法，便于在热路径上最小开销调用
/// - 需要异步处理时，建议实现方自行把事件投递到队列/channel
///
/// # 示例
///
/// ```rust
/// use agentkit_core::runtime::RuntimeObserver;
/// use agentkit_core::channel::types::ChannelEvent;
///
/// struct LoggingObserver;
///
/// impl RuntimeObserver for LoggingObserver {
///     fn on_event(&self, event: ChannelEvent) {
///         println!("收到事件：{:?}", event);
///     }
/// }
/// ```
///
/// # 事件类型
///
/// 观测器可以接收以下类型的事件：
/// - `Message`: 对话消息
/// - `TokenDelta`: Token 流式输出
/// - `ToolCall`: 工具调用
/// - `ToolResult`: 工具结果
/// - `Debug`: 调试信息
/// - `Error`: 错误信息
pub trait RuntimeObserver: Send + Sync {
    /// 接收事件
    ///
    /// # 参数
    ///
    /// - `event`: 事件内容
    ///
    /// # 说明
    ///
    /// 该方法在运行时热路径上被调用，应该：
    /// - 快速返回
    /// - 避免阻塞
    /// - 避免抛出异常
    ///
    /// 需要异步处理时，建议将事件投递到队列：
    ///
    /// ```rust
    /// use agentkit_core::runtime::RuntimeObserver;
    /// use agentkit_core::channel::types::ChannelEvent;
    /// use tokio::sync::mpsc;
    ///
    /// struct AsyncObserver {
    ///     sender: mpsc::UnboundedSender<ChannelEvent>,
    /// }
    ///
    /// impl RuntimeObserver for AsyncObserver {
    ///     fn on_event(&self, event: ChannelEvent) {
    ///         // 投递到队列，异步处理
    ///         let _ = self.sender.send(event);
    ///     }
    /// }
    /// ```
    fn on_event(&self, event: ChannelEvent);
}

/// 默认空实现（丢弃所有观测事件）
///
/// 用于不需要观测功能的场景。
///
/// # 示例
///
/// ```rust
/// use agentkit_core::runtime::{RuntimeObserver, NoopRuntimeObserver};
/// use agentkit_core::channel::types::ChannelEvent;
///
/// let observer = NoopRuntimeObserver;
///
/// // 不会有任何效果
/// observer.on_event(ChannelEvent::Debug(
///     agentkit_core::channel::types::DebugEvent {
///         message: "test".to_string(),
///         data: None,
///     }
/// ));
/// ```
#[derive(Debug, Default, Clone)]
pub struct NoopRuntimeObserver;

impl RuntimeObserver for NoopRuntimeObserver {
    fn on_event(&self, _event: ChannelEvent) {}
}

/// Runtime（运行时）trait
///
/// 定义运行时的最小能力接口。
///
/// # 设计意图
///
/// - `agentkit-core` 只定义"运行时需要满足的最小能力"，不绑定任何具体实现
/// - `agentkit-runtime` 提供默认实现（[`DefaultRuntime`](https://docs.rs/agentkit-runtime/latest/agentkit_runtime/struct.DefaultRuntime.html)）
/// - 业务方也可以按该 trait 自定义 runtime（例如：加入 tracing、限流、并发调度、多 agent 编排等）
///
/// # 注意事项
///
/// - 这里复用 `AgentInput/AgentOutput` 作为统一输入输出，以保持 core 层类型稳定
/// - Runtime 的实现内部可以自由决定如何组织 agent loop（ReAct、tool-calling 等）
///
/// # 示例
///
/// ```rust,no_run
/// use agentkit_core::runtime::Runtime;
/// use agentkit_core::agent::types::{AgentInput, AgentOutput};
/// use agentkit_core::error::AgentError;
///
/// # async fn example(runtime: &dyn Runtime) -> Result<(), AgentError> {
/// let input = AgentInput {
///     messages: vec![],
///     metadata: None,
/// };
///
/// let output = runtime.run(input).await?;
/// println!("回复：{}", output.message.content);
/// # Ok(())
/// # }
/// ```
///
/// # 实现说明
///
/// 典型的 `run` 方法实现会：
///
/// 1. 构造消息历史
/// 2. 调用 provider
/// 3. 解析工具调用并执行工具
/// 4. 循环直到得到最终回答
///
/// ```text
/// ┌─────────────────────────────────────────┐
/// │           Runtime::run()                │
/// ├─────────────────────────────────────────┤
/// │  1. 添加系统提示词                       │
/// │     │                                   │
/// │     ▼                                   │
/// │  2. 调用 LLM Provider                    │
/// │     │                                   │
/// │     ▼                                   │
/// │  3. 有工具调用？                         │
/// │     │                                   │
/// │     ├─ 否 ──► 返回结果                  │
/// │     │                                   │
/// │     ▼ 是                                │
/// │  4. 执行工具（Policy Check + 执行）      │
/// │     │                                   │
/// │     ▼                                   │
/// │  5. 添加工具结果到历史                   │
/// │     │                                   │
/// │     └───────► 回到步骤 2                │
/// └─────────────────────────────────────────┘
/// ```
#[async_trait]
pub trait Runtime: Send + Sync {
    /// 执行一次任务
    ///
    /// # 参数
    ///
    /// - `input`: 输入，包含消息历史和元数据
    ///
    /// # 返回值
    ///
    /// - `Ok(AgentOutput)`: 执行成功，返回最终回复
    /// - `Err(AgentError)`: 执行失败，返回错误
    ///
    /// # 典型实现
    ///
    /// 典型实现可能会：
    ///
    /// 1. 构造消息历史
    /// 2. 调用 provider
    /// 3. 解析工具调用并执行工具
    /// 4. 循环直到得到最终回答
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use agentkit_core::runtime::Runtime;
    /// use agentkit_core::agent::types::{AgentInput, AgentOutput};
    /// use agentkit_core::error::AgentError;
    /// use async_trait::async_trait;
    ///
    /// struct SimpleRuntime;
    ///
    /// #[async_trait]
    /// impl Runtime for SimpleRuntime {
    ///     async fn run(&self, input: AgentInput) -> Result<AgentOutput, AgentError> {
    ///         // 简单实现
    ///         Ok(AgentOutput {
    ///             message: agentkit_core::provider::types::ChatMessage {
    ///                 role: agentkit_core::provider::types::Role::Assistant,
    ///                 content: "你好！".to_string(),
    ///                 name: None,
    ///             },
    ///             tool_results: vec![],
    ///         })
    ///     }
    /// }
    /// ```
    async fn run(&self, input: AgentInput) -> Result<AgentOutput, AgentError>;
}
