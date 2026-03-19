//! MCP 协议模型定义。
//!
//! 该模块主要用于转导出 `rmcp::model` 中的类型，便于上层以 `agentkit_mcp::protocol::*`
//! 的方式引用，而不需要直接依赖/引用 `rmcp`。
//!
pub use rmcp::model::*;
