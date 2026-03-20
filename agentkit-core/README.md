# agentkit-core（核心抽象）

`agentkit-core` 定义 Agentkit 的**最小稳定内核**：trait、核心类型、错误类型、事件模型。

## 设计原则

- **只放抽象，不放实现**：避免引入具体 HTTP 客户端、外部服务依赖。
- **可扩展**：第三方可以只依赖 core 实现自己的 Provider/Tool/VectorStore。
- **兼容性护栏**：提供 contract tests，约束关键 trait 的行为预期。

## 主要模块

- `provider`：`LlmProvider`、Chat 类型
- `tool`：`Tool`、ToolCall/ToolResult/ToolDefinition
- `retrieval`：`VectorStore`、向量查询/搜索结果类型
- `channel`：`ChannelEvent`（统一事件模型）
- `runtime`：`Runtime`（唯一执行入口）+ `RuntimeObserver`（统一观测协议）
- `error`：统一错误类型 + `DiagnosticError`

说明：core 不再定义 `Agent` trait。
`AgentInput/AgentOutput` 仍保留在 `agent::types` 中，作为 runtime 的稳定输入/输出类型。

## 测试

- `tests/contract_tool.rs`
- `tests/contract_provider.rs`
- `tests/contract_vector_store.rs`

这些测试用于约束第三方实现行为，避免 runtime 依赖的隐含假设被破坏。
