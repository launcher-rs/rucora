# agentkit-runtime（默认运行时）

`agentkit-runtime` 提供默认的 Agent 运行时实现：tool-calling loop、流式事件输出、policy/audit、trace。

## 你会在这里得到什么

- `ToolRegistry`：按名称管理 `Tool`，并生成 `ToolDefinition` 给 provider 注册
- `ToolCallingAgent`：非流式 tool loop
- `StreamingToolCallingAgent`：流式 tool loop，输出 `ChannelEvent`
- `DefaultToolPolicy`：默认安全策略（命令/域名等）
- `AuditSink`：审计接口（默认 `NoopAuditSink`）
- `trace`：JSONL 轨迹持久化与回放

## Trace（轨迹）

- `write_trace_jsonl`：把事件序列写入 JSONL
- `read_trace_jsonl`：从 JSONL 读取
- `replay_events`：回放事件流（便于测试/调试）

## Bench

- `benches/tool_policy_check.rs`：`DefaultToolPolicy::check` 热路径
