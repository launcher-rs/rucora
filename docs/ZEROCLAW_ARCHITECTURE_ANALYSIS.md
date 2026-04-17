# Zeroclaw vs Agentkit 架构对比分析与改进方案

> 分析日期: 2026-04-17  
> 分析范围: `temp/zeroclaw/crates/` vs agentkit 全工作区

---

## 一、代码库概览

### Agentkit 当前 crate 结构

```
agentkit-core          # 核心抽象层：trait、类型、错误（仅接口，无实现）
agentkit               # 主实现包：Agent、Execution、Memory、Policy、Middleware
agentkit-providers     # LLM Provider 实现（OpenAI、Anthropic 等）
agentkit-tools         # 工具实现（web_fetch、browse、搜索等）
agentkit-skills        # 技能（Skill）封装
agentkit-mcp           # MCP 协议支持
agentkit-a2a           # Agent-to-Agent 协议
agentkit-embed         # 向量嵌入
agentkit-retrieval     # 向量检索
```

### Zeroclaw 当前 crate 结构

```
zeroclaw-api           # 纯接口层（trait only，无实现，无重依赖）
zeroclaw-config        # 配置模型 + SecurityPolicy（策略、告警）
zeroclaw-providers     # Provider 实现（OpenAI、Anthropic、Gemini 等）
zeroclaw-tools         # 工具实现（100+ 工具）
zeroclaw-memory        # Memory 后端实现（SQLite、Qdrant、文件等）
zeroclaw-runtime       # 运行时核心：AgentLoop、History管理、Observability
zeroclaw-channels      # 消息渠道（Telegram、Discord、Slack 等20+）
zeroclaw-infra         # 基础设施（Session 存储、Stall 看门狗）
zeroclaw-macros        # 过程宏
zeroclaw-plugins       # WASM 插件
zeroclaw-gateway       # HTTP Gateway（WebSocket、SSE、REST API）
zeroclaw-hardware      # 硬件外设（GPIO、Arduino 等）
zeroclaw-tui           # 终端 UI
zeroclaw-tool-call-parser  # 工具调用解析器（独立 crate）
robot-kit              # 机器人套件
aardvark-sys           # FFI 绑定
```

---

## 二、核心架构差异分析

### 2.1 接口抽象层（最重要的架构差异）

**Zeroclaw：`zeroclaw-api` 完全纯接口**

`zeroclaw-api/src/lib.rs` 文档注释明确声明：
> "No implementations, no heavy dependencies. Every other crate in the workspace depends on this. The compiler enforces that no implementation crate can import another without going through these interfaces."

```
zeroclaw-api
  ├── agent.rs          # TurnEvent（流式事件）
  ├── channel.rs        # Channel trait（消息渠道接口）
  ├── media.rs          # 多媒体类型
  ├── memory_traits.rs  # Memory trait（完整 CRUD + 命名空间 + GDPR 导出）
  ├── observability_traits.rs  # Observer trait（事件 + 指标 双轨）
  ├── peripherals_traits.rs    # 硬件外设 trait
  ├── provider.rs       # Provider + StreamEvent
  ├── runtime_traits.rs # RuntimeAdapter（平台能力声明）
  ├── schema.rs         # 共享数据类型
  └── tool.rs           # Tool trait（最简洁版本）
```

**Agentkit：`agentkit-core` 接口层结构相似，但有几处重要差异**

agentkit-core 包含了 `error_classifier`、`injection_guard` 等实用模块，这些不完全是纯接口，是实现了具体逻辑的模块。

**差异总结：**

| 维度 | Zeroclaw | Agentkit |
|------|---------|---------|
| 接口层纯度 | 完全纯 trait，无实现代码 | 含少量实现（error_classifier、injection_guard）|
| 依赖约束 | 编译器强制：所有 impl crate 只能通过 api 层交互 | 存在 agentkit 直接依赖 agentkit-tools 的情况 |
| Task-local 存储 | `zeroclaw-api` 定义 TOOL_LOOP_THREAD_ID（任务级数据） | agentkit 中无对应机制 |

---

### 2.2 Observability（可观测性）—— zeroclaw 领先

**Zeroclaw 的 Observer 系统**

`zeroclaw-api/src/observability_traits.rs` 定义了丰富的 `ObserverEvent` 枚举：

```rust
pub enum ObserverEvent {
    AgentStart { provider, model },
    LlmRequest { provider, model, messages_count },
    LlmResponse { provider, model, duration, success, input_tokens, output_tokens },
    AgentEnd { tokens_used, cost_usd },
    ToolCallStart { tool, arguments },
    ToolCall { tool, duration, success },
    TurnComplete,
    ChannelMessage { channel, direction },
    HeartbeatTick,
    CacheHit { cache_type, tokens_saved },
    CacheMiss { cache_type },
    Error { component, message },
    HandStarted, HandCompleted, HandFailed,      // CI/CD 相关
    DeploymentStarted, DeploymentCompleted, DeploymentFailed, RecoveryCompleted,
}
```

同时有独立的指标系统：
```rust
pub enum ObserverMetric {
    RequestLatency(Duration),
    TokensUsed(u64),
    ActiveSessions(u64),
    QueueDepth(u64),
    HandRunDuration { ... },
    ...
}
```

实现端提供了 **多种后端**：
- `LogObserver` - 结构化日志
- `VerboseObserver` - 详细调试输出
- `PrometheusObserver` - Prometheus 指标（feature-gated）
- `OtelObserver` - OpenTelemetry（feature-gated）
- `MultiObserver` - 组合多个 Observer
- `NoopObserver` - 空实现

**Agentkit 的 Observer 现状**

agentkit 目前在 `tool_execution.rs` 和 `execution.rs` 中使用基于字符串的事件：
```rust
// 字符串 event name，缺乏类型安全
observer.observe("tool_call.start", &data);
observer.observe("tool_call.cache_hit", &data);
```

没有 `ObserverMetric` 分离，没有多后端工厂。

**差距评估：中等偏大**

---

### 2.3 Memory（记忆）—— zeroclaw 领先

**Zeroclaw Memory trait 的高级特性：**

```rust
pub trait Memory: Send + Sync {
    // 基础 CRUD
    async fn store(&self, key, content, category, session_id) -> Result<()>;
    async fn recall(&self, query, limit, session_id, since, until) -> Result<Vec<MemoryEntry>>;
    
    // 高级功能
    async fn purge_namespace(&self, namespace) -> Result<usize>;  // 批量清除命名空间
    async fn purge_session(&self, session_id) -> Result<usize>;   // 批量清除会话
    async fn store_procedural(&self, messages, session_id);       // 程序记忆（从对话提取 how-to）
    async fn recall_namespaced(&self, namespace, query, ...);      // 带命名空间的召回
    async fn export(&self, filter: &ExportFilter) -> Result<Vec<MemoryEntry>>;  // GDPR 数据导出
    async fn store_with_metadata(&self, ..., namespace, importance); // 带元数据存储
}

pub struct MemoryEntry {
    pub importance: Option<f64>,       // 重要性分数
    pub superseded_by: Option<String>, // 被更新条目替代标记
    pub namespace: String,             // 命名空间隔离
}
```

还有独立的 `zeroclaw-memory` crate 提供多种后端：
- SQLite（本地持久化）
- Qdrant（向量数据库）
- Markdown（文件格式）
- 内存缓存
- 知识图谱（knowledge_graph.rs）
- 记忆衰减（decay.rs）
- 记忆整合（consolidation.rs）
- 重要性评分（importance.rs）
- 内存清理（hygiene.rs）
- 冲突解决（conflict.rs）

**Agentkit Memory 现状**

agentkit-core 定义了基础 Memory trait，agentkit 有 `FileMemory` 和 `InMemoryMemory` 两种实现，功能较简单，缺少：
- 命名空间隔离
- 重要性评分
- 记忆衰减
- 程序记忆（从对话中提取知识模式）
- GDPR 数据导出

**差距评估：较大**

---

### 2.4 History 管理（会话历史修剪）—— zeroclaw 领先

`zeroclaw-runtime/src/agent/history_pruner.rs` 实现了完整的 `prune_history` 函数，包含：

1. **Phase 1 - 折叠工具对**：将 `assistant(tool_calls) + tool(results)` 原子性折叠为摘要
2. **Phase 2 - 预算强制**：Token 超预算时，按原子组（assistant+tool）整体删除，防止孤儿消息
3. **Phase 3 - 孤儿清理**：删除无配对 assistant 的 tool 消息

`remove_orphaned_tool_messages` 还处理了：
- 会话历史加载后 assistant(tool_calls) 被截断留下的孤儿 tool 消息
- 连续 assistant 消息（工具调用+工具结果对的归一化问题）

**Agentkit 现状**

agentkit 在 `compact/` 模块有 token 压缩功能，但缺少针对 native tool calls 的孤儿消息处理。当 context 超限时的恢复策略也较简单。

**差距评估：中等**

---

### 2.5 Hook 系统（钩子）—— zeroclaw 领先

zeroclaw-runtime 有成熟的 `HookHandler` trait 系统：

```rust
pub trait HookHandler: Send + Sync {
    fn name(&self) -> &str;
    fn priority(&self) -> i32 { 0 }  // 优先级排序

    // 只读（fire-and-forget，并行执行）
    async fn on_gateway_start(&self, host, port) {}
    async fn on_session_start(&self, session_id, channel) {}
    async fn on_llm_input(&self, messages, model) {}
    async fn on_llm_output(&self, response) {}
    async fn on_after_tool_call(&self, tool, result, duration) {}

    // 修改型（按优先级顺序执行，可取消）
    async fn before_model_resolve(&self, provider, model) -> HookResult<(String, String)>;
    async fn before_prompt_build(&self, prompt) -> HookResult<String>;
    async fn before_llm_call(&self, messages, model) -> HookResult<(...)>;
    async fn before_tool_call(&self, name, args) -> HookResult<(String, Value)>;
    async fn on_message_received(&self, message) -> HookResult<ChannelMessage>;
}
```

支持两类钩子：
- **Void 钩子**：观察性，并行 fire-and-forget
- **Modifying 钩子**：按优先级顺序执行，返回 `HookResult<T>` 可修改数据或取消操作

**Agentkit 现状**

agentkit 有 `middleware.rs` 实现中间件模式，但没有完整的优先级排序和 void/modifying 两级区分。

**差距评估：中等**

---

### 2.6 RuntimeAdapter（运行时适配器）—— zeroclaw 独有

`zeroclaw-api/src/runtime_traits.rs` 定义了 `RuntimeAdapter` trait：

```rust
pub trait RuntimeAdapter: Send + Sync {
    fn name(&self) -> &str;
    fn has_shell_access(&self) -> bool;
    fn has_filesystem_access(&self) -> bool;
    fn storage_path(&self) -> PathBuf;
    fn supports_long_running(&self) -> bool;
    fn memory_budget(&self) -> u64 { 0 }
    fn build_shell_command(&self, command, workspace_dir) -> Result<Command>;
}
```

这使得相同的 agent 代码可以在 Native、Docker、WASM、Serverless 等多个运行时中工作，只需提供不同的 adapter。

**Agentkit 现状**

agentkit 没有运行时适配器概念，shell 命令执行直接硬编码在工具实现中。

**差距评估：较大（对于需要跨平台部署的场景）**

---

### 2.7 Tool Filter Groups（工具过滤分组）—— zeroclaw 独有

`zeroclaw-runtime` 中的 `filter_tool_specs_for_turn` 支持：
- `always` 模式：工具始终可用
- `dynamic` 模式：工具仅在用户消息包含特定关键词时可用

这对 MCP 工具数量较多时非常有用，可以避免 token 浪费。

**差距评估：小（可以按需添加）**

---

### 2.8 Loop Detector（循环检测）—— zeroclaw 独有

`zeroclaw-runtime/src/agent/loop_detector.rs` 提供了防止 Agent 进入无限循环的机制：
- 检测相同 tool+args+output 的重复模式
- 支持 Warning（注入 nudge）/ Block（替换输出）/ Break（终止循环）三级响应
- 时间门控（`loop_detection_min_elapsed_secs`）避免对长任务误报

**Agentkit 现状**

agentkit 无专门的循环检测，仅靠 `max_iterations` 限制。

**差距评估：小（可以按需添加）**

---

### 2.9 zeroclaw-tool-call-parser（独立解析 crate）—— zeroclaw 独有

zeroclaw 将工具调用解析器提取为独立 crate `zeroclaw-tool-call-parser`，支持：
- XML tags：`<tool_call>...</tool_call>`
- Native OpenAI format
- GLM format
- Markdown code blocks

好处：解析逻辑可单独测试、版本化，可被第三方使用。

---

### 2.10 Credential 清洗（安全）—— zeroclaw 独有

`zeroclaw-runtime` 中的 `scrub_credentials` 函数通过正则表达式清洗工具输出中的凭据，防止 API key、token 等出现在日志或 LLM 上下文中。这是一个重要的安全特性。

---

### 2.11 Context 窗口溢出恢复—— zeroclaw 更完善

Zeroclaw 在 `run_tool_call_loop` 中内置了三步恢复策略：
1. 快速修剪旧工具结果
2. 紧急删除最旧的非系统消息
3. 仍不够时才返回错误

Agentkit 的 compact 模块处理这个问题，但没有在 agent loop 内嵌内联恢复。

---

## 三、综合评估

| 特性 | Agentkit 现状 | Zeroclaw | 优先级 |
|------|------|---------|------|
| 接口层纯度 | 基本良好 | 更严格 | P2 |
| Observability 多后端 | 无（字符串事件） | 完整（Log/Verbose/Prometheus/OTel/Multi） | **P0** |
| ObserverMetric 独立 | 无 | 有 | P1 |
| Memory 命名空间 | 无 | 有 | P1 |
| Memory 重要性/衰减 | 无 | 有 | P2 |
| Memory GDPR 导出 | 无 | 有 | P2 |
| History 孤儿清理 | 无 | 有 | **P0** |
| History 原子 Prune | 无 | 有 | P1 |
| Hook 系统（优先级） | 中间件 | 完整 Hook | P1 |
| RuntimeAdapter | 无 | 有 | P2 |
| Tool Filter Groups | 无 | 有 | P2 |
| Loop Detector | 无 | 有 | P1 |
| Tool Call Parser 独立 crate | 无 | 有 | P2 |
| Credential 清洗 | 无 | 有 | **P0** |
| Context 溢出内联恢复 | compact 模块 | 内联恢复 | P1 |

---

## 四、改进方案

### P0: 立即修复（高安全性/稳定性影响）

#### P0-1: Credential 清洗

在 tool 输出返回给 LLM 前，需要清洗敏感信息。

**实现位置**: `agentkit/src/agent/tool_execution.rs`

```rust
// 在 tool_result_to_message() 前调用
fn scrub_credentials(output: &str) -> String {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?i)(token|api[_-]?key|password|secret|bearer)["']?\s*[:=]\s*(?:"([^"]{8,})"|'([^']{8,})'|([a-zA-Z0-9_\-\.]{8,}))"#).unwrap()
    });
    RE.replace_all(output, |caps: &regex::Captures| {
        let key = &caps[1];
        format!("{}: [REDACTED]", key)
    }).to_string()
}
```

#### P0-2: 孤儿 Tool 消息清理

在每次 `run()` 开始时，清理历史记录中失去配对 assistant 的 tool 消息。

**实现位置**: `agentkit/src/agent/execution.rs`

新增 `remove_orphaned_tool_messages` 函数，在调用 provider 前处理。

#### P0-3: Observability 类型安全事件

将字符串事件升级为枚举类型，参考 `zeroclaw-api` 的 `ObserverEvent`。

**实现位置**: `agentkit-core/src/channel/types.rs` 或新建 `agentkit-core/src/observability.rs`

---

### P1: 高优先级（功能完善）

#### P1-1: ObserverMetric 独立系统

在 Observer trait 中增加 `record_metric` 方法，支持数值指标采集（延迟、token 数量等）。

#### P1-2: Loop Detector

在 `ToolAgent` 的 execution 循环中增加循环检测：
- 记录 (tool_name, args_hash, output_hash) 三元组
- 连续出现 3 次相同 hash 时注入告警消息
- 超过阈值时终止循环

#### P1-3: History 原子修剪

升级 `compact/engine.rs`，在删除历史记录时保证 assistant(tool_calls) + tool(results) 的原子性，不产生孤儿消息。

#### P1-4: Context 溢出内联恢复

在 `execution.rs` 的 LLM 调用失败处理中，增加 context overflow 检测和快速恢复逻辑。

#### P1-5: Memory 命名空间

在 `Memory` trait 中增加 `namespace` 参数支持，允许不同 Agent 实例隔离记忆。

---

### P2: 中优先级（架构改进）

#### P2-1: RuntimeAdapter

提取运行时平台接口 `RuntimeAdapter` trait，使 Agent 可以在不同平台（native/docker/wasm）运行。

#### P2-2: Hook 系统升级

将现有 Middleware 升级为支持：
- 优先级排序（`priority: i32`）
- Void 钩子（并行 fire-and-forget）
- Modifying 钩子（顺序执行，可取消）

#### P2-3: Tool Filter Groups

支持配置 MCP 工具的动态可见性（always/dynamic 两种模式）。

#### P2-4: 接口层纯化

将 `agentkit-core` 中的 `error_classifier` 和 `injection_guard` 的实现逻辑分离到实现 crate，使 core 成为纯接口层。

#### P2-5: Tool Call Parser 独立 crate

将工具调用解析（XML、Native、Markdown 格式）提取为独立 `agentkit-tool-call-parser` crate。

---

## 五、实施顺序建议

```
Sprint 1（P0）:
  [x] P0-1: Credential 清洗
  [ ] P0-2: 孤儿 Tool 消息清理
  [ ] P0-3: Observability 枚举事件

Sprint 2（P1）:
  [ ] P1-2: Loop Detector
  [ ] P1-3: History 原子修剪
  [ ] P1-4: Context 溢出内联恢复

Sprint 3（P1 续）:
  [ ] P1-1: ObserverMetric 独立系统
  [ ] P1-5: Memory 命名空间

Sprint 4（P2）:
  [ ] P2-1: RuntimeAdapter
  [ ] P2-2: Hook 系统升级
  [ ] P2-3: Tool Filter Groups
  [ ] P2-4: 接口层纯化
  [ ] P2-5: Tool Call Parser 独立 crate
```

---

## 六、结论

Zeroclaw 相较于 Agentkit 最主要的优势在于：

1. **生产级可靠性**：孤儿消息清理、context 溢出恢复、循环检测确保 Agent 在边缘情况下不崩溃
2. **安全性**：Credential 清洗防止密钥泄露
3. **可观测性**：结构化事件 + 指标双轨，多后端（Prometheus/OTel）支持
4. **Memory 丰富度**：命名空间、重要性评分、记忆衰减、程序记忆、GDPR 合规

Agentkit 的优势在于更简洁的 API、更快的上手学习曲线，以及已实现的 retry/timeout/circuit breaker 工具调用增强（本次 session 早期已完成）。

建议优先实施 P0 级别改进（Credential 清洗 + 孤儿消息清理 + 类型安全 Observability），这些改变对系统稳定性和安全性影响最大，且实施风险较低。
