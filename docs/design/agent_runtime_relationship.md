# Agent 架构说明

## 概述

在 rucora 框架中，**Agent** 是核心概念，它将"决策"与"执行"两层能力内聚在一起，分工明确：

```
┌─────────────────────────────────────────────────┐
│  Agent (智能体)                                  │
│  ┌───────────────────────────────────────────┐  │
│  │  决策层 (think) - "大脑"                  │  │
│  │  - 负责思考、决策、规划                    │  │
│  │  - 决定"做什么" (What to do)              │  │
│  └───────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────┐  │
│  │  执行层 (DefaultExecution) - "身体"       │  │
│  │  - 负责工具调用循环、流式执行              │  │
│  │  - 负责"怎么做" (How to do)               │  │
│  └───────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘
```

## Agent 类型

| Agent 类型 | 职责 | 适用场景 |
|------------|------|----------|
| `SimpleAgent` | 简单问答 | 翻译、总结、一次性任务 |
| `ChatAgent` | 多轮对话 | 客服、心理咨询、闲聊 |
| `ToolAgent` | 工具调用 | 执行具体任务（默认选择） |
| `ReActAgent` | 推理 + 行动 | 多步推理任务 |
| `ReflectAgent` | 反思迭代 | 代码生成、写作 |
| `SummaryAgent` | 长文本摘要 | 文档总结、要点提取 |

## 两种使用模式

### 模式 1：简单对话

适合直接对话、无需工具调用的场景：

```rust
use rucora::agent::SimpleAgent;
use rucora::provider::OpenAiProvider;
use rucora::prelude::Agent;

let provider = OpenAiProvider::from_env()?;

let agent = SimpleAgent::builder(provider)
    .model("gpt-4o-mini")
    .system_prompt("你是有用的助手")
    .build();

let output = agent.run("你好".into()).await?;
println!("{}", output.text().unwrap_or("无回复"));
```

**特点**：
- ✅ 简单直接
- ✅ 无需配置工具
- ❌ 不支持工具调用

### 模式 2：工具调用（复杂场景）

适合需要工具调用、多轮推理、复杂编排的场景：

```rust
use rucora::agent::ToolAgent;
use rucora::provider::OpenAiProvider;
use rucora::tools::HttpRequestTool;
use rucora::prelude::Agent;

let provider = OpenAiProvider::from_env()?;

// 创建 ToolAgent（注册工具）
let agent = ToolAgent::builder(provider)
    .model("gpt-4o-mini")
    .system_prompt("你是有用的助手")
    .tool(HttpRequestTool::new())
    .max_steps(5)
    .build();

let output = agent.run("帮我查询北京天气".into()).await?;

// Agent 会：
// 1. 思考是否需要调用工具
// 2. 执行工具调用（http_request）
// 3. 将结果回传给 LLM
// 4. 再次思考，生成最终回复
```

**特点**：
- ✅ 支持工具调用
- ✅ 支持多轮推理
- ✅ 支持并发工具执行
- ✅ 可观测性强（事件流）
- ❌ 配置稍复杂

## Agent 决策类型

每种 Agent 通过内部策略生成决策，`ToolAgent` 会自动判断何时调用工具：

```rust
pub enum AgentDecision {
    /// 调用 LLM 进行对话
    Chat { request: ChatRequest },

    /// 调用工具
    ToolCall {
        name: String,
        input: Value
    },

    /// 直接返回结果
    Return(Value),

    /// 需要更多思考（继续循环）
    ThinkAgain,

    /// 停止执行
    Stop,
}
```

## 执行流程

```
1. 接收用户输入
   │
   ▼
2. 创建 AgentContext（上下文）
   │
   ▼
3. Agent 思考，生成决策
   │
   ▼
4. 根据决策执行：
   │
   ├─ Chat → 调用 Provider.chat()
   │         ├─ 有工具调用 → 执行工具 → 回到步骤 3
   │         └─ 无工具调用 → 返回结果
   │
   ├─ ToolCall → 调用 Tool.call()
   │             回到步骤 3
   │
   ├─ Return → 直接返回
   │
   └─ Stop → 返回空结果
   │
   ▼
5. 检查步数限制，超出则报错
```

## 流式输出

```rust
use rucora::prelude::*;
use rucora::agent::{SimpleAgent, AgentStream};

let agent = SimpleAgent::builder(provider)
    .model("gpt-4o-mini")
    .build();

// 逐事件处理
let mut stream = AgentStream::new(agent.run_stream("你好".into()));
while let Some(event) = stream.next().await {
    match event? {
        ChannelEvent::TokenDelta(delta) => print!("{}", delta.delta),
        _ => {}
    }
}

// 直接获取最终文本
let text = agent.run_stream_text("你好").await?;
```

## 自定义 Agent

你可以实现自己的 Agent 逻辑（实现 `Agent` trait）：

```rust
use rucora_core::agent::{Agent, AgentContext, AgentDecision, AgentInput, AgentOutput};
use async_trait::async_trait;

struct WeatherAgent;

#[async_trait]
impl Agent for WeatherAgent {
    async fn think(&self, context: &AgentContext) -> AgentDecision {
        if context.input.text().contains("天气") {
            AgentDecision::ToolCall {
                name: "weather".to_string(),
                input: serde_json::json!({"location": "北京"}),
            }
        } else {
            AgentDecision::Chat {
                request: context.default_chat_request(),
            }
        }
    }

    fn name(&self) -> &str { "weather_agent" }
}
```

## 总结

| 问题 | 答案 |
|------|------|
| **什么时候用 `SimpleAgent`？** | 简单对话、无需工具、快速原型 |
| **什么时候用 `ToolAgent`？** | 需要工具调用、多步任务（默认选择） |
| **什么时候用 `ReActAgent`？** | 3-5 步多步推理任务 |
| **什么时候用 `ChatAgent`？** | 需要自动记忆的多轮对话 |
| **推荐使用哪种？** | 默认使用 `ToolAgent`，按需选择 |

## 架构优势

1. **决策与执行分离**: Agent 专注思考（think），`DefaultExecution` 专注执行
2. **能力内聚**: 工具调用循环、流式执行、策略检查内聚到 Agent
3. **易于测试**: 各种 Agent 可以独立测试
4. **可扩展**: 轻松实现自定义 Agent
