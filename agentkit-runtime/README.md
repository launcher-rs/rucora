# agentkit-runtime（默认运行时）

`agentkit-runtime` 提供默认的运行时实现：tool-calling loop、流式事件输出、policy、trace。

## 你会在这里得到什么

- `ToolRegistry`：按名称管理 `Tool`，并生成 `ToolDefinition` 给 provider 注册
- `DefaultRuntime`：默认运行时（实现 `agentkit_core::runtime::Runtime`）
  - `run()`：非流式执行
  - `run_stream()`：流式执行，输出 `ChannelEvent`
- `DefaultToolPolicy`：默认安全策略（命令/域名等）
- 统一观测协议：通过 `agentkit_core::runtime::RuntimeObserver` 接收 `ChannelEvent`
- `trace`：JSONL 轨迹持久化与回放

## Trace（轨迹）

- `write_trace_jsonl`：把事件序列写入 JSONL
- `read_trace_jsonl`：从 JSONL 读取
- `replay_events`：回放事件流（便于测试/调试）

## Bench

- `benches/tool_policy_check.rs`：`DefaultToolPolicy::check` 热路径
