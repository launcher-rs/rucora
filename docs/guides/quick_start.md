# rucora 快速入门教程

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
rucora = "0.5"
tokio = { version = "1", features = ["full"] }
serde_json = "1"
anyhow = "1"
```

### 步骤 3：配置 API Key

```bash
# Windows (PowerShell)
$env:OPENAI_API_KEY="sk-your-api-key"

# Linux/Mac
export OPENAI_API_KEY=sk-your-api-key
```

### 步骤 4：编写代码

编辑 `src/main.rs`：

```rust
use rucora::agent::SimpleAgent;
use rucora::prelude::Agent;
use rucora::provider::OpenAiProvider;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🤖 rucora 快速开始\n");

    // 1. 创建 Provider
    println!("1️⃣ 创建 Provider...");
    let provider = OpenAiProvider::from_env()?;

    // 2. 创建 Agent
    println!("2️⃣ 创建 Agent...");
    let agent = SimpleAgent::builder(provider)
        .model("gpt-4o-mini")
        .system_prompt("你是有用的助手，用简洁的中文回答")
        .build();

    // 3. 开始对话
    println!("3️⃣ 开始对话...\n");
    let output = agent.run("用一句话介绍 Rust 编程语言".into()).await?;

    // 4. 显示结果
    println!("💬 助手回复：\n");
    println!("{}", output.text().unwrap_or("无回复"));

    Ok(())
}
```

### 步骤 5：运行

```bash
cargo run
```

输出：
```
🤖 rucora 快速开始

1️⃣ 创建 Provider...
2️⃣ 创建工具...
3️⃣ 创建运行时...
4️⃣ 开始对话...

💬 助手回复：

Rust 是一门系统编程语言，专注于安全性和性能，由 Mozilla 研发。
```

## 🎯 10 分钟进阶：添加工具

### 步骤 1：添加工具依赖

```toml
[dependencies]
rucora = { version = "0.5", features = ["builtin-tools"] }
tokio = { version = "1", features = ["full"] }
serde_json = "1"
anyhow = "1"
```

### 步骤 2：修改代码

```rust
use rucora::agent::ToolAgent;
use rucora::prelude::Agent;
use rucora::provider::OpenAiProvider;
use rucora::tools::{FileReadTool, FileWriteTool};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🤖 rucora 带工具示例\n");

    // 创建 Provider
    let provider = OpenAiProvider::from_env()?;

    // 创建 ToolAgent，注册文件工具
    let agent = ToolAgent::builder(provider)
        .model("gpt-4o-mini")
        .system_prompt(
            "你是有用的助手。你可以使用工具来完成任务。
            可用工具：
            - file_read: 读取文件内容
            - file_write: 写入文件内容"
        )
        .tool(FileReadTool::new())
        .tool(FileWriteTool::new())
        .max_steps(5)
        .build();

    // 运行 Agent（会自动调用工具）
    let output = agent.run("请创建一个文件 hello.txt，内容为'Hello, rucora!'".into()).await?;

    println!("💬 助手回复：\n");
    println!("{}", output.text().unwrap_or("无回复"));

    Ok(())
}
```

### 步骤 3：运行

```bash
cargo run
```

## 📚 下一步学习

完成快速入门后，您可以：

1. 📖 阅读 [用户指南](./user_guide.md) 深入了解
2. 🍳 查看 [示例集合](./cookbook.md) 学习更多用例
3. 🔧 尝试 [自定义工具](./user_guide.md#自定义工具)
4. 💰 学习 [成本管理](./user_guide.md#token-计数和成本管理)

## 🆘 常见问题

### Q: 提示 "缺少 OPENAI_API_KEY"

A: 确保已设置环境变量：
```bash
export OPENAI_API_KEY=sk-your-api-key
```

### Q: 如何切换到 Ollama？

A: 设置 Ollama 环境变量：
```bash
export OLLAMA_BASE_URL=http://localhost:11434
```

然后修改代码：
```rust
use rucora::provider::OllamaProvider;
let provider = OllamaProvider::from_env();
```

### Q: 如何添加更多工具？

A: 在 ToolAgent 中注册：
```rust
let agent = ToolAgent::builder(provider)
    .tool(FileReadTool::new())
    .tool(FileWriteTool::new())
    .tool(HttpRequestTool::new())
    .build();
```

---

**恭喜！您已完成快速入门教程！** 🎉
