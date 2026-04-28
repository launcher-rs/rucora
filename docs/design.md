# rucora 设计思想（中文）

本文档描述 rucora 的总体设计目标、模块划分与关键数据流，便于你在阅读/扩展代码时快速建立心智模型。

## 设计目标

- **接口与实现解耦**：`rucora-core` 只定义 trait 与核心类型，第三方可以只依赖 core 自己实现 Provider/Tool/VectorStore 等。
- **runtime 可替换**：`rucora-runtime` 提供默认的 tool-calling loop 与 streaming loop，但不把“如何运行”写死在 core 里。
- **可组合**：Provider/Tools/Skills/Policy/Audit/Trace 都以独立组件存在，上层自由组合。
- **可观测**：统一事件模型 `ChannelEvent`，配套 trace JSONL 持久化与回放。

## 模块与 crate 关系

- **`rucora-core`**
  - 负责：trait、数据结构、错误类型、事件类型。
  - 不负责：HTTP 请求、具体 provider、具体工具实现、默认 runtime。
- **`rucora`**
  - 负责：常用实现（providers/tools/skills/retrieval/memory/embed/config）。
  - 作为“开箱即用”的聚合 crate。
- **`rucora-runtime`**
  - 负责：默认 Runtime 实现（DefaultRuntime）、工具注册表、policy/audit、trace。
- **`rucora-cli`（可选）**
  - 负责：命令行快速试用（加载 config + skills，运行一次 agent）。
- **`rucora-server`（可选）**
  - 负责：把 agent 暴露为 HTTP 服务（SSE streaming 输出事件）。
- **`rucora-mcp`（可选）**
  - 负责：MCP 工具/资源适配，把外部 MCP tools 映射为 `Tool`。
- **`rucora-a2a`（可选）**
  - 负责：A2A 集成（用于多 agent/跨服务协作的扩展）。

## 核心抽象（core）

### Provider（`LlmProvider`）

- 输入：`ChatRequest`（messages/tools/metadata 等）
- 输出：
  - 非流式：`ChatResponse`
  - 流式：`Stream<Item = Result<ChatStreamChunk, ProviderError>>`

约定：`stream_chat` 默认实现会返回“不支持”的错误；实现方可覆盖。

### Tool（`Tool`）

- 以 JSON `Value` 作为输入/输出载体，便于与 LLM tool-calling 结构对齐。
- 通过 `ToolDefinition { name, description, input_schema }` 对外暴露 schema。

### Retrieval（`VectorStore`）

- `upsert/get/delete/search/clear/count` 等操作。
- `search` 返回 `SearchResult`，由实现决定打分/排序，但 contract tests 约束了基本行为（如 top_k、生效、排序预期）。

## 默认运行时数据流（runtime）

### DefaultRuntime（非流式）

1. 组装 `tool_defs = tools.definitions()`
2. `provider.chat()` 得到 assistant message 与 `tool_calls`
3. 若有 `tool_calls`：
   - 先 `policy.check` 决定是否允许
   - 执行 tool
   - 生成 `ToolResult` 并回灌到 messages
4. 循环直到无 tool_calls 或达到 `max_steps`

### DefaultRuntime::run_stream（流式）

- `provider.stream_chat()` 持续产出 chunk
- 每个 token delta 转成 `ChannelEvent::TokenDelta`
- step 结束后产出完整 `ChannelEvent::Message`
- 产生工具调用时：发出 `ToolCall/ToolResult` 事件并回灌 messages

## 统一事件模型与 Trace

- `ChannelEvent`：Message / TokenDelta / ToolCall / ToolResult / Skill / Memory / Debug / Error / Raw
- `rucora-runtime::trace`：
  - `write_trace_jsonl(path, &[ChannelEvent])`
  - `read_trace_jsonl(path)`
  - `replay_events(events)`

JSONL 的好处：易于流式写入、增量分析、对比回放。

## 配置（rucora::config）

- 支持：配置文件（YAML/TOML）+ profile + 环境变量覆盖。
- `rucoraConfig::load()`：读取 `rucora_CONFIG` + `rucora_PROFILE`，并应用 `rucora_*` 覆盖。
- `rucoraConfig::build_provider(&ProfileConfig)`：构建 `RouterProvider`（openai/ollama/router）。

## 代码导航建议

- 先看：
  - `rucora-core/src/*`：trait/类型/错误/事件
  - `rucora-runtime/src/lib.rs`：默认 agent loop
  - `rucora/src/skills/mod.rs`：skills loader + skill->tool 适配
  - `rucora/src/config.rs`：配置加载
- 再看 examples：`rucora/examples/*`
