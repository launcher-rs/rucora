//! MCP 协议模型定义
//!
//! # 概述
//!
//! 该模块主要用于转导出 `rmcp::model` 中的类型，便于上层以 `rucora_mcp::protocol::*`
//! 的方式引用，而不需要直接依赖/引用 `rmcp`。
//!
//! # 核心类型
//!
//! 本模块转导出以下关键类型：
//!
//! - `Tool`: MCP 工具定义
//! - `CallToolRequest`: 工具调用请求
//! - `CallToolResult`: 工具调用结果
//! - `ListToolsRequest`: 工具列表请求
//! - `InitializeRequest`: 初始化请求
//! - `Message`: MCP 消息类型
//! - `Role`: 角色类型（Client/Server）
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use rucora_mcp::protocol::Tool;
//!
//! // MCP 工具定义（Tool 为 #[non_exhaustive]，需通过 rmcp 提供的构造方式创建）
//! // 实际使用中通过 list_tools() 获取远端工具定义，无需手动构造
//! ```
//!
//! # 依赖
//!
//! 本模块基于 [`rmcp`](https://crates.io/crates/rmcp) 库构建。

pub use rmcp::model::*;
