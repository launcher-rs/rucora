//! agentkit-mcp - MCP（Model Context Protocol）集成
//!
//! # 概述
//!
//! 本模块提供 MCP（Model Context Protocol）集成支持，用于：
//! - 连接 MCP 服务器
//! - 将 MCP 工具转换为 agentkit 的 Tool trait
//! - 统一 MCP 工具调用接口
//!
//! # 什么是 MCP
//!
//! MCP（Model Context Protocol）是一个开放协议，用于：
//! - 标准化 AI 模型与外部工具的交互
//! - 提供工具发现、调用、结果返回的统一格式
//! - 支持多种传输层（stdio、HTTP、WebSocket 等）
//!
//! # 核心组件
//!
//! ## 传输层（Transport）
//!
//! 支持多种 MCP 传输方式：
//!
//! - [`StdioTransport`]: 标准输入输出（用于本地进程）
//! - [`StreamableHttpTransport`]: HTTP 流式传输
//!
//! ## 协议层（Protocol）
//!
//! MCP 协议定义的消息类型：
//!
//! - 工具列表请求/响应
//! - 工具调用请求/响应
//! - 错误处理
//!
//! ## 工具适配（Tool Adapter）
//!
//! [`McpToolAdapter`] 将远程 MCP 工具包装为 [`agentkit_core::tool::Tool`]：
//!
//! ```rust
//! use agentkit::mcp::McpToolAdapter;
//! use agentkit_core::tool::Tool;
//!
//! // MCP 工具可以直接作为 agentkit 的 Tool 使用
//! let adapter: McpToolAdapter = ...;
//! let result = adapter.call(input).await?;
//! ```
//!
//! # 使用示例
//!
//! ## 连接 MCP 服务器
//!
//! ```rust,no_run
//! use agentkit::mcp::{McpClient, StdioTransport};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // 创建传输层
//! let transport = StdioTransport::new("mcp-server");
//!
//! // 创建客户端
//! let client = McpClient::connect(transport).await?;
//!
//! // 列出可用工具
//! let tools = client.list_tools().await?;
//!
//! for tool in tools {
//!     println!("工具：{}", tool.name);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## 调用 MCP 工具
//!
//! ```rust,no_run
//! use agentkit::mcp::{McpClient, StdioTransport};
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let transport = StdioTransport::new("mcp-server");
//! let client = McpClient::connect(transport).await?;
//!
//! // 调用工具
//! let result = client.call_tool(
//!     "my_tool",
//!     json!({"param": "value"})
//! ).await?;
//!
//! println!("结果：{}", result);
//! # Ok(())
//! # }
//! ```
//!
//! ## 将 MCP 工具转换为 agentkit Tool
//!
//! ```rust,no_run
//! use agentkit::mcp::{McpClient, McpToolAdapter, StdioTransport};
//! use agentkit_core::tool::Tool;
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let transport = StdioTransport::new("mcp-server");
//! let client = McpClient::connect(transport).await?;
//!
//! // 获取 MCP 工具定义
//! let tools = client.list_tools().await?;
//! let mcp_tool = tools.into_iter().next().unwrap();
//!
//! // 创建适配器
//! let adapter = McpToolAdapter::new(client.clone(), mcp_tool);
//!
//! // 现在可以作为 agentkit Tool 使用
//! let result = adapter.call(serde_json::json!({})).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # 子模块
//!
//! - [`protocol`]: MCP 协议模型类型
//! - [`tool`]: MCP 工具适配器
//! - [`transport`]: MCP 传输层
//!
//! # 依赖
//!
//! 本模块基于 [`rmcp`](https://crates.io/crates/rmcp) 库构建。
//!
//! # Feature 标志
//!
//! - `client`: 启用 MCP 客户端支持
//! - `server`: 启用 MCP 服务器支持
//! - `transport-streamable-http-client`: HTTP 流式传输客户端

/// MCP 协议模型
pub mod protocol;

/// MCP 工具适配器
pub mod tool;

/// MCP 传输层
pub mod transport;

/// `rmcp` 服务扩展 trait（直接转导出）
pub use rmcp::ServiceExt;

/// MCP 协议模型类型（从 `rmcp::model` 转导出）
pub use protocol::*;

/// MCP 工具适配（将远端 MCP 工具包装为 `agentkit-core` 的 Tool）
pub use tool::*;

/// MCP 传输层（从 `rmcp::transport` 转导出）
pub use transport::*;
