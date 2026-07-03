//! MCP 传输层
//!
//! # 概述
//!
//! 该模块主要用于转导出 `rmcp::transport`，包含 MCP 连接所需的 transport 实现与类型。
//! 上层可以通过 `rucora_mcp::transport::*` 使用这些能力。
//!
//! # 支持的传输方式
//!
//! - `StdioTransport`: 标准输入输出传输（用于本地进程）
//! - `StreamableHttpTransport`: HTTP 流式传输
//! - `HttpClientTransport`: HTTP 客户端传输
//!
//! # 使用示例
//!
//! ## Stdio 传输
//!
//! ```rust,ignore
//! use rucora_mcp::transport::IntoTransport;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // 创建 Stdio 传输层（具体 transport 类型请参考 rmcp 文档）
//! # Ok(())
//! # }
//! ```
//!
//! ## HTTP 传输
//!
//! ```rust,ignore
//! use rucora_mcp::transport::StreamableHttpClientTransport;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // 创建 HTTP 传输层（具体 transport 类型请参考 rmcp 文档）
//! # Ok(())
//! # }
//! ```
//!
//! # 依赖
//!
//! 本模块基于 [`rmcp`](https://crates.io/crates/rmcp) 库构建。

pub use rmcp::transport::*;
