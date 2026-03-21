//! agentkit-a2a - A2A（Agent-to-Agent）协议支持
//!
//! # 概述
//!
//! 本 crate 提供 A2A（Agent-to-Agent）协议集成支持，用于：
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
//! use agentkit_a2a::client::Client;
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
//! use agentkit_a2a::server::Server;
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
//! use agentkit_a2a::client::Client;
//! use agentkit_a2a::types::Task;
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
//! use agentkit_a2a::server::Server;
//! use agentkit_a2a::types::{Task, TaskStatus, TaskResult};
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
//! use agentkit_a2a::client::Client;
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
//!
//! # 依赖
//!
//! 本 crate 基于 [`ra2a`](https://crates.io/crates/ra2a) 库构建。
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

// 重新导出 ra2a 的所有内容
pub use ra2a::*;

/// A2A 客户端相关 API（来自 `ra2a::client`）
///
/// 用于连接远程 Agent 并发送任务请求。
pub mod client {
    pub use ra2a::client::*;
}

/// A2A 服务端相关 API（来自 `ra2a::server`）
///
/// 用于接收任务请求并执行任务逻辑。
///
/// # 注意
///
/// 服务端通常会依赖具体 Web 框架（例如 `axum`）。
/// 本 crate 仅转导出 `ra2a` 的 server 侧能力，
/// 具体监听端口、路由挂载、中间件等由使用方自行完成。
pub mod server {
    pub use ra2a::server::*;
}

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
