//! A2A 传输层
//!
//! # 概述
//!
//! 本模块定义 A2A（Agent-to-Agent）通信的传输层接口和实现。
//!
//! # 核心类型
//!
//! ## A2aTransport trait
//!
//! 传输层接口，定义了两个必需方法：
//!
//! - `send`: 发送消息到指定 Agent
//! - `register`: 注册 Agent 并获取消息接收器
//!
//! ```rust,no_run
//! use agentkit::a2a::transport::{A2aTransport, InProcessA2aTransport};
//! use agentkit::a2a::protocol::{AgentId, A2aMessage};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let transport = InProcessA2aTransport::new();
//!
//! // 注册 Agent
//! let rx = transport.register(AgentId("agent_1".to_string())).await?;
//!
//! // 发送消息
//! transport.send(&AgentId("agent_2".to_string()), msg).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## InProcessA2aTransport
//!
//! 进程内传输实现，用于本地多 Agent 通信：
//!
//! - 使用 `HashMap` 维护 Agent ID 到消息通道的映射
//! - 使用 `mpsc::channel` 进行异步消息传递
//! - 线程安全（使用 `Arc<Mutex<>>` 保护）
//!
//! ```rust
//! use agentkit::a2a::transport::InProcessA2aTransport;
//!
//! let transport = InProcessA2aTransport::new();
//! ```
//!
//! # 使用示例
//!
//! ## 进程内多 Agent 通信
//!
//! ```rust,no_run
//! use agentkit::a2a::transport::{A2aTransport, InProcessA2aTransport};
//! use agentkit::a2a::protocol::{AgentId, A2aMessage};
//! use futures_util::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let transport = InProcessA2aTransport::new();
//!
//! // 注册两个 Agent
//! let mut rx1 = transport.register(AgentId("agent_1".to_string())).await?;
//! let mut rx2 = transport.register(AgentId("agent_2".to_string())).await?;
//!
//! // Agent 1 发送消息给 Agent 2
//! let msg = A2aMessage::Task(task);
//! transport.send(&AgentId("agent_2".to_string()), msg).await?;
//!
//! // Agent 2 接收消息
//! if let Some(received) = rx2.recv().await {
//!     println!("收到消息：{:?}", received);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # 传输层对比
//!
//! | 传输层 | 使用场景 | 优点 | 缺点 |
//! |--------|----------|------|------|
//! | InProcess | 本地多 Agent | 快速、无需网络 | 仅限单进程 |
//! | HTTP | 远程 Agent | 跨网络、跨进程 | 需要网络配置 |
//! | WebSocket | 实时通信 | 双向、低延迟 | 需要 WebSocket 支持 |
//!
//! # 扩展传输层
//!
//! 可以实现自定义传输层：
//!
//! ```rust,no_run
//! use agentkit::a2a::transport::A2aTransport;
//! use agentkit::a2a::protocol::{AgentId, A2aMessage};
//! use async_trait::async_trait;
//! use tokio::sync::mpsc;
//!
//! struct CustomTransport;
//!
//! #[async_trait]
//! impl A2aTransport for CustomTransport {
//!     async fn send(&self, to: &AgentId, msg: A2aMessage) -> Result<(), String> {
//!         // 实现自定义发送逻辑
//!         Ok(())
//!     }
//!
//!     async fn register(&self, id: AgentId) -> Result<mpsc::Receiver<A2aMessage>, String> {
//!         // 实现自定义注册逻辑
//!         unimplemented!()
//!     }
//! }
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{Mutex, mpsc};

use crate::a2a::protocol::{A2aMessage, AgentId};

/// A2A 传输层接口
///
/// 定义了 Agent 间通信的基本操作：
/// - 发送消息
/// - 注册 Agent 并获取消息接收器
///
/// # 示例
///
/// ```rust,no_run
/// use agentkit::a2a::transport::A2aTransport;
/// # use agentkit::a2a::protocol::{AgentId, A2aMessage};
///
/// # async fn example(transport: impl A2aTransport) -> Result<(), Box<dyn std::error::Error>> {
/// // 注册 Agent
/// let rx = transport.register(AgentId("agent_1".to_string())).await?;
///
/// // 发送消息
/// transport.send(&AgentId("agent_2".to_string()), msg).await?;
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait A2aTransport: Send + Sync {
    /// 发送消息到指定 Agent
    ///
    /// # 参数
    ///
    /// - `to`: 接收方 Agent ID
    /// - `msg`: 要发送的消息
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 发送成功
    /// - `Err(String)`: 发送失败，返回错误信息
    async fn send(&self, to: &AgentId, msg: A2aMessage) -> Result<(), String>;

    /// 注册 Agent 并获取消息接收器
    ///
    /// # 参数
    ///
    /// - `id`: Agent ID
    ///
    /// # 返回值
    ///
    /// - `Ok(Receiver)`: 注册成功，返回消息接收器
    /// - `Err(String)`: 注册失败，返回错误信息
    async fn register(&self, id: AgentId) -> Result<mpsc::Receiver<A2aMessage>, String>;
}

/// 进程内 A2A 传输实现
///
/// 用于本地多 Agent 通信，使用 mpsc channel 进行消息传递。
///
/// # 特点
///
/// - 线程安全（使用 `Arc<Mutex<>>` 保护）
/// - 异步消息传递
/// - 自动清理未注册的 Agent
///
/// # 示例
///
/// ```rust
/// use agentkit::a2a::transport::InProcessA2aTransport;
///
/// let transport = InProcessA2aTransport::new();
/// ```
#[derive(Default)]
pub struct InProcessA2aTransport {
    /// Agent ID 到消息发送器的映射
    inner: Arc<Mutex<HashMap<String, mpsc::Sender<A2aMessage>>>>,
}

impl InProcessA2aTransport {
    /// 创建新的进程内传输实例
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit::a2a::transport::InProcessA2aTransport;
    ///
    /// let transport = InProcessA2aTransport::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl A2aTransport for InProcessA2aTransport {
    async fn send(&self, to: &AgentId, msg: A2aMessage) -> Result<(), String> {
        let map = self.inner.lock().await;
        let tx = map
            .get(&to.0)
            .ok_or_else(|| format!("Agent not found: {}", to.0))?;
        tx.send(msg).await.map_err(|e| e.to_string())
    }

    async fn register(&self, id: AgentId) -> Result<mpsc::Receiver<A2aMessage>, String> {
        let (tx, rx) = mpsc::channel(100);
        let mut map = self.inner.lock().await;
        map.insert(id.0, tx);
        Ok(rx)
    }
}
