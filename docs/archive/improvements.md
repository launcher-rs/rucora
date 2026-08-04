# rucora 代码改进计划

> 本文档记录了对 rucora 代码库的全面审查中发现的问题，以及对应的修复计划和状态跟踪。

---

## 状态说明

| 状态 | 含义 |
|------|------|
| 🔴 未解决 | 待修复 |
| 🟡 进行中 | 修复中 |
| 🟢 已解决 | 已修复并验证 |

---

## 一、架构与设计

### 1.1 `Agent::run()` 默认实现与 `run_with()` 职责混淆

- **严重程度**: 中
- **位置**: `rucora-core/src/agent/mod.rs:575-622`
- **问题**: `run()` 默认实现在检测到 `Chat`/`ToolCall`/`MapAll`/`Reduce` 决策时直接返回 `RequiresRuntime` 错误，但 `ToolAgent`/`ReActAgent` 等又通过 `run_with()` 覆盖 `run()`。这导致 `run()` 的默认实现几乎没有实际用途，且文档与实际行为不一致。
- **修复计划**: 简化 `run()` 的默认实现，使其仅适用于纯推理 Agent（无需工具调用的场景），移除对 `RequiresRuntime` 的返回逻辑；或明确文档说明 `run()` 仅适用于 SimpleAgent 等无需工具调用的 Agent。同时清理 `run_with()` 和 `run()` 的文档示例。

### 1.2 `DefaultExecution` 过于臃肿（God Object 倾向）

- **严重程度**: 中
- **位置**: `rucora/src/agent/execution.rs:300-331`
- **问题**: `DefaultExecution` 同时承担工具调用循环、流式执行、并发控制、策略检查、观测器、中间件循环检测等功能，字段超过 15 个，违反单一职责原则。
- **修复计划**: 将 `DefaultExecution` 拆分为独立组件：
  - `ToolExecutionEngine` - 工具调用循环与执行
  - `StreamEngine` - 流式输出控制
  - `PolicyEngine` - 工具策略检查
  - `LoopDetector` - 循环检测（已部分独立）
  - `MiddlewareChain` - 中间件链（已独立）

### 1.3 `AgentDecision` 枚举的 `MapAll`/`Reduce` 与 `Chat` 语义重叠

- **严重程度**: 低
- **位置**: `rucora-core/src/agent/mod.rs:33-69`
- **问题**: `MapAll` 和 `Reduce` 本质上也是 `Chat` 的变体（需要调用 LLM），可以合并到 `Chat` 中通过 `ChatRequest` 的元数据区分，减少枚举复杂度。
- **修复计划**: 将 `MapAll` 和 `Reduce` 合并到 `Chat` 变体中，通过 `ChatRequest` 的辅助字段或新的 `ChatMode` 枚举区分模式。

### 1.4 `ToolResult` 的 `success` 字段设计缺陷

- **严重程度**: 高
- **位置**: `rucora-core/src/tool/types.rs:396`
- **问题**: `success: Option<bool>` 默认 `Some(true)`，但 `ToolError` 和 `ToolResult::failure()` 都不设置 `success: Some(false)`，导致逻辑失败和框架错误难以区分。同时 `is_success()` 使用 `unwrap_or(true)` 作为默认值，在 `success` 为 `None` 时返回 `true`，这可能掩盖错误。
- **修复计划**:
  1. 将 `success` 改为 `bool` 类型（非 `Option`），默认 `true`
  2. `ToolResult::failure()` 中设置 `success: false`
  3. 移除 `is_default_tool_success` 辅助函数
  4. 更新所有相关反序列化逻辑

### 1.5 `AgentDecision::ToolCall` 缺少 `tool_call_id`

- **严重程度**: 中
- **位置**: `rucora-core/src/agent/mod.rs:58-62`
- **问题**: `ToolCall` 决策只有 `name` 和 `input`，没有 `tool_call_id`，这与 LLM 返回的 `ToolCall` 结构不一致，导致追踪困难。
- **修复计划**: 为 `AgentDecision::ToolCall` 添加 `tool_call_id: String` 字段。

---

## 二、错误处理

### 2.1 `ToolError::Message` 无法携带结构化诊断信息

- **严重程度**: 中
- **位置**: `rucora-core/src/error.rs:370`
- **问题**: `ToolError::Message(String)` 没有 `category`、`retry_after` 等字段，与 `ProviderError` 的丰富结构化不统一。
- **修复计划**: 为 `ToolError::Message` 添加上下文字段（category, source），或统一所有错误类型到 `ErrorDiagnostic`。

### 2.2 `AgentError::ProviderError` 丢弃原始诊断上下文

- **严重程度**: 低
- **位置**: `rucora-core/src/error.rs:549-553`
- **问题**: `AgentError::ProviderError` 在 `diagnostic()` 中只修改了 `kind`，但丢失了 `source` 中的 `status_code` 等关键信息。
- **修复计划**: 保留完整诊断链，将 `AgentError::ProviderError` 的诊断信息完整传递。

### 2.3 `ChannelError` 仅有 `Message` 变体，无法分类

- **严重程度**: 中
- **位置**: `rucora-core/src/error.rs:612-616`
- **问题**: `ChannelError` 只有一个 `Message(String)` 变体，无法进行错误分类和重试决策。
- **修复计划**: 为 `ChannelError` 增加结构化变体，如 `SendError`、`StreamError`、`Timeout` 等。

### 2.4 `ErrorCategory::is_client_error()` 分类不准确

- **严重程度**: 低
- **位置**: `rucora-core/src/error.rs:80-88`
- **问题**: `Configuration` 和 `Policy` 被归为客户端错误，但它们通常不可重试且不应触发 fallback。`is_client_error()` 的命名与语义不匹配。
- **修复计划**: 将 `is_client_error()` 拆分为 `is_retryable_client_error()` 和 `is_permanent_client_error()`，或重命名方法以更准确表达语义。

### 2.5 `RetryPolicy::max_retries()` 默认返回 `u32::MAX` 不合理

- **严重程度**: 中
- **位置**: `rucora-core/src/retry.rs:103`
- **问题**: 默认实现返回 `u32::MAX`，意味着如果不重写 `max_retries()`，策略将永远重试。这与 `ExponentialBackoff` 和 `FixedDelay` 重写了该方法的做法不一致，可能导致意外行为。
- **修复计划**: 将默认 `max_retries()` 实现改为返回 `3`（合理默认值），或者将其改为必须实现的方法（从 trait 中移除默认实现）。

### 2.6 `ErrorClassifier` trait 无默认实现

- **严重程度**: 低
- **位置**: `rucora-core/src/error_classifier_trait.rs:156`
- **问题**: `ErrorClassifier` trait 的 `classify` 方法没有默认实现，但 `ProviderErrorExt` extension trait 也未在 core 层提供默认分类器。用户必须自己实现整个分类器。
- **修复计划**: 在 core 层提供基于 `ProviderError` 内建分类的默认 `ErrorClassifier` 实现，或将 `classify` 方法设为默认实现，调用 `self.diagnostic().category()` 做基础分类。

---

## 三、类型系统与 API 设计

### 3.1 `ChatMessage` 废弃字段未完全移除

- **严重程度**: 中
- **位置**: `rucora-core/src/provider/types.rs:180-193`
- **问题**: `tool_calls` 和 `tool_call_id` 字段已标记 `#[deprecated]` 但仍存在于结构体中，且 `ChatMessage::tool()` 方法也标记 `#[deprecated]`。Deprecated 字段增加序列化/反序列化复杂度和维护负担。下一个 minor 版本应直接移除。
- **修复计划**: 在下一个 breaking change 版本中直接移除 `tool_calls`、`tool_call_id` 字段和 `tool()` 方法。

### 3.2 `ChatRequest` 与 `LlmParams` 字段大量重复

- **严重程度**: 高
- **位置**: `rucora-core/src/provider/types.rs:494-544` vs `348-376`
- **问题**: `ChatRequest` 和 `LlmParams` 几乎包含相同的 LLM 参数（temperature, top_p, max_tokens, stop 等）。`LlmParams.apply_to()` 和 `ChatRequest` 字段重复定义，增加维护成本和一致性问题。
- **修复计划**: 将 `LlmParams` 的所有字段直接整合到 `ChatRequest` 中，移除 `LlmParams` 作为独立类型，改为 `ChatRequest::params()` 返回 `LlmParams` 的兼容视图。或让 `ChatRequest` 直接包含 `LlmParams` 作为嵌套类型。

### 3.3 `MessageContent` 的 `#[serde(untagged)]` 可能导致反序列化歧义

- **严重程度**: 中
- **位置**: `rucora-core/src/provider/types.rs:64`
- **问题**: `MessageContent` 使用 `untagged` serde，但在 `ToolCalls` 和 `ToolResult` 都包含 `text` 字段时，某些边缘情况下反序列化可能不安全。
- **修复计划**: 改用 `#[serde(tag = "role")]` 或为每种变体添加区分字段。

### 3.4 `ToolContext` 的 `HashMap<String, String>` 过于简单

- **严重程度**: 中
- **位置**: `rucora-core/src/tool/types.rs:505-541`
- **问题**: `ToolContext` 仅支持 `String` 键值对，无法传递结构化数据或二进制数据。部分工具需要传递更丰富的上下文（如文件内容、二进制数据等）。
- **修复计划**: 将 `ToolContext` 中的值类型改为 `serde_json::Value`，支持结构化数据传递。

### 3.5 `ToolCall` 缺少 `cost` / `latency` 跟踪字段

- **严重程度**: 低
- **位置**: `rucora-core/src/tool/types.rs:308-325`
- **问题**: `ToolCall` 没有记录调用延迟或 Token 消耗，难以进行性能分析和成本追踪。
- **修复计划**: 添加 `latency_ms: Option<u64>` 和 `token_usage: Option<Usage>` 字段到 `ToolResult`（而非 `ToolCall`，因为工具本身不消耗 LLM token）。

---

## 四、异步模式与并发

### 4.1 `_execute_tool_calls` 中并发路径的循环检测串行化

- **严重程度**: 中
- **位置**: `rucora/src/agent/execution.rs:1029-1056`
- **问题**: 并发执行的工具调用结果在收集后串行进行循环检测，失去了并发优势。
- **修复计划**: 将循环检测改为并发安全的方式（如使用 `DashMap` 或 `RwLock<LoopDetector>`），或使用 `rayon` 进行并行循环检测。

### 4.2 `run_batch` 无法充分利用多核

- **严重程度**: 低
- **位置**: `rucora-core/src/agent/mod.rs:691-706`
- **问题**: `run_batch` 使用 `buffer_unordered` 但每个 `self.run(input)` 是顺序 await 的，无法真正利用多核并行执行独立的 Agent 任务。
- **修复计划**: 对每个输入使用 `tokio::task::spawn` 创建独立任务，或将 `run_batch` 改为并行执行。

### 4.3 `stream_chat` 中闭包频繁 `clone()`

- **严重程度**: 低
- **位置**: `rucora-providers/src/openai.rs:658-803`
- **问题**: SSE 解析闭包中频繁访问 `self` 和局部变量，部分 `clone()` 可改为引用传递减少开销。
- **修复计划**: 将 `self` 的引用提前捕获到闭包环境中，减少不必要的 `clone()` 调用。

---

## 五、依赖与构建

### 5.1 `reqwest` 在多个 crate 中重复声明

- **严重程度**: 低
- **位置**: `rucora-providers/Cargo.toml:30`、`rucora-tools/Cargo.toml:22`
- **问题**: `reqwest` 在多个下游 crate 中重复声明，版本可能有差异。建议在工作区 `Cargo.toml` 的 `[workspace.dependencies]` 中统一锁定。
- **修复计划**: 在工作区 `Cargo.toml` 的 `[workspace.dependencies]` 中添加 `reqwest = "0.13"`，下游 crate 改为 `reqwest = { workspace = true }`。

### 5.2 `rucora-core/Cargo.toml` 包含不必要的直接依赖

- **严重程度**: 中
- **位置**: `rucora-core/Cargo.toml:15-25`
- **问题**: `regex`、`uuid`、`chrono` 等依赖放在 core 中，但这些 crate 主要在 tools/providers 中使用。core 应尽量轻量。
- **修复计划**: 将 `regex` 移到 `rucora-tools`（用于凭据清洗），`uuid` 和 `chrono` 评估是否真的需要在 core 中，或移到具体实现 crate。

### 5.3 缺少 `Cargo deny` / `cargo-audit` 配置

- **严重程度**: 低
- **位置**: 工作区根 `Cargo.toml`
- **问题**: 整个工作区没有 `deny.toml` 或 `cargo-audit` 配置，无法持续检测依赖安全和许可证合规问题。
- **修复计划**: 添加 `deny.toml` 配置到工作区根目录，启用 `cargo deny` 检查。

---

## 六、测试与质量

### 6.1 测试覆盖率不足

- **严重程度**: 中
- **位置**: 全局
- **问题**: `rucora-core/tests/` 仅有契约测试，缺少对 `ErrorClassifier`、`RetryPolicy`、`ToolFilter` 等核心逻辑的单元测试。`rucora-providers` 各 provider 实现也缺少单元测试。
- **修复计划**: 为每个核心模块补充单元测试，特别是 `ErrorClassifier`、`RetryPolicy`、`ToolFilter`、`ErrorDiagnostic` 等。
- **完成情况**: ✅ 核心模块（ErrorClassifier/DefaultErrorClassifier、RetryPolicy/ExponentialBackoff/FixedDelay、ToolFilter、ErrorDiagnostic、LoopDetector、ToolResult builder、MessageContent 兼容）均已有单元测试。本次为 `rucora-providers` 补齐缺失的 provider 单测：
  - `openai.rs`：+15 个测试（headers、response_format、tools、messages、tool_calls 解析、finish_reason、HTTP 错误映射、超时设置、模型解析）
  - `ollama.rs`：+9 个测试（provider 创建、headers、role 映射、messages/工具调用/工具结果、HTTP 错误映射、超时设置）
  - `resilient.rs`：+15 个测试（错误分类、状态码分类、backoff 上限与增长、should_retry 决策、RetryConfig 默认值）
  - **附带修复**：`resilient.rs` 的 `backoff_delay_ms()` 原实现存在溢出 bug（attempt ≥ 3 时 `attempt as u64 * 6364136223846793005` 溢出 u64），且抖动公式 `(jitter * x) % jitter` 恒为 0 导致抖动实际不生效；已改用 `wrapping_mul` 并在封顶后再计算抖动。

### 6.2 文档示例未实际运行验证

- **严重程度**: 中
- **位置**: 全局
- **问题**: 多个 trait 的文档中包含大量 `rust,ignore` 和 `rust,no_run` 的 doctest 示例，这些示例从未实际运行验证。错误信息过时后难以发现。
- **修复计划**: 将可运行示例提取到 `tests/` 目录中的集成测试文件，使用 `#[tokio::test]` 标记并实际运行验证。
- **完成情况**: ✅
  - 新增 `rucora-core/tests/doc_examples.rs`（10 个测试）：将 `Agent::run()` 的 EchoAgent 纯推理示例、`run_with_timeout`、`run_batch`（并发）、`run_stream` 默认实现、`AgentDecision` 构造器（chat/map_all/reduce）、`AgentOutput` 文本访问器、max_steps 上限等文档示例转换为实际运行的集成测试。
  - 新增 `rucora/tests/agent_doc_examples.rs`（4 个测试）：将 `SimpleAgent` 构建器 + 运行文档示例（非流式/流式/流式文本拼接/单次调用）改用 `MockProvider` 实际运行验证。
  - 修复 `Agent::run()` 文档中的 EchoAgent 示例：原示例返回 `{"echo": ...}` 但断言 `output.text()` 能取到文本（`text()` 需要 `content` 字段），且示例仅编译未实际运行；已改为返回 `{"content": ...}` 并用 `#[tokio::main(flavor = "current_thread")]` 使 doctest 真正执行断言。

### 6.3 `AgentStream` 未实现 `Stream` trait 的 `size_hint`

- **严重程度**: 低
- **位置**: `rucor/src/agent/mod.rs:371-377`
- **问题**: `AgentStream` 实现了 `Stream` trait 但未重写 `size_hint()`，可能导致某些消费者分配不当。
- **修复计划**: 为 `AgentStream` 实现 `size_hint()` 方法。

---

## 七、文档与代码约定

### 7.1 中文注释不完整

- **严重程度**: 低
- **位置**: 全局
- **问题**: AGENTS.md 要求"所有代码注释必须使用中文"，但部分关键类型（如 `ToolResult`、`VectorRecord`）的字段级文档是英文的。
- **修复计划**: 补充中文注释，将重要类型的字段文档统一为中文。

### 7.2 `#[allow(deprecated)]` 使用过于宽泛

- **严重程度**: 中
- **位置**: `rucora-core/src/provider/types.rs:197-268`
- **问题**: `ChatMessage` 的构造器方法整体标记 `#[allow(deprecated)]`，这会压制所有弃用警告（包括未来可能出现的新的弃用）。建议只对具体使用 deprecated 字段/方法的行使用 `#[allow(deprecated)]`。
- **修复计划**: 将 `#[allow(deprecated)]` 从方法级别移到具体的废弃字段引用上。

### 7.3 `AgentDecision` 文档示例中的 `no_run` 标签

- **严重程度**: 低
- **位置**: `rucora-core/src/agent/mod.rs:483-498`
- **问题**: `Agent` trait 文档中的示例使用 `rust,no_run` 标签但未提供实际的 `MockProvider` 或测试基础设施。
- **修复计划**: 将文档示例改为 `rust,ignore` 或提供完整的可运行示例。

---

## 修复进度总览

| 编号 | 问题 | 状态 |
|------|------|------|
| 1.1 | Agent::run() 与 run_with() 职责混淆 | 🟢 已解决 |
| 1.2 | DefaultExecution 过于臃肿 | 🟢 已解决 |
| 1.3 | MapAll/Reduce 与 Chat 语义重叠 | 🟢 已解决 |
| 1.4 | ToolResult.success 字段设计缺陷 | 🟢 已解决 |
| 1.5 | AgentDecision::ToolCall 缺少 tool_call_id | 🟢 已解决 |
| 2.1 | ToolError::Message 无结构化诊断 | 🟢 已解决 |
| 2.2 | AgentError::ProviderError 丢弃诊断上下文 | 🟢 已解决 |
| 2.3 | ChannelError 仅有 Message 变体 | 🟢 已解决 |
| 2.4 | ErrorCategory::is_client_error() 分类不准确 | 🟢 已解决 |
| 2.5 | RetryPolicy::max_retries() 默认 u32::MAX | 🟢 已解决 |
| 2.6 | ErrorClassifier trait 无默认实现 | 🟢 已解决 |
| 3.1 | ChatMessage 废弃字段未移除 | 🟢 已解决 |
| 3.2 | ChatRequest 与 LlmParams 字段重复 | 🟢 已解决 |
| 3.3 | MessageContent untagged 反序列化歧义 | 🟢 已解决 |
| 3.4 | ToolContext 仅支持 String 键值对 | 🟢 已解决 |
| 3.5 | ToolCall 缺少延迟跟踪字段 | 🟢 已解决 |
| 4.1 | 并发路径循环检测串行化 | 🟢 已解决 |
| 4.2 | run_batch 无法充分利用多核 | 🟢 已解决 |
| 4.3 | stream_chat 闭包频繁 clone() | 🟢 已解决 |
| 5.1 | reqwest 多 crate 重复声明 | 🟢 已解决 |
| 5.2 | rucora-core 包含不必要依赖 | 🟢 已解决 |
| 5.3 | 缺少 Cargo deny / cargo-audit 配置 | 🟢 已解决 |
| 6.1 | 测试覆盖率不足 | 🟢 已解决 |
| 6.2 | 文档示例未实际运行验证 | 🟢 已解决 |
| 6.3 | AgentStream 未实现 size_hint | 🟢 已解决 |
| 7.1 | 中文注释不完整 | 🟢 已解决 |
| 7.2 | #[allow(deprecated)] 使用过于宽泛 | 🟢 已解决 |
| 7.3 | AgentDecision 文档示例 no_run 标签 | 🟢 已解决 |