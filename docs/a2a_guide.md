# A2A (Agent-to-Agent) 协议指南

AgentKit 提供了 A2A (Agent-to-Agent) 协议集成，允许 Agent 之间通过网络进行通信和协作。

## 概述

A2A 是一个用于自治 AI Agent 之间通信的协议。在 AgentKit 中，它是 [`ra2a`](https://crates.io/crates/ra2a) crate（版本 0.9）的轻量包装器，这是 [Google A2A 协议规范](https://github.com/google/A2A) 的 Rust 实现。

**用途：**
- Agent 间通信和协作
- Agent 间的任务委托和结果返回
- 多 Agent 系统编排
- 将 Agent 能力暴露为可远程调用的服务
- 将其他 Agent 的能力作为本地工具消费

**核心设计原则：** 远程 A2A Agent 被适配为本地 `Tool`，因此任何 AgentKit Agent（ToolAgent、ReActAgent 等）都可以像调用本地工具（如 `ShellTool` 或 `FileReadTool`）一样调用远程 Agent。

## 架构

```
用户问题 --> 本地 Agent (LLM) --> 决定调用 A2A 工具
                                          |
                                          v
                                A2AToolAdapter.call()
                                          |
                                          v (HTTP, A2A 协议)
                                远程 A2A 服务器
                                (AgentExecutor 执行)
                                          |
                                          v
                                返回带消息的 Task
                                          |
                                          v
                                A2AToolAdapter 提取文本
                                返回 {"response": "..."}
                                          |
                                          v
                                本地 Agent 继续使用结果
```

## 核心类型

### A2AToolAdapter

```rust
pub struct A2AToolAdapter {
    name: String,                           // 工具名称（唯一标识符）
    description: String,                     // 工具描述（用于 LLM）
    parameters: Value,                       // JSON Schema 输入参数
    client: Arc<ra2a::client::Client>,      // A2A 客户端（Arc 包装，线程安全）
}
```

**构造函数：**
```rust
pub fn new(
    name: String,
    description: String,
    parameters: Value,       // JSON Schema 对象
    client: ra2a::client::Client,
) -> Self
```

**Tool trait 实现：**

| 方法 | 实现 |
|------|------|
| `name() -> &str` | 返回工具名称 |
| `description() -> Option<&str>` | 返回工具描述 |
| `categories() -> &[ToolCategory]` | 返回 `[ToolCategory::External]` |
| `input_schema() -> Value` | 返回 JSON Schema 参数 |
| `call(input: Value) -> Result<Value, ToolError>` | 实际调用 |

**`call()` 工作原理：**
1. 从 JSON 输入中提取 `"message"` 字段
2. 创建带 `Part::text()` 的 A2A `Message::user()`
3. 包装到 `SendMessageRequest` 中
4. 调用 `self.client.send_message(&req).await`
5. 从嵌套 JSON 结构中提取响应文本（处理多种可能的响应格式）
6. 返回 `{"response": "<提取的文本>"}`

## 客户端实现

### 创建客户端

```rust
use ra2a::client::Client;

let client = Client::from_url("http://localhost:8080")?;
```

### 服务发现

```rust
let card = client.get_agent_card().await?;
println!("Agent 名称：{}", card.name);
println!("描述：{}", card.description);
```

### 完整客户端示例

```rust
use agentkit::a2a::A2AToolAdapter;
use agentkit::agent::ToolRegistry;
use agentkit::core::tool::Tool;
use ra2a::client::Client;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server_url = "http://localhost:8080";

    // 创建 A2A 客户端
    let client = Client::from_url(server_url)?;

    // 通过 AgentCard 验证连接
    let card = client.get_agent_card().await?;
    println!("已连接到：{} - {}", card.name, card.description);

    // 创建 A2A 工具适配器
    let a2a_tool = A2AToolAdapter::new(
        "a2a_time_agent".to_string(),
        "通过 A2A 协议调用远程时间助手".to_string(),
        serde_json::json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "发送到远程 Agent 的消息，例如'现在几点了？'"
                }
            },
            "required": ["message"]
        }),
        client,
    );

    // 注册到 ToolRegistry
    let tools = ToolRegistry::new().register(a2a_tool);

    // 直接调用工具
    let input = serde_json::json!({"message": "现在几点了？"});
    let result = a2a_tool.call(input).await?;
    println!("响应：{}", result["response"]);

    Ok(())
}
```

### 与 ToolAgent 集成

```rust
use agentkit::agent::ToolAgent;
use agentkit::prelude::Agent;
use agentkit::provider::OpenAiProvider;
use agentkit::a2a::A2AToolAdapter;
use ra2a::client::Client;

// 创建 A2A 客户端
let client = Client::from_url("http://localhost:8080")?;

// 创建 A2A 工具
let a2a_tool = A2AToolAdapter::new(
    "research_assistant".to_string(),
    "调用远程研究助手进行文档分析".to_string(),
    serde_json::json!({
        "type": "object",
        "properties": {
            "message": {
                "type": "string",
                "description": "要分析的研究问题或文档内容"
            }
        },
        "required": ["message"]
    }),
    client,
);

// 创建 Agent 并注册 A2A 工具
let agent = ToolAgent::builder()
    .provider(provider)
    .model("gpt-4o-mini")
    .system_prompt("你是一个研究助手，可以使用远程服务来分析文档。")
    .tool(a2a_tool)
    .build();

// 运行 Agent
let output = agent.run("请分析量子计算的最新进展".into()).await?;
```

## 服务端实现

### AgentExecutor trait

必须实现 `execute()` 和 `cancel()` 方法：

```rust
use ra2a::{
    server::{AgentExecutor, Event, EventQueue, RequestContext, ServerState, a2a_router},
    types::{AgentCard, Message, Part, Task, TaskState, TaskStatus},
};
use std::{future::Future, pin::Pin};
use chrono::Local;

struct TimeAgent;

impl TimeAgent {
    fn get_current_time() -> String {
        Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
    }

    fn process_message(input: &str) -> String {
        let lower = input.to_lowercase();
        if lower.contains("time") || lower.contains("clock") {
            format!("Current time: {}", Self::get_current_time())
        } else {
            format!("Received: {}", input)
        }
    }
}

impl AgentExecutor for TimeAgent {
    fn execute<'a>(
        &'a self,
        ctx: &'a RequestContext,
        queue: &'a EventQueue,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            // 1. 提取输入消息
            let input = ctx.message.as_ref()
                .and_then(ra2a::Message::text_content)
                .unwrap_or_default();

            // 2. 处理消息
            let response = Self::process_message(&input);

            // 3. 创建已完成的任务并附带响应
            let mut task = Task::new(&ctx.task_id, &ctx.context_id);
            task.status = TaskStatus::with_message(
                TaskState::Completed,
                Message::agent(vec![Part::text(response)]),
            );

            // 4. 发送事件
            queue.send(Event::Task(task))?;
            Ok(())
        })
    }

    fn cancel<'a>(
        &'a self,
        ctx: &'a RequestContext,
        queue: &'a EventQueue,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let mut task = Task::new(&ctx.task_id, &ctx.context_id);
            task.status = TaskStatus::new(TaskState::Canceled);
            queue.send(Event::Task(task))?;
            Ok(())
        })
    }
}
```

### 启动服务器

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 创建 AgentCard
    let card = AgentCard::new(
        "Time Assistant Agent",
        "http://localhost:8080",
        vec![], // AgentInterface 列表，空 = 使用默认
    );

    // 从执行器创建 ServerState
    let state = ServerState::from_executor(TimeAgent, card);

    // 构建带 A2A 路由的 Axum 路由器
    let app = axum::Router::new().merge(a2a_router(state));

    // 启动服务器
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

**AgentCard 端点：** `http://localhost:8080/.well-known/agent.json`

## 如何暴露 Agent 能力

1. 实现 `AgentExecutor` trait 的 `execute()` 和 `cancel()` 方法
2. 创建描述 Agent 的 `AgentCard`
3. 创建 `ServerState::from_executor(your_agent, card)`
4. 将 `a2a_router(state)` 挂载到 Axum 路由器
5. 使用 `axum::serve(listener, app)` 启动服务

## 如何消费其他 Agent

1. 创建 `Client::from_url("http://remote-agent:port")`
2. 可选：通过 `client.get_agent_card().await` 验证连接
3. 包装到 `A2AToolAdapter::new(name, description, parameters, client)`
4. 通过 `ToolRegistry::new().register(adapter)` 注册
5. 将工具传递给任何 Agent 类型（ToolAgent、ReActAgent 等）

## 与 Agent 系统集成

### ToolRegistry 集成

A2A 工具通过 `ToolSource::A2A` 注册：

```rust
pub enum ToolSource {
    BuiltIn,
    Skill,
    Mcp,
    A2A,     // 通过 A2A 协议加载的工具
    Custom,
}
```

按来源过滤：

```rust
let a2a_tools = registry.filter_by_source(ToolSource::A2A);
```

### Agent 消费

任何 Agent 类型都可以通过在构建时传递 A2A 工具来使用它们：

```rust
let agent = ToolAgent::builder()
    .provider(provider)
    .tool(a2a_tool)
    .build();
```

## 启用 A2A 功能

A2A 模块在 `a2a` 功能标志后面：

```toml
[dependencies]
agentkit = { version = "0.1", features = ["a2a"] }
```

或使用所有功能：

```toml
[dependencies]
agentkit = { version = "0.1", features = ["full"] }
```

## 使用多个远程 Agent

为每个远程 Agent 创建独立的 `Client` 实例和 `A2AToolAdapter`，然后将它们全部注册到同一个 `ToolRegistry`：

```rust
use agentkit::a2a::A2AToolAdapter;
use agentkit::agent::ToolRegistry;
use ra2a::client::Client;

let mut registry = ToolRegistry::new();

// 时间助手 Agent
let time_client = Client::from_url("http://time-agent:8080")?;
let time_tool = A2AToolAdapter::new(
    "time_assistant".to_string(),
    "获取当前时间".to_string(),
    serde_json::json!({
        "type": "object",
        "properties": {
            "message": { "type": "string" }
        },
        "required": ["message"]
    }),
    time_client,
);
registry = registry.register(time_tool);

// 研究助手 Agent
let research_client = Client::from_url("http://research-agent:8081")?;
let research_tool = A2AToolAdapter::new(
    "research_assistant".to_string(),
    "分析文档和研究问题".to_string(),
    serde_json::json!({
        "type": "object",
        "properties": {
            "message": { "type": "string" }
        },
        "required": ["message"]
    }),
    research_client,
);
registry = registry.register(research_tool);

// 现在两个远程 Agent 都可以通过本地 Agent 调用
let agent = ToolAgent::builder()
    .provider(provider)
    .model("gpt-4o-mini")
    .tool_registry(registry)
    .build();
```

## 最佳实践

1. **描述性的工具名称和描述** - LLM 使用这些信息来决定何时调用 A2A 工具。使其清晰具体。

2. **正确的 JSON Schema** - 定义结构良好的 `input_schema`，以便 LLM 准确知道要传递什么参数。

3. **连接验证** - 创建适配器前始终调用 `client.get_agent_card().await` 验证连接。

4. **使用 ToolRegistry** - 通过 `ToolRegistry` 注册 A2A 工具，以利用按 `ToolSource::A2A` 过滤、启用/禁用和命名空间支持。

5. **多个远程 Agent** - 为每个远程 Agent 创建独立的 `Client` 实例和 `A2AToolAdapter`，然后将它们全部注册到一个 `ToolRegistry`。

6. **错误处理** - 适配器捕获来自 `ra2a` 客户端的错误并转换为 `ToolError::Message`。在 Agent 循环中优雅地处理这些错误。

7. **分类为 External** - `A2AToolAdapter` 自动将自己分类为 `ToolCategory::External`，与内置工具区分开来。

## 架构概览

### 客户端组件

| 组件 | 用途 |
|------|------|
| `ra2a::client::Client` | A2A HTTP 客户端 |
| `A2AToolAdapter` | 将远程 Agent 适配为本地工具 |
| `AgentCard` | 远程 Agent 的服务发现元数据 |

### 服务端组件

| 组件 | 用途 |
|------|------|
| `AgentExecutor` | 必须由你的 Agent 实现的 trait |
| `ServerState` | 服务器状态管理 |
| `a2a_router` | A2A 协议的 Axum 路由器 |
| `EventQueue` | 事件发送队列 |
| `RequestContext` | 请求上下文（包含 task_id、context_id、消息） |

### 类型系统

通过 `ra2a::types` 重新导出的常用类型：

| 类型 | 用途 |
|------|------|
| `Message` | 消息载体（用户、助手、系统） |
| `Part` | 消息片段（文本、图像等） |
| `SendMessageRequest` | 发送消息请求 |
| `AgentCard` | Agent 服务发现卡片 |
| `Task` | 任务实体 |
| `TaskState` | 任务状态枚举（Pending、Running、Completed、Failed、Canceled） |
| `TaskStatus` | 带状态和可选消息的任务状态 |

## 相关文件

- `agentkit-a2a/src/lib.rs` - 主模块：A2AToolAdapter，重新导出
- `agentkit-a2a/Cargo.toml` - Crate 依赖（ra2a 0.9）
- `agentkit-a2a/README.md` - Crate 文档
- `agentkit/src/lib.rs` - 主库（重新导出 `agentkit_a2a as a2a`）
- `agentkit/src/agent/tool_registry.rs` - 工具注册表（含 ToolSource::A2A）
- `examples/a2a-server/src/main.rs` - 完整的 A2A 服务器示例
- `examples/a2a-client/src/main.rs` - 完整的 A2A 客户端示例
- `examples/README_A2A.md` - A2A 示例文档
