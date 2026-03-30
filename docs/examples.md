# AgentKit 示例索引

> 完整的示例代码列表和说明

## 快速开始

### Hello World - 最简单的 Agent

**文件**: [`examples/hello_world.rs`](../examples/hello_world.rs)

**说明**: 展示如何用最少的代码创建一个 Agent 应用。

**运行**:
```bash
export OPENAI_API_KEY=sk-your-key
cargo run --example hello_world
```

**学习点**:
- 创建 Provider
- 创建 Agent
- 运行对话

**代码量**: ~80 行

---

## 对话示例

### 基础对话 - 支持多轮对话

**文件**: [`examples/chat_basic.rs`](../examples/chat_basic.rs)

**说明**: 展示如何创建支持多轮对话的 Agent，记住对话历史。

**运行**:
```bash
export OPENAI_API_KEY=sk-your-key
cargo run --example chat_basic
```

**学习点**:
- 启用对话历史
- 设置最大消息数
- 多轮对话测试

**代码量**: ~70 行

### 带工具对话 - 调用工具

**文件**: [`examples/chat_with_tools.rs`](../examples/chat_with_tools.rs)

**说明**: 展示如何创建支持工具调用的 Agent。

**运行**:
```bash
export OPENAI_API_KEY=sk-your-key
cargo run --example chat_with_tools
```

**学习点**:
- 注册工具
- 工具调用
- 工具结果处理

**代码量**: ~80 行

---

## Skills 示例

### Agent + Skills 完整示例

**文件**: [`examples/agentkit-skills-example/src/main.rs`](../examples/agentkit-skills-example/src/main.rs)

**说明**: 展示 Agent 如何自动调用 Skills 完成任务。

**运行**:
```bash
cd examples/agentkit-skills-example
cargo run
```

**学习点**:
- Skills 加载
- Skills 转换为 Tools
- 构建系统提示词
- Agent 自动调用 Skills

**代码量**: ~200 行

---

## 按功能分类

### Provider 相关

| 示例 | 说明 | 文件 |
|------|------|------|
| hello_world | 使用 OpenAI Provider | hello_world.rs |
| hello_world | 使用 Ollama Provider | hello_world.rs |

### Agent 相关

| 示例 | 说明 | 文件 |
|------|------|------|
| hello_world | 基础 Agent | hello_world.rs |
| chat_basic | 多轮对话 Agent | chat_basic.rs |
| chat_with_tools | 带工具的 Agent | chat_with_tools.rs |

### Tools 相关

| 示例 | 说明 | 使用的工具 |
|------|------|------|
| chat_with_tools | DatetimeTool, EchoTool | chat_with_tools.rs |

### Skills 相关

| 示例 | 说明 | 文件 |
|------|------|------|
| agentkit-skills-example | 完整 Skills 集成 | agentkit-skills-example/ |

---

## 按难度分类

### 入门级 ⭐

适合第一次使用 AgentKit 的用户。

- [hello_world.rs](../examples/hello_world.rs) - Hello World
- [chat_basic.rs](../examples/chat_basic.rs) - 基础对话

### 进阶级 ⭐⭐

适合已经了解基础用法的用户。

- [chat_with_tools.rs](../examples/chat_with_tools.rs) - 带工具对话
- [agentkit-skills-example](../examples/agentkit-skills-example/) - Skills 集成

### 高级 ⭐⭐⭐

适合需要实现复杂功能的用户。

- 自定义 Agent（待添加）
- 自定义 Tool（待添加）
- 自定义 Skill（待添加）

---

## 代码片段

### 创建 Provider

```rust
use agentkit::provider::OpenAiProvider;

// 从环境变量加载
let provider = OpenAiProvider::from_env()?;

// 或直接指定
let provider = OpenAiProvider::new("sk-your-key")?;
```

### 创建 Agent

```rust
use agentkit::agent::DefaultAgent;

let agent = DefaultAgent::builder()
    .provider(provider)
    .model("gpt-4o-mini")
    .system_prompt("你是有用的助手")
    .build();
```

### 运行对话

```rust
let output = agent.run("你好").await?;
println!("{}", output.text().unwrap_or("无回复"));
```

### 启用对话历史

```rust
let agent = DefaultAgent::builder()
    .provider(provider)
    .model("gpt-4o-mini")
    .with_conversation(true)  // 启用对话历史
    .with_max_messages(20)    // 保留最近 20 条消息
    .build();
```

### 注册工具

```rust
use agentkit::tools::{DatetimeTool, EchoTool};

let agent = DefaultAgent::builder()
    .provider(provider)
    .model("gpt-4o-mini")
    .tool(DatetimeTool)
    .tool(EchoTool)
    .build();
```

---

## 运行所有示例

```bash
# Hello World
cargo run --example hello_world

# 基础对话
cargo run --example chat_basic

# 带工具对话
cargo run --example chat_with_tools

# Skills 示例
cd examples/agentkit-skills-example
cargo run
```

---

## 贡献示例

欢迎贡献示例代码！请遵循以下规范：

### 文件结构

```rust
//! 示例名称
//!
//! 简要说明
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example example_name
//! ```

use agentkit::...;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 代码
}
```

### 要求

1. **可运行**: 示例必须可以编译和运行
2. **有注释**: 关键步骤要有注释
3. **有文档**: 在文件头部说明用途和运行方法
4. **错误处理**: 使用 `anyhow::Result` 处理错误
5. **日志输出**: 使用 `tracing` 输出日志

---

## 相关文档

- [快速开始](quick_start.md)
- [用户指南](user_guide.md)
- [故障排除](TROUBLESHOOTING.md)
- [Skill 配置规范](skill_yaml_spec.md)
