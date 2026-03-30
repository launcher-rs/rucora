# AgentKit 架构改进方案

## 📋 文档信息

- **版本**: 1.0
- **日期**: 2026 年 3 月 31 日
- **状态**: 提案
- **作者**: AgentKit Team

---

## 🎯 一、执行摘要

本提案建议**移除 `DefaultRuntime`，将 Runtime 的执行能力内聚到增强 Agent 中**，并提供多种预定义的 Agent 类型以满足不同场景需求。

### 核心变更

| 变更项 | 当前状态 | 改进后 |
|--------|----------|--------|
| **Runtime** | 独立的 `DefaultRuntime` | 移除，能力内聚到 Agent |
| **Agent** | 单一 `DefaultAgent` | 多种类型（Simple/Chat/Tool/ReAct/Reflect 等） |
| **执行能力** | Runtime 独有 | 所有 Agent 共享 |
| **决策逻辑** | 混合在 Agent 中 | 清晰的决策策略分离 |

### 预期收益

- ✅ **API 简化**：用户只需学习 Agent，无需理解 Runtime
- ✅ **功能完整**：不损失流式/观测器/策略/并发控制等能力
- ✅ **灵活性提升**：多种 Agent 类型适配不同场景
- ✅ **代码复用**：执行逻辑统一，决策逻辑分离
- ✅ **向后兼容**：现有代码迁移成本低

---

## 🔍 二、问题分析

### 2.1 当前架构痛点

#### 痛点 1：职责边界模糊

```rust
// 当前：用户困惑何时用 Agent，何时用 Runtime
// 场景 1：简单对话
let agent = DefaultAgent::builder()...build();
agent.run("你好").await?;  // ✅ 可以

// 场景 2：工具调用
let agent = DefaultAgent::builder()...build();
agent.run("ls -la").await?;  // ✅ 也可以，但内部有完整执行循环

// 场景 3：需要流式输出
let runtime = DefaultRuntime::new(provider, tools)...;
let mut stream = runtime.run_stream(input);  // ❌ 必须用 Runtime
```

**问题**：为什么简单场景和复杂场景要用不同的对象？

#### 痛点 2：代码重复

`DefaultAgent::run()` 和 `DefaultRuntime::run_with_agent()` 有几乎相同的 tool-calling loop 实现：

```rust
// DefaultAgent::run() - 约 200 行
loop {
    let decision = self.think(&context).await;
    match decision {
        AgentDecision::Chat { request } => { /* 调用 LLM */ }
        AgentDecision::ToolCall { name, input } => { /* 执行工具 */ }
        // ...
    }
}

// DefaultRuntime::run_with_agent() - 约 200 行
loop {
    let decision = agent.think(&context).await;
    match decision {
        AgentDecision::Chat { request } => { /* 调用 LLM */ }
        AgentDecision::ToolCall { name, input } => { /* 执行工具 */ }
        // ...
    }
}
```

#### 痛点 3：Runtime 存在感过强

根据示例分析，90% 的场景只用 `DefaultAgent`：

| 示例 | 使用对象 |
|------|----------|
| `hello_world.rs` | `DefaultAgent` |
| `chat_with_tools.rs` | `DefaultAgent` |
| `conversation.rs` | `DefaultAgent` |
| `tools.rs` | `DefaultAgent` |
| `mcp.rs` | `DefaultAgent` |

**问题**：既然大部分场景只用 Agent，为什么还要维护 Runtime？

#### 痛点 4：用户心智负担重

新用户需要理解：
1. Agent 是什么？
2. Runtime 是什么？
3. 什么时候用 Agent？
4. 什么时候用 Runtime？
5. 两者如何配合？

**理想状态**：用户只需要理解"Agent 是智能体，负责思考和执行"。

---

### 2.2 为什么不能简单合并

如果只保留 `DefaultAgent` 而不提供多种类型，会失去：

| 能力 | 说明 |
|------|------|
| **简单场景优化** | 简单问答不需要工具调用循环 |
| **专用决策逻辑** | 代码生成、研究分析等有特殊决策流程 |
| **多 Agent 协作** | Supervisor/Router 等需要组合多个 Agent |
| **渐进式学习** | 从 SimpleAgent 到 ReActAgent 的平滑升级路径 |

---

## ✅ 三、改进方案

### 3.1 核心设计原则

#### 原则 1：决策与执行分离

```
┌─────────────────────────────────────────────────────────┐
│                      Agent Trait                        │
│  ┌─────────────────┐    ┌─────────────────────────┐    │
│  │   决策层        │    │   执行层（默认实现）     │    │
│  │   (think)       │    │   (run/run_stream)      │    │
│  │   负责"做什么"   │    │   负责"怎么做"          │    │
│  └─────────────────┘    └─────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

- **决策层**：每个 Agent 类型有不同的思考策略
- **执行层**：所有 Agent 共享相同的执行能力（工具调用、流式、观测器等）

#### 原则 2：能力内聚而非分离

Runtime 的执行能力（流式/观测器/策略/并发）内聚到 Agent 中：

```rust
pub struct ToolAgent<P> {
    // 决策相关
    provider: P,
    model: String,
    system_prompt: Option<String>,
    
    // 工具相关
    tools: HashMap<String, Arc<dyn Tool>>,
    max_steps: usize,
    
    // ← Runtime 能力内聚
    observer: Arc<dyn RuntimeObserver>,
    policy: Arc<dyn ToolPolicy>,
    max_tool_concurrency: usize,
}
```

#### 原则 3：提供多种预定义类型

根据不同场景提供预定义的 Agent 类型：

```
基础层（常用）:
├─ SimpleAgent   - 简单问答
├─ ChatAgent     - 纯对话
└─ ToolAgent     - 工具调用（默认）

进阶层（复杂任务）:
├─ ReActAgent    - 推理 + 行动
├─ PlanAgent     - 规划执行
└─ ReflectAgent  - 反思迭代

专家层（专业场景）:
├─ CodeAgent     - 代码生成
├─ ResearchAgent - 研究分析
└─ DataAgent     - 数据分析

协作层（多 Agent 系统）:
├─ SupervisorAgent - 主管协调
└─ RouterAgent     - 路由分发
```

---

### 3.2 架构对比

#### 当前架构

```
┌─────────────────────────────────────────────────────────┐
│                    agentkit-core                        │
│  ┌─────────────────┐    ┌─────────────────────────┐    │
│  │   Agent trait   │    │   Runtime trait         │    │
│  │   (抽象)        │    │   (抽象)                │    │
│  └─────────────────┘    └─────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
                          ↓ 实现
┌─────────────────────────────────────────────────────────┐
│                      agentkit                           │
│  ┌─────────────────┐    ┌─────────────────────────┐    │
│  │  DefaultAgent   │    │   DefaultRuntime        │    │
│  │  (决策 + 执行)   │    │   (执行)                │    │
│  │                 │    │                         │    │
│  │ - think()       │    │ - run()                 │    │
│  │ - run()         │    │ - run_stream()          │    │
│  │                 │    │ - 工具策略              │    │
│  │                 │    │ - 观测器                │    │
│  └─────────────────┘    └─────────────────────────┘    │
│                                                         │
│  问题：代码重复、职责模糊、用户困惑                     │
└─────────────────────────────────────────────────────────┘
```

#### 改进后架构

```
┌─────────────────────────────────────────────────────────┐
│                    agentkit-core                        │
│  ┌─────────────────┐    ┌─────────────────────────┐    │
│  │   Agent trait   │    │   Runtime trait         │    │
│  │   (保留，可选)  │    │   (保留，第三方用)       │    │
│  └─────────────────┘    └─────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
                          ↓ 实现
┌─────────────────────────────────────────────────────────┐
│                      agentkit                           │
│  ┌─────────────────────────────────────────────────┐   │
│  │          共享执行能力 (DefaultExecution)         │   │
│  │  - run() / run_stream()                         │   │
│  │  - 工具调用循环                                  │   │
│  │  - 并发控制                                      │   │
│  │  - 策略检查                                      │   │
│  │  - 观测器                                        │   │
│  └─────────────────────────────────────────────────┘   │
│                          ↑ 组合                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐ │
│  │ Simple   │  │  Chat    │  │  Tool    │  │ ReAct  │ │
│  │ Agent    │  │  Agent   │  │  Agent   │  │ Agent  │ │
│  │ (决策)   │  │ (决策)   │  │ (决策)   │  │ (决策) │ │
│  └──────────┘  └──────────┘  └──────────┘  └────────┘ │
│                                                         │
│  优势：职责清晰、代码复用、类型丰富                     │
└─────────────────────────────────────────────────────────┘
```

---

### 3.3 详细设计

#### 3.3.1 共享执行能力

```rust
/// 默认执行实现（内聚所有 Runtime 能力）
pub struct DefaultExecution {
    provider: Arc<dyn LlmProvider>,
    tools: HashMap<String, Arc<dyn Tool>>,
    model: String,
    system_prompt: Option<String>,
    max_steps: usize,
    
    // Runtime 能力内聚
    observer: Arc<dyn RuntimeObserver>,
    policy: Arc<dyn ToolPolicy>,
    max_tool_concurrency: usize,
    
    // 对话历史（可选）
    conversation_manager: Option<Arc<Mutex<ConversationManager>>>,
}

impl DefaultExecution {
    /// 非流式执行
    pub async fn run(&self, agent: &dyn Agent, input: AgentInput) 
        -> Result<AgentOutput, AgentError> 
    {
        self._run_loop(agent, input).await
    }
    
    /// 流式执行
    pub fn run_stream(&self, agent: &dyn Agent, input: AgentInput) 
        -> BoxStream<'static, Result<ChannelEvent, AgentError>> 
    {
        self._run_stream_loop(agent, input)
    }
    
    // 内部实现：工具调用循环
    async fn _run_loop(&self, agent: &dyn Agent, input: AgentInput) 
        -> Result<AgentOutput, AgentError> 
    {
        // 完整的 tool-calling loop 实现
        // 复用当前 DefaultRuntime 的逻辑
    }
    
    fn _run_stream_loop(&self, agent: &dyn Agent, input: AgentInput) 
        -> BoxStream<'static, Result<ChannelEvent, AgentError>> 
    {
        // 流式执行实现
        // 复用当前 DefaultRuntime::run_stream() 的逻辑
    }
}
```

#### 3.3.2 Agent Trait 增强

```rust
#[async_trait]
pub trait Agent: Send + Sync {
    /// 思考：分析当前情况，决定下一步行动
    async fn think(&self, context: &AgentContext) -> AgentDecision;
    
    /// 获取 Agent 名称
    fn name(&self) -> &str;
    
    /// 获取 Agent 描述（可选）
    fn description(&self) -> Option<&str> {
        None
    }
    
    /// 运行 Agent（默认使用 DefaultExecution）
    async fn run(&self, input: impl Into<AgentInput> + Send) 
        -> Result<AgentOutput, AgentError> 
    where
        Self: Sized,
    {
        // 默认实现：使用内嵌的执行能力
        self.execution().run(self, input.into()).await
    }
    
    /// 流式运行（默认使用 DefaultExecution）
    fn run_stream(&self, input: AgentInput) 
        -> BoxStream<'static, Result<ChannelEvent, AgentError>> 
    where
        Self: Sized,
    {
        self.execution().run_stream(self, input)
    }
    
    /// 获取执行能力（由具体实现提供）
    fn execution(&self) -> &DefaultExecution;
}
```

#### 3.3.3 具体 Agent 实现示例

```rust
/// ToolAgent - 工具调用 Agent（当前 DefaultAgent 的定位）
pub struct ToolAgent<P> {
    // 决策相关
    provider: P,
    model: String,
    system_prompt: Option<String>,
    
    // 工具相关
    tools: HashMap<String, Arc<dyn Tool>>,
    max_steps: usize,
    
    // 执行能力（内聚）
    execution: DefaultExecution,
}

#[async_trait]
impl<P> Agent for ToolAgent<P>
where
    P: LlmProvider + Send + Sync + 'static,
{
    async fn think(&self, context: &AgentContext) -> AgentDecision {
        // ToolAgent 的决策逻辑
        if !context.tool_results.is_empty() {
            // 有工具结果，让 LLM 生成最终回复
            AgentDecision::Chat {
                request: self._build_chat_request(context),
            }
        } else {
            // 默认：让 LLM 决定是否调用工具
            AgentDecision::Chat {
                request: self._build_chat_request_with_tools(context),
            }
        }
    }

    fn name(&self) -> &str { "tool_agent" }
    
    fn execution(&self) -> &DefaultExecution {
        &self.execution
    }
}

// Builder 模式
impl<P> ToolAgent<P>
where
    P: LlmProvider,
{
    pub fn builder() -> ToolAgentBuilder<P> {
        ToolAgentBuilder::new()
    }
}
```

---

### 3.4 迁移路径

#### 阶段 1：新增增强 Agent（向后兼容）

```rust
// 新增模块结构
agentkit/
├── agent/
│   ├── mod.rs              // 导出所有 Agent 类型
│   ├── execution.rs        // DefaultExecution（共享执行能力）
│   ├── simple.rs           // SimpleAgent
│   ├── chat.rs             // ChatAgent
│   ├── tool.rs             // ToolAgent
│   ├── react.rs            // ReActAgent
│   ├── reflect.rs          // ReflectAgent
│   └── builder.rs          // 通用 Builder 工具
```

#### 阶段 2：标记 Runtime 为 Deprecated

```rust
#[deprecated(
    since = "0.2.0",
    note = "DefaultRuntime 已废弃，请使用 ToolAgent 或其他 Agent 类型。功能已内聚到 Agent 中。"
)]
pub struct DefaultRuntime {
    // ...
}
```

#### 阶段 3：更新示例和文档

| 原示例 | 新示例 |
|--------|--------|
| `agent.run()` | `agent.run()`（保持不变） |
| `runtime.run_with_agent()` | `agent.run()`（简化） |
| `runtime.run_stream()` | `agent.run_stream()`（统一） |

---

## 📊 四、代码对比

### 4.1 简单对话场景

#### 当前代码

```rust
use agentkit::agent::DefaultAgent;
use agentkit::provider::OpenAiProvider;

let provider = OpenAiProvider::from_env()?;
let agent = DefaultAgent::builder()
    .provider(provider)
    .model("gpt-4o-mini")
    .system_prompt("你是有用的助手")
    .build();

let output = agent.run("你好").await?;
```

#### 改进后代码

```rust
use agentkit::agent::ChatAgent;  // 或 SimpleAgent
use agentkit::provider::OpenAiProvider;

let provider = OpenAiProvider::from_env()?;
let agent = ChatAgent::builder()
    .provider(provider)
    .model("gpt-4o-mini")
    .system_prompt("你是友好的助手")
    .build();

let output = agent.run("你好").await?;
```

**变化**：更语义化（`ChatAgent` 明确表示用于对话）。

---

### 4.2 工具调用场景

#### 当前代码

```rust
use agentkit::agent::DefaultAgent;
use agentkit::tools::ShellTool;

let agent = DefaultAgent::builder()
    .provider(provider)
    .model("gpt-4o-mini")
    .tool(ShellTool)
    .build();

let output = agent.run("ls -la").await?;
```

#### 改进后代码

```rust
use agentkit::agent::ToolAgent;
use agentkit::tools::ShellTool;

let agent = ToolAgent::builder()
    .provider(provider)
    .model("gpt-4o-mini")
    .tool(ShellTool)
    .build();

let output = agent.run("ls -la").await?;
```

**变化**：语义更清晰（`ToolAgent` 明确表示支持工具调用）。

---

### 4.3 流式输出场景

#### 当前代码

```rust
use agentkit::agent::DefaultAgent;
use agentkit::runtime::DefaultRuntime;
use futures_util::StreamExt;

let agent = DefaultAgent::builder()...build();
let runtime = DefaultRuntime::new(provider, tools)...;

let mut stream = runtime.run_stream(input);
while let Some(event) = stream.next().await {
    match event {
        ChannelEvent::TokenDelta(delta) => print!("{}", delta.delta),
        _ => {}
    }
}
```

#### 改进后代码

```rust
use agentkit::agent::ToolAgent;
use futures_util::StreamExt;

let agent = ToolAgent::builder()
    .provider(provider)
    .model("gpt-4o-mini")
    .tool(ShellTool)
    .build();

let mut stream = agent.run_stream(input);
while let Some(event) = stream.next().await {
    match event {
        ChannelEvent::TokenDelta(delta) => print!("{}", delta.delta),
        _ => {}
    }
}
```

**变化**：统一 API，不需要额外创建 Runtime。

---

### 4.4 复杂任务场景（ReAct）

#### 当前代码

```rust
// 需要手动实现 ReAct 逻辑
let agent = DefaultAgent::builder()...build();
// 用户需要自己管理思考 - 行动 - 观察循环
```

#### 改进后代码

```rust
use agentkit::agent::ReActAgent;

let agent = ReActAgent::builder()
    .provider(provider)
    .model("gpt-4o-mini")
    .tools(vec![ShellTool, HttpTool, FileReadTool])
    .max_steps(15)
    .build();

let output = agent.run("帮我分析这个项目的代码结构").await?;
// 自动执行 ReAct 循环：思考 → 行动 → 观察 → 思考 → ...
```

**变化**：提供专用 Agent，简化复杂任务。

---

## 🎯 五、Agent 类型详解

### 5.1 基础层 Agent

| Agent 类型 | 职责 | 适用场景 | 复杂度 |
|------------|------|----------|--------|
| **SimpleAgent** | 一次 LLM 调用，直接返回 | 简单问答、翻译、总结 | ⭐ |
| **ChatAgent** | 支持多轮对话历史 | 客服、心理咨询、闲聊 | ⭐⭐ |
| **ToolAgent** | 工具调用循环 | 执行具体任务（默认选择） | ⭐⭐⭐ |

### 5.2 进阶层 Agent

| Agent 类型 | 职责 | 适用场景 | 复杂度 |
|------------|------|----------|--------|
| **ReActAgent** | 显式的思考 - 行动 - 观察循环 | 多步推理任务 | ⭐⭐⭐⭐ |
| **PlanAgent** | 先规划再分步执行 | 复杂项目、工作流 | ⭐⭐⭐⭐ |
| **ReflectAgent** | 执行后自我批评改进 | 代码生成、写作 | ⭐⭐⭐⭐ |

### 5.3 专家层 Agent

| Agent 类型 | 职责 | 适用场景 | 复杂度 |
|------------|------|----------|--------|
| **CodeAgent** | 专注于代码生成、调试 | 编程任务 | ⭐⭐⭐⭐⭐ |
| **ResearchAgent** | 信息搜集、整理、分析 | 市场调研、文献综述 | ⭐⭐⭐⭐⭐ |
| **DataAgent** | 数据分析、可视化 | 数据处理任务 | ⭐⭐⭐⭐⭐ |

### 5.4 协作层 Agent

| Agent 类型 | 职责 | 适用场景 | 复杂度 |
|------------|------|----------|--------|
| **SupervisorAgent** | 管理多个专家 Agent | 复杂项目协作 | ⭐⭐⭐⭐⭐ |
| **RouterAgent** | 根据输入路由到合适 Agent | 多技能系统 | ⭐⭐⭐⭐ |

---

## 📐 六、实现计划

### 阶段 1：核心基础设施（2 天）

- [ ] 实现 `DefaultExecution`（共享执行能力）
- [ ] 增强 `Agent trait`（添加默认 run/run_stream 方法）
- [ ] 实现 `SimpleAgent`
- [ ] 实现 `ChatAgent`
- [ ] 实现 `ToolAgent`（重构当前 `DefaultAgent`）

### 阶段 2：进阶 Agent（2 天）

- [ ] 实现 `ReActAgent`
- [ ] 实现 `PlanAgent`
- [ ] 实现 `ReflectAgent`
- [ ] 编写单元测试

### 阶段 3：专家 Agent（2 天）

- [ ] 实现 `CodeAgent`
- [ ] 实现 `ResearchAgent`
- [ ] 编写示例代码

### 阶段 4：协作 Agent（1 天）

- [ ] 实现 `SupervisorAgent`
- [ ] 实现 `RouterAgent`
- [ ] 编写多 Agent 协作示例

### 阶段 5：迁移与文档（1 天）

- [ ] 标记 `DefaultRuntime` 为 deprecated
- [ ] 更新所有示例代码
- [ ] 更新文档
- [ ] 编写迁移指南

**总计**: 8 天

---

## 🔧 七、风险与缓解

### 风险 1：代码量增加

**问题**：多种 Agent 类型会增加代码量。

**缓解**：
- 共享执行能力（`DefaultExecution`）减少重复
- 使用宏生成样板代码
- 分阶段实现，优先实现常用类型

### 风险 2：用户迁移成本

**问题**：现有用户需要修改代码。

**缓解**：
- 保持 `DefaultAgent` 作为 `ToolAgent` 的别名（向后兼容）
- 提供详细的迁移指南
- `DefaultRuntime` 标记为 deprecated 而非立即移除

### 风险 3：学习曲线

**问题**：太多 Agent 类型让用户困惑。

**缓解**：
- 提供清晰的选择指南（决策树）
- 文档强调"从 SimpleAgent 开始，按需升级"
- 示例代码展示渐进式使用场景

---

## 📈 八、成功指标

### 代码质量

- [ ] 代码重复率降低 30%
- [ ] 测试覆盖率 > 80%
- [ ] Clippy 警告为 0

### 用户体验

- [ ] 示例代码行数减少 20%
- [ ] 文档清晰度评分 > 4.5/5
- [ ] 用户迁移成功率 > 90%

### 性能指标

- [ ] 运行时无性能退化
- [ ] 内存占用无显著增加
- [ ] 编译时间无显著增加

---

## 📚 九、附录

### A. Agent 类型选择决策树

```
用户输入
    │
    ▼
是否需要工具调用？
    │
    ├─ 否 ──► 是否需要多轮对话历史？
    │           │
    │           ├─ 否 ──► SimpleAgent（简单问答）
    │           │
    │           └─ 是 ──► ChatAgent（对话）
    │
    └─ 是 ──► 需要多少步？
              │
              ├─ 1-2 步 ──► ToolAgent（默认选择）
              │
              ├─ 3-5 步 ──► ReActAgent（推理 + 行动）
              │
              ├─ 5+ 步 ──► PlanAgent（规划执行）
              │
              └─ 需要高质量 ──► ReflectAgent（反思迭代）
              
复杂任务？
    │
    ├─ 代码相关 ──► CodeAgent
    ├─ 研究分析 ──► ResearchAgent
    ├─ 多角色协作 ──► SupervisorAgent
    └─ 多技能分流 ──► RouterAgent
```

### B. API 对比表

| 功能 | 当前 API | 改进后 API |
|------|----------|------------|
| 简单对话 | `DefaultAgent::run()` | `SimpleAgent::run()` |
| 多轮对话 | `DefaultAgent::run()` | `ChatAgent::run()` |
| 工具调用 | `DefaultAgent::run()` | `ToolAgent::run()` |
| 流式输出 | `DefaultRuntime::run_stream()` | `Agent::run_stream()` |
| ReAct | 手动实现 | `ReActAgent::run()` |
| 规划执行 | 手动实现 | `PlanAgent::run()` |
| 反思迭代 | 手动实现 | `ReflectAgent::run()` |

### C. 迁移示例

#### 从 DefaultAgent 迁移

```rust
// 原代码
use agentkit::agent::DefaultAgent;
let agent = DefaultAgent::builder()...build();

// 新代码（推荐）
use agentkit::agent::ToolAgent;
let agent = ToolAgent::builder()...build();

// 或保持兼容（DefaultAgent 作为别名）
use agentkit::agent::DefaultAgent; // 实际是 ToolAgent 的别名
let agent = DefaultAgent::builder()...build();
```

#### 从 DefaultRuntime 迁移

```rust
// 原代码
let runtime = DefaultRuntime::new(provider, tools)...;
let output = runtime.run_with_agent(&agent, input).await?;

// 新代码
let agent = ToolAgent::builder()
    .provider(provider)
    .tools(tools)
    ...build();
let output = agent.run(input).await?;
```

---

## 📝 十、总结

本提案建议：

1. **移除 `DefaultRuntime`**，将执行能力内聚到 Agent 中
2. **提供多种 Agent 类型**，适配不同场景
3. **保持向后兼容**，平滑迁移路径

**预期收益**：
- API 简化（用户只需学习 Agent）
- 功能完整（不损失任何 Runtime 能力）
- 灵活性提升（多种 Agent 类型可选）
- 代码复用（共享执行能力）

**下一步**：
1. 评审本提案
2. 收集社区反馈
3. 开始分阶段实施

---

*文档结束*
