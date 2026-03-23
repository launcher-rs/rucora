# AgentKit 🦀

[![Crates.io](https://img.shields.io/crates/v/agentkit.svg)](https://crates.io/crates/agentkit)
[![Documentation](https://docs.rs/agentkit/badge.svg)](https://docs.rs/agentkit)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build Status](https://github.com/agentkit-rs/agentkit/workflows/CI/badge.svg)](https://github.com/agentkit-rs/agentkit/actions)

**用 Rust 构建生产级 LLM 应用**

AgentKit 是一个高性能、类型安全的 LLM 应用开发框架，提供完整的工具链帮助您快速构建智能 Agent 应用。

## 🌟 特性

- ⚡ **极速性能** - Rust 原生，零成本抽象
- 🔒 **类型安全** - 编译时错误检查，运行时更可靠
- 💰 **成本监控** - 内置 Token 计数和成本管理
- 🧰 **丰富工具** - 12+ 内置工具，轻松扩展
- 🔌 **灵活集成** - 支持 10+ LLM Provider（OpenAI、Anthropic、Gemini 等）
- 📊 **可观测性** - 完整的日志、指标、追踪支持
- 🧠 **Agent 架构** - 思考与执行分离，支持自定义 Agent

## 🚀 快速开始

### 安装

```bash
# 创建新项目
cargo new my-agent
cd my-agent
```

### 添加依赖

```toml
[dependencies]
agentkit = { version = "0.1", features = ["runtime"] }
tokio = { version = "1", features = ["full"] }
serde_json = "1"
anyhow = "1"
```

### 可选 Features

| Feature | 说明 |
|---------|------|
| `runtime` | 运行时支持（默认启用） |
| `mcp` | MCP 协议支持 |
| `a2a` | A2A 协议支持 |
| `skills` | Skills 技能系统（默认启用） |
| `rhai-skills` | Rhai 脚本技能支持 |

### 第一个 Agent

```rust
use agentkit::provider::OpenAiProvider;
use agentkit::runtime::{DefaultRuntime, ToolRegistry};
use agentkit::prelude::AgentInput;
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

### 运行

```bash
export OPENAI_API_KEY=sk-your-key
cargo run
```

## 🔌 支持的 Provider

AgentKit 支持 10+ 种主流 LLM Provider：

| Provider | 说明 | 环境变量 |
|----------|------|----------|
| **OpenAI** | GPT-4、GPT-3.5 | `OPENAI_API_KEY` |
| **Anthropic** | Claude 3.5/3 | `ANTHROPIC_API_KEY` |
| **Google Gemini** | Gemini 1.5 Pro/Flash | `GOOGLE_API_KEY` |
| **Azure OpenAI** | 企业级 GPT 部署 | `AZURE_OPENAI_API_KEY` |
| **OpenRouter** | 70+ 模型聚合 | `OPENROUTER_API_KEY` |
| **DeepSeek** | DeepSeek-V3/R1 | `DEEPSEEK_API_KEY` |
| **Moonshot** | 月之暗面 Kimi | `MOONSHOT_API_KEY` |
| **Ollama** | 本地模型 | `OLLAMA_BASE_URL` |

```rust
// 使用不同的 Provider
use agentkit::provider::*;

// OpenAI
let provider = OpenAiProvider::from_env()?;

// Anthropic Claude
let provider = AnthropicProvider::from_env()?
    .with_default_model("claude-3-5-sonnet-20241022");

// Google Gemini
let provider = GeminiProvider::from_env()?
    .with_default_model("gemini-1.5-pro");

// OpenRouter（支持多种模型）
let provider = OpenRouterProvider::from_env()?
    .with_default_model("anthropic/claude-3-5-sonnet");
```

## 🧠 Agent 架构

AgentKit 采用**思考与执行分离**的架构：

- **Agent（智能体）**: 负责思考、决策、规划（大脑）
- **Runtime（运行时）**: 负责执行、调用、编排（身体）

```rust
use agentkit::agent::DefaultAgent;

// Agent 独立运行（简单对话）
let agent = DefaultAgent::builder()
    .provider(provider)
    .build();
let output = agent.run("你好").await?;

// Agent + Runtime（支持工具调用）
let runtime = DefaultRuntime::new(provider, tools);
let output = runtime.run_with_agent(&agent, "帮我查询天气").await?;
```

详细说明请查看 [Agent 和 Runtime 关系](docs/agent_runtime_relationship.md)

## 📚 文档

| 文档类型 | 说明 | 路径 |
|----------|------|------|
| 📘 用户指南 | 完整使用文档 | `docs/user_guide.md` |
| 🏃 快速入门 | 10 分钟上手教程 | `docs/quick_start.md` |
| 🍳 示例集合 | 实用代码示例 | `docs/cookbook.md` |
| ❓ 常见问题 | 问题解答 | `docs/faq.md` |
| 📖 API 参考 | 完整 API 文档 | [docs.rs](https://docs.rs/agentkit) |
| 🔗 快速参考 | 常用代码片段 | `docs/QUICK_REFERENCE.md` |

## 🧰 核心功能

### 对话管理

自动维护多轮对话历史：

```rust
use agentkit::conversation::ConversationManager;

let mut conv = ConversationManager::new()
    .with_max_messages(20);

conv.add_user_message("你好");
conv.add_assistant_message("你好！");

// 获取历史
let messages = conv.get_messages();
```

### 工具系统

使用内置工具或创建自定义工具：

```rust
use agentkit::tools::{FileReadTool, HttpRequestTool};

let tools = ToolRegistry::new()
    .register(FileReadTool::new())
    .register(HttpRequestTool::new());
```

### 成本管理

精确追踪 API 使用量和成本：

```rust
use agentkit::cost::{TokenCounter, CostTracker};

// Token 计数
let counter = TokenCounter::new("gpt-4");
let tokens = counter.count_text("Hello");

// 成本追踪
let tracker = CostTracker::new()
    .with_budget_limit(10.0);

tracker.record_usage("gpt-4", 100, 50, 0.0045).await;
```

### 中间件系统

灵活的请求/响应拦截：

```rust
use agentkit::middleware::{
    MiddlewareChain,
    LoggingMiddleware,
    RateLimitMiddleware,
};

let chain = MiddlewareChain::new()
    .with(LoggingMiddleware::new())
    .with(RateLimitMiddleware::new(100));
```

## 🏗️ 项目结构

```
agentkit/
├── agentkit-core       # 核心抽象层（traits/types）
├── agentkit            # 主库（实现聚合，包含所有功能）
│   └── src/
│       ├── mcp/        # MCP 协议支持
│       ├── a2a/        # A2A 协议支持
│       └── skills/     # 技能系统
├── examples/           # 示例代码
│   ├── agentkit-examples-complete
│   └── agentkit-examples-deep-dive
└── docs/               # 文档
```

### Workspace 成员

- `agentkit-core` - 核心抽象层
- `agentkit` - 主库（包含所有功能）
- `examples/*` - 示例代码

## 🔧 系统要求

- Rust 1.70+
- Tokio 运行时
- OpenAI API Key 或 Ollama 服务

## 📊 性能对比

| 框架 | 语言 | 内存占用 | 启动时间 | 类型安全 |
|------|------|---------|---------|---------|
| **AgentKit** | Rust | <10MB | <100ms | ✅ |
| LangChain | Python | ~200MB | ~2s | ❌ |
| LlamaIndex | Python | ~150MB | ~1.5s | ❌ |

## 🤝 贡献

欢迎贡献代码、文档或反馈问题！

1. Fork 项目
2. 创建特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 开启 Pull Request

## 📄 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 🙏 致谢

感谢所有贡献者和用户！

---

**开始构建您的智能 Agent 应用吧！** 🚀
