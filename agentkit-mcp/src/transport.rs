//! MCP 传输层。
//!
//! 该模块主要用于转导出 `rmcp::transport`，包含 MCP 连接所需的 transport 实现与类型。
//! 上层可以通过 `agentkit_mcp::transport::*` 使用这些能力。
//!
pub use rmcp::transport::*;
