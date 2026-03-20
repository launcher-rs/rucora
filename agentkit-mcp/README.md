# agentkit-mcp（MCP 集成，可选）

`agentkit-mcp` 用于对接 MCP（Model Context Protocol）生态，把外部 MCP tools/resources 适配到 Agentkit。

## 目标

- 将 MCP tools 映射为 `agentkit_core::tool::Tool`
- 支持至少一种传输（stdio 或 http）
- 能与 runtime policy/audit 机制衔接（对外部资源访问做约束）

## 使用方式

参考 `agentkit/examples/mcp_demo.rs`。
