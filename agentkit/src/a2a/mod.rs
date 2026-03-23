//! agentkit-a2a - A2A（Agent-to-Agent）协议支持
//!
//! # 概述
//!
//! 本模块提供 A2A（Agent-to-Agent）协议集成支持，用于：
//! - Agent 之间的通信与协作
//! - 任务委托与结果返回
//! - 多 Agent 系统编排
//!
//! # 什么是 A2A
//!
//! A2A（Agent-to-Agent）协议是一个用于 Agent 间通信的协议，支持：
//! - 任务委托：一个 Agent 可以将任务委托给另一个 Agent
//! - 结果返回：被委托的 Agent 返回执行结果
//! - 状态同步：实时跟踪任务执行状态
//! - 流式输出：支持流式响应
//!
//! # 核心组件
//!
//! ## 客户端（Client）
//!
//! A2A 客户端用于：
//! - 连接远程 Agent
//! - 发送任务请求
//! - 接收任务结果
//!
//! ```rust,no_run
//! use agentkit::a2a::client::Client;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::connect("http://agent-server:8080").await?;
//!
//! // 发送任务
//! let task = client.send_task("process_data", "input data").await?;
//!
//! // 等待结果
//! let result = client.wait_for_result(&task.id).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## 服务端（Server）
//!
//! A2A 服务端用于：
//! - 接收任务请求
//! - 执行任务逻辑
//! - 返回任务结果
//!
//! ```rust,no_run
//! use agentkit::a2a::server::Server;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let server = Server::new()
//!     .register_handler("process_data", |input| async move {
//!         Ok(format!("Processed: {}", input))
//!     })
//!     .bind("127.0.0.1:8080")
//!     .await?;
//!
//! server.serve().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## 协议类型（Types）
//!
//! A2A 协议定义的核心数据结构：
//!
//! - [`types::Message`]: Agent 间消息
//! - [`types::Task`]: 任务定义
//! - [`types::TaskStatus`]: 任务状态
//! - [`types::TaskResult`]: 任务结果
//!
//! # 使用示例
//!
//! ## 客户端使用
//!
//! ```rust,no_run
//! use agentkit::a2a::client::Client;
//! use agentkit::a2a::types::Task;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // 连接 Agent 服务器
//! let client = Client::connect("http://agent-server:8080").await?;
//!
//! // 发送任务
//! let task = client.send_task(Task {
//!     id: "task_123".to_string(),
//!     name: "analyze_data".to_string(),
//!     input: "data to analyze".to_string(),
//!     ..Default::default()
//! }).await?;
//!
//! // 获取状态
//! let status = client.get_task_status(&task.id).await?;
//! println!("任务状态：{:?}", status);
//!
//! // 获取结果
//! let result = client.get_task_result(&task.id).await?;
//! println!("任务结果：{:?}", result);
//! # Ok(())
//! # }
//! ```
//!
//! ## 服务端使用
//!
//! ```rust,no_run
//! use agentkit::a2a::server::Server;
//! use agentkit::a2a::types::{Task, TaskStatus, TaskResult};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // 创建服务器
//! let mut server = Server::new();
//!
//! // 注册任务处理器
//! server.register_handler("analyze", |task: Task| async move {
//!     // 处理任务
//!     let result = format!("Analysis of: {}", task.input);
//!
//!     Ok(TaskResult {
//!         task_id: task.id,
//!         output: result,
//!         status: TaskStatus::Completed,
//!     })
//! });
//!
//! // 启动服务器
//! server.bind("127.0.0.1:8080").await?.serve().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## 多 Agent 协作
//!
//! ```rust,no_run
//! use agentkit::a2a::client::Client;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // 连接多个 Agent
//! let agent1 = Client::connect("http://agent1:8080").await?;
//! let agent2 = Client::connect("http://agent2:8080").await?;
//!
//! // 委托任务给 Agent 1
//! let task1 = agent1.send_task("fetch_data", "").await?;
//! let data = agent1.wait_for_result(&task1.id).await?;
//!
//! // 将结果传递给 Agent 2 处理
//! let task2 = agent2.send_task("process_data", &data.output).await?;
//! let result = agent2.wait_for_result(&task2.id).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # 子模块
//!
//! - [`client`]: A2A 客户端 API
//! - [`server`]: A2A 服务端 API
//! - [`types`]: A2A 协议类型定义
//! - [`protocol`]: A2A 协议模型定义
//! - [`transport`]: A2A 传输层
//!
//! # 依赖
//!
//! 本模块基于 [`ra2a`](https://crates.io/crates/ra2a) 库构建。
//!
//! # Feature 标志
//!
//! - `client`: 启用 A2A 客户端支持
//! - `server`: 启用 A2A 服务器支持
//! - `grpc`: 启用 gRPC 传输支持
//! - `telemetry`: 启用遥测支持
//! - `sql`: 启用 SQL 存储支持
//! - `postgresql`: 启用 PostgreSQL 存储
//! - `mysql`: 启用 MySQL 存储
//! - `sqlite`: 启用 SQLite 存储
//! - `full`: 启用所有功能

use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

// 重新导出 ra2a 的客户端和服务器（避免类型冲突）
pub use ra2a::{client, server};

/// A2A 协议核心数据结构（来自 `ra2a::types`）
///
/// 包含：
/// - [`types::Message`]: Agent 间消息
/// - [`types::Task`]: 任务定义
/// - [`types::TaskStatus`]: 任务状态
/// - [`types::TaskResult`]: 任务结果
/// - [`types::AgentCard`]: Agent 能力描述
pub mod types {
    pub use ra2a::types::*;
}

/// A2A 协议模型定义
pub mod protocol;

/// A2A 传输层
pub mod transport;

/// A2A 工具适配器
///
/// 将 A2A 远程 Agent 适配为本地 Tool 接口
pub struct A2AToolAdapter {
    /// 工具名称
    name: String,
    /// 工具描述
    description: String,
    /// 工具参数定义
    parameters: Value,
    /// A2A 客户端
    client: Arc<ra2a::client::Client>,
}

impl A2AToolAdapter {
    /// 创建新的 A2A 工具适配器
    pub fn new(
        name: String,
        description: String,
        parameters: Value,
        client: ra2a::client::Client,
    ) -> Self {
        Self {
            name,
            description,
            parameters,
            client: Arc::new(client),
        }
    }
}

#[async_trait]
impl crate::core::tool::Tool for A2AToolAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> Option<&str> {
        Some(&self.description)
    }

    fn categories(&self) -> &'static [crate::core::tool::ToolCategory] {
        &[crate::core::tool::ToolCategory::External]
    }

    fn input_schema(&self) -> Value {
        self.parameters.clone()
    }

    async fn call(
        &self,
        input: Value,
    ) -> std::result::Result<Value, crate::core::error::ToolError> {
        use ra2a::types::{Message, MessageSendParams, Part};

        // 从输入中提取 message 字段
        let message_text = input.get("message").and_then(|v| v.as_str()).unwrap_or("");

        // 调试输出
        eprintln!("[A2AToolAdapter] 调用工具：{}", self.name);
        eprintln!("[A2AToolAdapter] 输入：{:?}", input);
        eprintln!("[A2AToolAdapter] 发送消息：{}", message_text);

        // 构建消息
        let msg = Message::user(vec![Part::text(message_text.to_string())]);

        // 发送到 A2A server
        let result = self
            .client
            .send_message(&MessageSendParams::new(msg))
            .await
            .map_err(|e| crate::core::error::ToolError::Message(format!("A2A 调用失败：{}", e)))?;

        // 解析响应
        let response = match result {
            ra2a::types::SendMessageResult::Task(task) => task
                .status
                .message
                .as_ref()
                .and_then(|m| m.text_content())
                .unwrap_or_default()
                .to_string(),
            ra2a::types::SendMessageResult::Message(msg) => {
                msg.text_content().unwrap_or_default().to_string()
            }
        };

        eprintln!("[A2AToolAdapter] 收到响应：{}", response);

        // 返回 JSON 格式结果
        Ok(serde_json::json!({
            "response": response
        }))
    }
}
