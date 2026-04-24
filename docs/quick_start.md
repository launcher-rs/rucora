# AgentKit 快速入门教程

本教程将在 10 分钟内带您构建第一个 LLM 应用。

## 📋 前提条件

- Rust 1.70+ 已安装
- 有一个 OpenAI API Key（或本地 Ollama）

## 🚀 5 分钟快速开始

### 步骤 1：创建项目

```bash
cargo new my-agent
cd my-agent
```

### 步骤 2：添加依赖

编辑 `Cargo.toml`：

```toml
[package]
name = "my-agent"
version = "0.1.0"
edition = "2024"

[dependencies]
agentkit = "0.1"
tokio = { version = "1", features = ["full"] }
anyhow = "1"
```

### 步骤 3：配置 API Key

```bash
# Windows (PowerShell)
$env:OPENAI_API_KEY="sk-your-api-key"
$env:MODEL_NAME="gpt-4o-mini"

# Linux/Mac
export OPENAI_API_KEY=sk-your-api-key
export MODEL_NAME=gpt-4o-mini
```

> 使用 Ollama 本地模型：`export OPENAI_BASE_URL=http://localhost:11434`，`export MODEL_NAME=qwen3.5:9b`

### 步骤 4：编写代码

编辑 `src/main.rs`：

```rust
use agentkit::agent::SimpleAgent;
use agentkit::prelude::Agent;
use agentkit::provider::OpenAiProvider;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 创建 Provider（从环境变量读取）
    let provider = OpenAiProvider::from_env()?;

    // 创建 SimpleAgent
    let agent = SimpleAgent::builder()
        .provider(provider)
        .model(std::env::var("MODEL_NAME").unwrap_or("gpt-4o-mini".into()))
        .system_prompt("你是友好的 AI 助手，请简洁地回答问题。")
        .temperature(0.7)
        .build();

    // 运行对话
    let output = agent.run("用一句话介绍 Rust 编程语言".into()).await?;
    println!("助手：{}", output.text().unwrap_or("无回复"));

    Ok(())
}
```

### 步骤 5：运行

```bash
cargo run
```

输出示例：
```
助手：Rust 是一门注重安全性和性能的系统级编程语言，由 Mozilla 开发。
```

## 🎯 10 分钟进阶：多轮对话

使用 `ChatAgent` 支持记住对话历史：

```rust
use agentkit::agent::ChatAgent;
use agentkit::prelude::Agent;
use agentkit::provider::OpenAiProvider;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let provider = OpenAiProvider::from_env()?;

    let agent = ChatAgent::builder()
        .provider(provider)
        .model(std::env::var("MODEL_NAME").unwrap_or("gpt-4o-mini".into()))
        .system_prompt("你是友好的 AI 助手。")
        .with_conversation(true)      // 启用对话历史
        .max_history_messages(20)     // 保留最近 20 条消息
        .build();

    // 第一轮
    agent.run("你好，我叫小明".into()).await?;

    // 第二轮（自动记住上一轮）
    let output = agent.run("你还记得我叫什么吗？".into()).await?;
    println!("助手：{}", output.text().unwrap_or("无回复"));
    // 输出：小明

    Ok(())
}
```

## 🔧 15 分钟进阶：添加工具

让 Agent 能够调用工具完成任务：

```rust
use agentkit::agent::ToolAgent;
use agentkit::prelude::Agent;
use agentkit::provider::OpenAiProvider;
use agentkit::tools::{DatetimeTool, EchoTool};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let provider = OpenAiProvider::from_env()?;

    let agent = ToolAgent::builder()
        .provider(provider)
        .model(std::env::var("MODEL_NAME").unwrap_or("gpt-4o-mini".into()))
        .system_prompt("你是有用的助手。当用户询问时间时使用日期时间工具。")
        .tool(DatetimeTool)  // 注册日期时间工具
        .tool(EchoTool)      // 注册回显工具
        .max_steps(10)
        .temperature(0.7)
        .top_p(0.9)
        .build();

    // Agent 会自动选择合适的工具
    let output = agent.run("现在几点了？".into()).await?;
    println!("助手：{}", output.text().unwrap_or("无回复"));

    Ok(())
}
```

## 📚 下一步学习

完成快速入门后，您可以：

1. 📖 查看 `agentkit/examples/` 目录下 20+ 完整示例
2. 🔧 尝试 [自定义工具](./skill_yaml_spec.md) 扩展 Agent 能力
3. 🤖 学习 ReAct/Reflect 等高级 Agent 模式
4. 📝 查看 [Skill 配置规范](./skill_yaml_spec.md) 了解技能系统

## 🆘 常见问题

### Q: 提示 "缺少 OPENAI_API_KEY"

A: 确保已设置环境变量：
```bash
export OPENAI_API_KEY=sk-your-api-key
export MODEL_NAME=gpt-4o-mini
```

### Q: 如何切换到 Ollama？

A: 设置 Ollama 环境变量：
```bash
export OPENAI_BASE_URL=http://localhost:11434
export MODEL_NAME=qwen3.5:9b
```

代码无需修改，`OpenAiProvider` 兼容 Ollama API。

### Q: 如何配置 LLM 参数（temperature、top_p 等）？

A: 所有 Agent 类型都支持完整的 LLM 参数配置：
```rust
let agent = ToolAgent::builder()
    .temperature(0.7)
    .top_p(0.9)
    .top_k(50)
    .max_tokens(2048)
    .frequency_penalty(0.1)
    .presence_penalty(0.1)
    .build();
```

---

**恭喜！您已完成快速入门教程！**
