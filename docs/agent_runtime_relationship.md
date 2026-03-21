# Agent 和 Runtime 关系说明

## 概述

在 AgentKit 框架中，**Agent** 和 **Runtime** 是两个核心概念，它们分工明确、相互配合：

```
┌─────────────────────────────────────────────────┐
│  Agent (智能体) - "大脑"                        │
│  - 负责思考、决策、规划                          │
│  - 决定"做什么" (What to do)                    │
└─────────────────────────────────────────────────┘
                    ↓ 决策 (Decision)
┌─────────────────────────────────────────────────┐
│  Runtime (运行时) - "身体"                      │
│  - 负责执行、调用、编排                          │
│  - 负责"怎么做" (How to do)                     │
└─────────────────────────────────────────────────┘
```

## 核心区别

| 特性 | Agent | Runtime |
|------|-------|---------|
| **职责** | 思考、决策、规划 | 执行、调用、编排 |
| **输入** | 用户请求、上下文 | Agent 的决策 |
| **输出** | 决策（做什么） | 最终结果 |
| **依赖** | 可以独立运行 | 依赖 Provider/Tools |
| **复杂度** | 可简单可复杂 | 相对固定 |
| **类比** | 大脑（思考） | 身体（执行） |

## 两种使用模式

### 模式 1: Agent 独立运行（简单场景）

适合直接对话、无需工具调用的场景：

```rust
use agentkit::agent::DefaultAgent;
use agentkit::provider::OpenAiProvider;
use agentkit_core::agent::Agent;

// 创建 Agent
let agent = DefaultAgent::builder()
    .provider(provider)
    .system_prompt("你是有用的助手")
    .build();

// 独立运行（简单对话）
let input = AgentInput::new("你好");
let output = agent.run(input).await?;

// 输出：AgentOutput { value: {"content": "你好！..."}, ... }
```

**特点**：
- ✅ 简单直接
- ✅ 无需配置 Runtime
- ❌ 不支持工具调用
- ❌ 不支持复杂编排

### 模式 2: Agent + Runtime（复杂场景）

适合需要工具调用、多轮对话、复杂编排的场景：

```rust
use agentkit::agent::DefaultAgent;
use agentkit::provider::OpenAiProvider;
use agentkit_runtime::{DefaultRuntime, ToolRegistry};

// 创建 Agent
let agent = DefaultAgent::builder()
    .provider(provider.clone())
    .system_prompt("你是有用的助手")
    .build();

// 创建 Runtime（配置工具等）
let runtime = DefaultRuntime::new(provider, tools)
    .with_system_prompt("你是有用的助手")
    .with_max_steps(5);

// 使用 Runtime 运行 Agent
let input = AgentInput::new("帮我查询天气");
let output = runtime.run_with_agent(&agent, input).await?;

// Runtime 会：
// 1. 执行 Agent 的 think() 获取决策
// 2. 根据决策调用 Provider 或 Tools
// 3. 管理对话历史和上下文
// 4. 返回最终结果
```

**特点**：
- ✅ 支持工具调用
- ✅ 支持多轮对话
- ✅ 支持复杂编排
- ✅ 可观测性强（事件流）
- ❌ 配置稍复杂

## Agent 决策类型

Agent 通过 `think()` 方法返回 `AgentDecision`，告诉 Runtime 下一步做什么：

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

## Runtime 执行流程

```
1. 接收 Agent 输入
   │
   ▼
2. 创建 AgentContext（上下文）
   │
   ▼
3. 调用 Agent.think(context) 获取决策
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
   ├─ ThinkAgain → 回到步骤 3
   │
   └─ Stop → 返回空结果
   │
   ▼
5. 检查步数限制，超出则报错
```

## 实际示例对比

### 场景：简单对话

```rust
// 使用 Agent 独立运行
let agent = DefaultAgent::builder()
    .provider(provider)
    .build();

let response = agent.run("你好").await?;
// 简单、直接
```

### 场景：需要工具调用

```rust
// 使用 Agent + Runtime
let agent = DefaultAgent::builder()
    .provider(provider.clone())
    .build();

let runtime = DefaultRuntime::new(provider, tools)
    .with_max_steps(5);

let response = runtime.run_with_agent(&agent, "帮我查询北京天气").await?;
// Runtime 会：
// 1. 让 Agent 思考
// 2. 执行工具调用（weather_query）
// 3. 将结果返回给 Agent
// 4. Agent 再次思考，生成最终回复
```

## 自定义 Agent

你可以实现自己的 Agent 逻辑：

```rust
struct WeatherAgent;

#[async_trait]
impl Agent for WeatherAgent {
    async fn think(&self, context: &AgentContext) -> AgentDecision {
        // 自定义思考逻辑
        if context.input.text().contains("天气") {
            AgentDecision::ToolCall {
                name: "weather".to_string(),
                input: json!({"location": "北京"}),
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
| **什么时候用 Agent 独立运行？** | 简单对话、无需工具、快速原型 |
| **什么时候用 Agent + Runtime？** | 需要工具调用、多轮对话、复杂编排 |
| **Agent 可以脱离 Runtime 吗？** | 可以，简单场景下独立运行 |
| **Runtime 可以脱离 Agent 吗？** | 可以，Runtime 本身有默认的对话逻辑 |
| **推荐使用哪种？** | 复杂应用推荐 Agent + Runtime 模式 |

## 架构优势

1. **职责分离**: Agent 专注思考，Runtime 专注执行
2. **灵活组合**: 可以独立使用，也可以组合使用
3. **易于测试**: Agent 和 Runtime 可以分别测试
4. **可扩展**: 轻松实现自定义 Agent 或 Runtime
5. **向后兼容**: 现有 Runtime 代码无需修改
