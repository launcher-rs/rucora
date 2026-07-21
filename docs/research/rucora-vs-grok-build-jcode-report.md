# rucora 对比 grok-build / jcode 研究报告

> 研究时间：2026-07-21
> 项目版本：rucora v0.4.0 | grok-build (xAI) | jcode v0.54.4

---

## 一、项目定位对比

| 维度 | rucora | grok-build (xAI) | jcode |
|------|--------|-------------------|-------|
| **定位** | 通用 Rust AI Agent 框架 | xAI 的终端编码 Agent | 全能编码 Agent 工具 |
| **规模** | ~11 个 crate | ~80 个 crate | ~76 个 crate |
| **Provider** | 8 个 Provider | 1 个（xAI） | 15+ Provider |
| **Swarm** | 无 | 子 Agent 工具 | 多 Agent 协调 |
| **Memory** | Memory trait 接口 | 无 | 图数据库 + Embedding |
| **MCP** | rucora-mcp crate | 有 | 有 |
| **协议** | 无 | ACP | 自有协议 |

---

## 二、关键架构差异

### 2.1 工具系统

| 特性 | grok-build | jcode | rucora |
|------|-----------|-------|--------|
| 流式结果 | ToolStream<T> [Progress*, Terminal] | - | 无（仅 Result） |
| 工具上下文 | ToolCallContext + TypedExtensions | ToolContext | 无 |
| 类型安全 | Tool<Args, Output> 泛型 | serde_json::Value | serde_json::Value |
| 类型擦除 | 自动 impl ToolDyn for T | Arc<dyn Tool> | Arc<dyn Tool> |
| ToolId 命名空间 | namespace:name | name | name |
| intent 自动注入 | - | 所有工具 Schema 注入 | 无 |

### 2.2 Provider 系统

| 特性 | jcode | rucora |
|------|-------|--------|
| 结构化路由 | RouteSelection + RuntimeKey | 字符串 "openai/gpt-4o" |
| Failover | FailoverDecision 分类 | 无 |
| 双重认证 | OAuth + API Key | 仅 API Key |
| Transport Retry | fresh_transport_client() | 无 |
| Provider Runtime | 分离为实现 + 注册 crate | 集中式 |

### 2.3 Agent 系统

| 特性 | grok-build | jcode | rucora |
|------|-----------|-------|--------|
| 中断控制 | CancellationToken | InterruptSignal (epoch) | GracefulShutdown trait |
| 提示模板 | MiniJinja 条件模板 | 动态上下文注入 | 固定字符串 |
| Context 管理 | CompactionPolicy | Compaction + 窗口 | 手动 |
| 子 Agent | task/kill_task 工具 | Swarm 系统 | 无 |

### 2.4 错误处理

| 特性 | grok-build | jcode | rucora |
|------|-----------|-------|--------|
| 错误分类 | ToolErrorKind 枚举 | FailoverDecision | 简单枚举 |
| 因果链 | source: anyhow::Error | - | 无 |
| 结构化元数据 | details: Option<Value> | - | 无 |
| HTTP 状态码匹配 | - | 智能（防误匹配） | 直接匹配 |

### 2.5 Memory 系统

jcode 最完善：图数据库 + Embedding + 类别半衰期 + 每轮 4 步流程。
rucora 仅有 Memory trait 接口，无实现。

---

## 三、v0.4.0 集成清单

所有以下改进均为 v0.4.0 目标，按实施顺序排列：

### [已实现] Provider 超时配置

- with_request_timeout() / with_connect_timeout() / with_client()
- 所有 8 个 Provider 均已添加
- elapsed 0ns bug 已修复（map_reqwest_error 改用实际耗时）

### [进行中] 结构化模型路由 RuntimeKey + RouteSelection

- RuntimeKey 枚举（OpenAi/Anthropic/Gemini/DeepSeek/Moonshot/OpenRouter/Ollama/AzureOpenAi/Custom）
- RouteSelection 结构体（model + runtime_key + base_url）
- Agent builder 支持 .route() 替代 .provider() + .model()
- 向后兼容：保留 .provider() 和 .model() 作为便捷方法

### [待实施] Tool 系统升级

- ToolContext { session_id, cancellation, working_dir }
- ToolStream 流式输出
- ToolResult 增加 images/metadata/title
- ToolDescription 增加 namespace/kind
- 所有内置工具适配

### [待实施] Provider Failover

- FailoverDecision 枚举
- FailoverProvider 包装器
- Agent 执行循环集成 failover

### [待实施] InterruptSignal

- AtomicBool + Notify + epoch counter
- 替代 GracefulShutdown trait
- 同步/异步双模式查询

### [待实施] 断路器和重试策略升级

- 滑动窗口断路器
- HalfOpen 探针
- Observer trait

### [待实施] 条件提示模板

- MiniJinja 或等效模板引擎
- SummaryAgent / ToolAgent 支持

### [待实施] Memory 图实现

- 图数据库存储
- 类别半衰期
- Embedding 语义搜索

### [待实施] Swarm / 子 Agent

- TaskAgent
- 后台执行 + 结果轮询
- Agent 间通信

### [待实施] 类型安全 Identifier

- SessionId / ToolId / ProviderId newtype
- opaque_id! 宏

### [待实施] Serde 宽松反序列化

- Provider 响应解析
- 单字段错误不破坏整体

---

## 四、架构变更概要

```
v0.3.0                          v0.4.0
──────                          ──────
rucora-core/                     rucora-core/
  tool/                            tool/
    trait.rs   Tool trait           trait.rs   Tool trait + ToolContext
    types.rs   ToolResult          context.rs ToolContext
                                   stream.rs  ToolStream
                                   types.rs   ToolResult (扩展 images/metadata)
  provider/                        provider/
    trait.rs   LlmProvider          trait.rs   LlmProvider (扩展)
    types.rs   ChatRequest          route.rs   RuntimeKey + RouteSelection
                                   failover.rs FailoverDecision
  error.rs     ProviderError        error.rs   ProviderError (扩展)
  agent/                           agent/
    mod.rs     Agent trait          mod.rs     Agent trait (扩展 InterruptSignal)
                                   interrupt.rs InterruptSignal

rucora-providers/                  rucora-providers/
  openai.rs                         openai.rs
  anthropic.rs                      anthropic.rs
  ...                               ...

rucora/                            rucora/
  agent/                            agent/
    summary.rs                       summary.rs (条件提示模板)
    tool.rs                          tool.rs (Failover 集成)
```

---

## 五、关键设计（代码草案）

### 5.1 RuntimeKey + RouteSelection

```rust
pub enum RuntimeKey {
    OpenAi, Anthropic, Gemini, DeepSeek,
    Moonshot, OpenRouter, Ollama, AzureOpenAi,
    Custom(String),
}

pub struct RouteSelection {
    pub model: String,
    pub runtime_key: RuntimeKey,
    pub base_url: Option<String>,
}
```

### 5.2 ToolContext

```rust
pub struct ToolContext {
    pub session_id: String,
    pub cancellation: CancellationToken,
    pub working_dir: Option<PathBuf>,
}
```

### 5.3 ToolStream

```rust
pub enum ToolStreamItem {
    Text(String),
    Progress { current: u64, total: u64, message: String },
    Terminal(Result<ToolOutput, ToolError>),
}

pub type ToolStream = Pin<Box<dyn Stream<Item = ToolStreamItem> + Send>>;
```

### 5.4 FailoverDecision

```rust
pub enum FailoverDecision {
    None,
    Retry,
    RetryNextProvider,
    RetryAndMarkUnavailable,
}
```

### 5.5 InterruptSignal

```rust
pub struct InterruptSignal {
    flag: AtomicBool,
    epoch: AtomicU64,
    notify: Notify,
}
```

---

*本报告基于 rucora v0.4.0、grok-build (xAI)、jcode v0.54.4 源码分析*
