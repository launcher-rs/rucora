# rucora 快速参考

## 快速开始

### 安装

```toml
[dependencies]
rucora = "0.5"
tokio = { version = "1", features = ["full"] }
serde_json = "1"
anyhow = "1"
```

### 第一个 Agent

```rust
use rucora::agent::SimpleAgent;
use rucora::prelude::Agent;
use rucora::provider::OpenAiProvider;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let provider = OpenAiProvider::from_env()?;

    let agent = SimpleAgent::builder(provider)
        .model("gpt-4o-mini")
        .system_prompt("你是有用的助手")
        .build();

    let output = agent.run("你好".into()).await?;
    println!("{}", output.text().unwrap_or("无回复"));
    Ok(())
}
```

---

## Agent 类型

| Agent | 场景 | 说明 |
|-------|------|------|
| `SimpleAgent` | 简单问答 | 翻译、总结、一次性任务 |
| `ChatAgent` | 多轮对话 | 客服、心理咨询、闲聊 |
| `ToolAgent` | 工具调用 | 执行具体任务（默认选择） |
| `ReActAgent` | 多步推理 | 推理 + 行动 |
| `ReflectAgent` | 反思迭代 | 代码生成、写作 |
| `SummaryAgent` | 长文本摘要 | 文档总结、要点提取 |

### 简单对话

```rust
let agent = SimpleAgent::builder(provider)
    .model("gpt-4o-mini")
    .system_prompt("你是翻译助手")
    .build();

let output = agent.run("翻译 'Hello' 为中文".into()).await?;
```

### 工具调用

```rust
let agent = ToolAgent::builder(provider)
    .model("gpt-4o-mini")
    .system_prompt("你是有用的助手")
    .tool(rucora::tools::EchoTool)
    .tool(rucora::tools::DatetimeTool)
    .max_steps(10)
    .build();

let output = agent.run("现在几点了？".into()).await?;
```

### 多轮对话

```rust
let agent = ChatAgent::builder(provider)
    .model("gpt-4o-mini")
    .system_prompt("你是友好的助手")
    .with_conversation(true)
    .build();

agent.run("你好".into()).await?;
agent.run("我叫小明".into()).await?;
agent.run("我叫什么？".into()).await?;
```

---

## Provider 使用

### OpenAI

```rust
let provider = OpenAiProvider::from_env()?
    .with_default_model("gpt-4o-mini");
```

### Anthropic Claude

```rust
let provider = AnthropicProvider::from_env()?
    .with_default_model("claude-3-5-sonnet-20241022");
```

### Google Gemini

```rust
let provider = GeminiProvider::from_env()?
    .with_default_model("gemini-1.5-pro");
```

### 环境变量

| Provider | 环境变量 |
|----------|----------|
| OpenAI | `OPENAI_API_KEY` |
| Anthropic | `ANTHROPIC_API_KEY` |
| Gemini | `GOOGLE_API_KEY` |
| Ollama | `OPENAI_BASE_URL` |

---

## AgentInput 使用

```rust
// 简单输入
let input = AgentInput::new("你好")?;

// 带上下文
let input = AgentInput::with_context(
    "帮我查询天气",
    serde_json::json!({"location": "北京"})
)?;

// Builder 模式
let input = AgentInput::builder("帮我查询天气")?
    .with_context("location", "北京")?
    .build()?;
```

---

## AgentOutput 使用

```rust
// 获取文本内容
if let Some(content) = output.text() {
    println!("回复：{}", content);
}

// 访问统计
println!("对话轮数：{}", output.message_count());
println!("工具调用：{} 次", output.tool_call_count());
```

---

## 流式输出

```rust
use rucora::prelude::*;
use rucora::agent::{SimpleAgent, AgentStream};
use rucora::provider::OpenAiProvider;

let agent = SimpleAgent::builder(OpenAiProvider::from_env()?)
    .model("gpt-4o-mini")
    .build();

// 方式 1：逐事件处理
let mut stream = AgentStream::new(agent.run_stream("你好".into()));
while let Some(event) = stream.next().await {
    match event? {
        ChannelEvent::TokenDelta(delta) => print!("{}", delta.delta),
        _ => {}
    }
}

// 方式 2：直接拼接最终文本（推荐）
let text = agent.run_stream_text("你好").await?;
```

---

## 工具使用

### 内置工具

```rust
let agent = ToolAgent::builder(provider)
    .model("gpt-4o-mini")
    .tool(rucora::tools::FileReadTool::new())
    .tool(rucora::tools::ShellTool::new())
    .tool(rucora::tools::HttpRequestTool::new())
    .build();
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
    Err(e) => println!("Agent 错误：{}", e),
}
```

---

## 对话管理

```rust
use rucora::conversation::ConversationManager;

let mut conv = ConversationManager::new()
    .with_system_prompt("你是项目助手")
    .with_max_messages(20);

conv.add_user_message("你好");
conv.add_assistant_message("你好！有什么可以帮助你的？");

// 获取历史
let messages = conv.get_messages();

// 序列化 / 反序列化
let json = conv.to_json()?;
let restored = ConversationManager::from_json(&json)?;
```

---

## 中间件

```rust
use rucora::middleware::{MiddlewareChain, LoggingMiddleware, RateLimitMiddleware};

let chain = MiddlewareChain::new()
    .with(LoggingMiddleware::new())
    .with(RateLimitMiddleware::new(100)  // 100 次/分钟
        .with_window_secs(60));
```

---

## 环境变量

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

## 常见问题

### Q: 如何选择 Agent 类型？

A: 根据需求选择：
- **简单问答**: `SimpleAgent`
- **多轮对话**: `ChatAgent`
- **工具调用**: `ToolAgent`（默认选择）
- **多步推理**: `ReActAgent`
- **高质量输出**: `ReflectAgent`

### Q: 如何调试？

A: 启用详细日志：
```bash
export RUST_LOG=rucora=debug
```

---

## 更多资源

- [快速开始](quick_start.md)
- [用户指南](user_guide.md)
- [示例集合](cookbook.md)
- [常见问题](faq.md)

---

**版本**: v0.6.0
