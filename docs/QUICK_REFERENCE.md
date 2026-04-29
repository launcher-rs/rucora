# rucora 快速参考

## 快速开始

### 安装

```toml
[dependencies]
rucora = "0.2.0"
rucora-runtime = "0.2.0"
tokio = { version = "1", features = ["full"] }
serde_json = "1"
anyhow = "1"
```

### 第一个 Agent

```rust
use rucora::provider::OpenAiProvider;
use rucora_runtime::{DefaultRuntime, ToolRegistry};
use rucora_core::agent::AgentInput;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 创建 Provider
    let provider = OpenAiProvider::from_env()?;

    // 2. 创建运行时
    let runtime = DefaultRuntime::new(
        Arc::new(provider),
        ToolRegistry::new()
    ).with_system_prompt("你是有用的助手");

    // 3. 运行对话
    let input = AgentInput::new("用一句话介绍 Rust");
    let output = runtime.run(input).await?;

    println!("{}", output.text().unwrap_or("无回复"));
    Ok(())
}
```

---

## Provider 使用

### OpenAI

```rust
use rucora::provider::OpenAiProvider;

let provider = OpenAiProvider::from_env()?
    .with_default_model("gpt-4o-mini");
```

### Anthropic Claude

```rust
use rucora::provider::AnthropicProvider;

let provider = AnthropicProvider::from_env()?
    .with_default_model("claude-3-5-sonnet-20241022");
```

### Google Gemini

```rust
use rucora::provider::GeminiProvider;

let provider = GeminiProvider::from_env()?
    .with_default_model("gemini-1.5-pro");
```

### OpenRouter（多模型）

```rust
use rucora::provider::OpenRouterProvider;

let provider = OpenRouterProvider::from_env()?
    .with_default_model("anthropic/claude-3-5-sonnet");
```

### 环境变量

| Provider | 环境变量 |
|----------|----------|
| OpenAI | `OPENAI_API_KEY` |
| Anthropic | `ANTHROPIC_API_KEY` |
| Gemini | `GOOGLE_API_KEY` |
| OpenRouter | `OPENROUTER_API_KEY` |
| DeepSeek | `DEEPSEEK_API_KEY` |
| Moonshot | `MOONSHOT_API_KEY` |

---

## Agent 使用

### 简单对话（独立模式）

```rust
use rucora::agent::DefaultAgent;

let agent = DefaultAgent::builder()
    .provider(provider)
    .system_prompt("你是有用的助手")
    .tool(rucora::tools::EchoTool)
    .build();

let input = rucora::core::agent::AgentInput::new("你好");
let output = agent.run(input).await?;

if let Some(content) = output.text() {
    println!("回复：{}", content);
}
```

### 复杂任务（Runtime 模式）

```rust
use rucora::provider::OpenAiProvider;
use rucora::agent::DefaultAgent;
use rucora_runtime::{DefaultRuntime, ToolRegistry};

// 创建 Agent
let agent = DefaultAgent::builder()
    .provider(provider)
    .model("gpt-4o-mini")
    .build();

// 创建 Runtime（带工具）
let runtime = DefaultRuntime::new(
    Arc::new(provider),
    ToolRegistry::new()
        .register(rucora::tools::FileReadTool::new())
);

// 运行
let input = AgentInput::new("帮我读取 README.md 文件");
let output = runtime.run_with_agent(&agent, input).await?;
```

### 自定义 Agent

```rust
use rucora_core::agent::{Agent, AgentContext, AgentDecision, AgentInput};
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

---

## AgentInput 使用

### 简单输入

```rust
let input = AgentInput::new("你好");
```

### 带上下文

```rust
let input = AgentInput::with_context(
    "帮我查询天气",
    serde_json::json!({"location": "北京"})
);
```

### Builder 模式

```rust
let input = AgentInput::builder("帮我查询天气")
    .with_context("location", "北京")
    .with_context("date", "今天")
    .build();
```

---

## AgentOutput 使用

### 访问内容

```rust
let output = run_agent().await?;

// 获取文本内容
if let Some(content) = output.text() {
    println!("回复：{}", content);
}

// 访问原始值
println!("输出：{}", output.value);
```

### 访问统计

```rust
println!("对话轮数：{}", output.message_count());
println!("工具调用：{} 次", output.tool_call_count());
```

---

## Runtime 使用

### 基本使用

```rust
let runtime = DefaultRuntime::new(provider, tools)
    .with_system_prompt("你是有用的助手")
    .with_max_steps(5);

let output = runtime.run(input).await?;
```

### 流式执行

```rust
use futures_util::StreamExt;
use rucora_runtime::ChannelEvent;

let mut stream = runtime.run_stream(input);
while let Some(event) = stream.next().await {
    match event? {
        ChannelEvent::TokenDelta(delta) => {
            print!("{}", delta.delta);
        }
        ChannelEvent::ToolCall(call) => {
            println!("\n工具调用：{}", call.name);
        }
        _ => {}
    }
}
```

---

## 工具使用

### 内置工具

```rust
use rucora_runtime::ToolRegistry;

let tools = ToolRegistry::new()
    .register(rucora::tools::FileReadTool::new())
    .register(rucora::tools::ShellTool::new())
    .register(rucora::tools::HttpRequestTool::new());
```

### 自定义工具

```rust
use rucora_core::tool::{Tool, ToolCategory};
use rucora_core::error::ToolError;
use async_trait::async_trait;
use serde_json::{json, Value};

struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str { "echo" }
    fn description(&self) -> Option<&str> { Some("回显输入") }
    fn categories(&self) -> &'static [ToolCategory] { &[ToolCategory::Basic] }
    
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "text": {"type": "string", "description": "要回显的文本"}
            },
            "required": ["text"]
        })
    }

    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let text = input.get("text").and_then(|v| v.as_str()).unwrap_or("");
        Ok(json!({"echo": text}))
    }
}
```

---

## 错误处理

### 错误类型

```rust
use rucora_core::error::{AgentError, ProviderError, ToolError};

// Provider 错误
match provider.chat(request).await {
    Ok(response) => println!("成功"),
    Err(ProviderError::Network { message, .. }) => println!("网络错误：{}", message),
    Err(ProviderError::Api { status, message }) => println!("API 错误：{} - {}", status, message),
    _ => println!("其他错误"),
}

// Agent 错误
match agent.run(input).await {
    Ok(output) => println!("成功"),
    Err(AgentError::MaxStepsExceeded { max_steps }) => println!("超过最大步数：{}", max_steps),
    Err(AgentError::Provider { source }) => println!("Provider 错误：{}", source),
    _ => println!("其他错误"),
}
```

---

## 配置使用

### 从配置文件加载

```rust
use rucora::config::rucoraConfig;

let config = rucoraConfig::load().await?;
let provider = rucoraConfig::build_provider(&config)?;
```

### 环境变量

```bash
# OpenAI
export OPENAI_API_KEY=sk-...

# Anthropic
export ANTHROPIC_API_KEY=sk-ant-...

# Google Gemini
export GOOGLE_API_KEY=...

# 自定义 Base URL
export OPENAI_BASE_URL=https://api.openai.com/v1
```

---

## 最佳实践

### 1. 选择合适的运行模式

- **简单对话**: 使用 `agent.run()`
- **复杂任务**: 使用 `runtime.run_with_agent()`

### 2. 错误处理

```rust
match agent.run(input).await {
    Ok(output) => {
        if let Some(content) = output.text() {
            println!("{}", content);
        }
    }
    Err(e) => {
        eprintln!("错误：{}", e);
        // 记录详细错误信息
        eprintln!("详细：{:?}", e);
    }
}
```

### 3. 资源管理

```rust
// 使用 Arc 共享 Provider
let provider = Arc::new(OpenAiProvider::from_env()?);

// 多个 Agent 共享
use rucora::agent::DefaultAgent;
let agent1 = DefaultAgent::builder().provider(provider.clone()).model("gpt-4o-mini").build();
let agent2 = DefaultAgent::builder().provider(provider.clone()).model("gpt-4o-mini").build();
```

### 4. 性能优化

```rust
// 缓存 tool_definitions
let tool_defs = tools.definitions();

// 使用 spawn_blocking 处理阻塞 IO
let content = tokio::task::spawn_blocking(move || {
    std::fs::read_to_string(path)
}).await??;
```

---

## 常见问题

### Q: 如何选择 Provider？

A: 根据需求选择：
- **高质量**: OpenAI GPT-4, Anthropic Claude
- **性价比**: DeepSeek, OpenRouter
- **本地部署**: Ollama
- **多模型**: OpenRouter

### Q: Agent 和 Runtime 有什么区别？

A: 
- **Agent**: 负责思考、决策（大脑）
- **Runtime**: 负责执行、调用（身体）

详见：[Agent 和 Runtime 关系](docs/agent_runtime_relationship.md)

### Q: 如何调试？

A: 启用详细日志：
```bash
export RUST_LOG=rucora=debug,rucora_runtime=debug
```

---

## 更多资源

- [完整示例](examples/rucora-examples-complete/src/agent_example.rs)
- [Agent 架构文档](docs/agent_runtime_relationship.md)
- [变更日志](CHANGELOG.md)
- [项目总结](PROJECT_SUMMARY.md)

---

**版本**: v0.2.0  
**最后更新**: 2026-03-21
