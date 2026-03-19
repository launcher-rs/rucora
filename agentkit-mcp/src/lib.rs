//! MCP（Model Context Protocol）支持。
//!
//! 本 crate 主要用于在 `agentkit` workspace 中统一对外暴露 MCP 相关能力。
//! 当前实现以对第三方库 `rmcp` 的封装/转导出为主，并提供与 `agentkit-core` 的
//! Tool 抽象对接的适配层（见 `tool` 模块）。
//!
//! 典型使用：
//! - 通过 transport 建立连接
//! - 使用 `McpClient` 列出工具并调用工具
//! - 将 MCP 工具包装成 `agentkit-core::tool::Tool`
//!
pub mod protocol;
pub mod tool;
pub mod transport;

/// `rmcp` 服务扩展 trait（直接转导出）。
pub use rmcp::ServiceExt;

/// MCP 协议模型类型（从 `rmcp::model` 转导出）。
pub use protocol::*;
/// MCP 工具适配（将远端 MCP 工具包装为 `agentkit-core` 的 Tool）。
pub use tool::*;
/// MCP 传输层（从 `rmcp::transport` 转导出）。
pub use transport::*;
