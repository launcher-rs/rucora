# 核心抽象 (agentkit-core)

`agentkit-core` 是 AgentKit 的核心抽象层，仅定义 trait、类型和错误模型，不包含任何具体实现。第三方可以通过实现这些 trait 来扩展 AgentKit 的能力。

## 模块结构

```
agentkit-core/src/
├── agent/          # Agent 核心抽象
├── channel/        # 事件通道系统
├── embed/          # 向量嵌入抽象
├── memory/         # 记忆系统抽象
├── provider/       # LLM Provider 抽象
├── retrieval/      # 语义检索抽象
├── skill/          # 技能抽象
├── tool/           # 工具抽象
├── error.rs        # 统一错误类型
├── error_classifier_trait.rs  # 错误分类器
└── injection_guard_trait.rs   # Prompt 注入防护
```

## Agent 模块 (`agent/`)

定义 Agent 的核心 trait 和类型，是所有 Agent 实现的接口契约。

### 核心类型

| 类型 | 说明 |
|------|------|
| `AgentInput` | Agent 输入，包含文本消息和上下文 |
| `AgentOutput` | Agent 输出，包含回复文本、工具调用记录、usage |
| `AgentDecision` | Agent 决策枚举：`Chat`、`ToolCall`、`Return`、`ThinkAgain`、`Stop` |
| `AgentContext` | Agent 执行上下文，包含输入、消息历史、步骤计数 |
| `AgentExecutor` | Agent 执行器 trait |

### 核心 Trait

```rust
#[async_trait]
pub trait Agent: Send + Sync {
    async fn think(&self, context: &AgentContext) -> AgentDecision;
    fn name(&self) -> &str;
    async fn run(&self, input: AgentInput) -> Result<AgentOutput, AgentError>;
}
```

## Provider 模块 (`provider/`)

定义 LLM Provider 的统一接口，所有 Provider 实现都遵循此契约。

### 核心类型

| 类型 | 说明 |
|------|------|
| `ChatRequest` | 聊天请求，支持 messages、tools、temperature、top_p、top_k、max_tokens 等 |
| `ChatResponse` | 聊天响应，包含 assistant 消息和 usage 统计 |
| `ChatMessage` | 聊天消息，角色：System / User / Assistant / Tool |
| `ChatStreamChunk` | 流式响应分块 |
| `LlmParams` | LLM 请求参数统一配置（temperature、top_p、top_k 等） |
| `ToolDefinition` | 工具定义，用于传递给 LLM |
| `ToolCall` | 工具调用请求 |
| `ToolResult` | 工具执行结果 |

### 核心 Trait

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError>;
    async fn chat_stream(&self, request: ChatRequest) -> Result<BoxStream<'static, Result<ChatStreamChunk, ProviderError>>, ProviderError>;
}
```

## Tool 模块 (`tool/`)

定义工具的抽象接口和注册机制。

### 核心类型

| 类型 | 说明 |
|------|------|
| `ToolDefinition` | 工具定义（name、description、input_schema） |
| `ToolCall` | 工具调用（name、input） |
| `ToolResult` | 工具结果（tool_call_id、output） |
| `ToolCategory` | 工具分类枚举 |
| `ToolRegistry` | 工具注册表 |
| `ToolFilter` | 工具过滤器（动态可见性控制） |
| `ToolFilterRule` | 过滤规则 |

### 核心 Trait

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> Value;
    async fn call(&self, input: Value) -> Result<ToolResult, ToolError>;
}
```

## Skill 模块 (`skill/`)

定义技能抽象，比 Tool 更高级、可复用。

### 核心类型

| 类型 | 说明 |
|------|------|
| `SkillDefinition` | 技能定义 |
| `SkillInput` | 技能输入 |
| `SkillResult` | 技能结果 |
| `SkillContext` | 技能执行上下文 |

### 核心 Trait

```rust
#[async_trait]
pub trait Skill: Send + Sync {
    fn name(&self) -> &str;
    fn definition(&self) -> SkillDefinition;
    async fn execute(&self, input: SkillInput, ctx: &SkillContext) -> Result<SkillResult, SkillError>;
}
```

## Memory 模块 (`memory/`)

定义记忆系统抽象，支持高级记忆操作。

### 核心 Trait

```rust
#[async_trait]
pub trait Memory: Send + Sync {
    async fn store(&self, key: &str, value: Value) -> Result<(), MemoryError>;
    async fn retrieve(&self, key: &str) -> Result<Option<Value>, MemoryError>;
    async fn delete(&self, key: &str) -> Result<(), MemoryError>;
}
```

### 高级记忆 (`advanced_trait.rs`)

| 类型 | 说明 |
|------|------|
| `MemoryEntry` | 记忆条目（带命名空间和重要性评分） |
| `MemoryNamespace` | 命名空间（Session/User/Agent/Team/Org/Global） |
| `MemoryImportance` | 重要性评分（1-10 级） |
| `AdvancedMemory` | 高级记忆 trait（命名空间存储、GDPR 合规、程序记忆） |

## Retrieval 模块 (`retrieval/`)

定义向量存储和语义检索接口。

### 核心类型

| 类型 | 说明 |
|------|------|
| `VectorRecord` | 向量记录（id、embedding、metadata） |
| `VectorQuery` | 向量查询 |
| `SearchResult` | 搜索结果 |

### 核心 Trait

```rust
#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn upsert(&self, records: Vec<VectorRecord>) -> Result<Vec<String>, RetrievalError>;
    async fn search(&self, query: VectorQuery) -> Result<Vec<SearchResult>, RetrievalError>;
    async fn delete(&self, ids: Vec<String>) -> Result<(), RetrievalError>;
}
```

## Embedding 模块 (`embed/`)

定义向量嵌入 Provider 接口。

### 核心 Trait

```rust
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;
    async fn embed_batch(&self, texts: Vec<&str>) -> Result<Vec<Vec<f32>>, EmbeddingError>;
}
```

## Channel 模块 (`channel/`)

统一事件模型，用于 Runtime、Tools、Skills 之间的通信。

### 核心事件类型

| 类型 | 说明 |
|------|------|
| `ChannelEvent` | 统一事件枚举 |
| `TokenDeltaEvent` | Token 增量事件（流式输出） |
| `DebugEvent` | 调试事件 |
| `ErrorEvent` | 错误事件 |
| `ChannelSender` | 事件发送端 |
| `ChannelReceiver` | 事件接收端 |

### 钩子系统

| 类型 | 说明 |
|------|------|
| `VoidHook` | 无返回值钩子（日志、监控） |
| `ModifyingHook` | 可修改数据钩子（转换、验证） |
| `HookPriority` | 钩子优先级枚举 |
| `HookRegistry` | 钩子注册表 |

## 错误模块 (`error.rs`)

统一错误类型体系。

### 错误枚举

| 类型 | 说明 |
|------|------|
| `AgentError` | Agent 执行错误 |
| `ProviderError` | LLM Provider 错误 |
| `ToolError` | 工具执行错误 |
| `SkillError` | 技能执行错误 |
| `MemoryError` | 记忆系统错误 |
| `ChannelError` | 通道错误 |

### 错误分类器 (`error_classifier_trait.rs`)

| 类型 | 说明 |
|------|------|
| `ErrorClassifier` | 结构化错误分类器 |
| `ClassifiedError` | 分类后的错误（含恢复策略） |
| `FailoverReason` | 失败原因分类（14 种精细原因） |
| `ErrorContext` | 错误上下文 |

### 注入防护 (`injection_guard_trait.rs`)

| 类型 | 说明 |
|------|------|
| `InjectionGuard` | Prompt 注入防护扫描器 |
| `ScanResult` | 扫描结果 |
| `ThreatType` | 威胁类型（8 种） |
| `ContentScannable` | 可扫描内容 trait |

## 使用方式

用户通常不需要直接依赖 `agentkit-core`，而是通过 `agentkit` 主 crate 使用：

```rust
use agentkit::prelude::*;  // 常用类型快捷导入
use agentkit::core::*;     // 核心抽象
```
