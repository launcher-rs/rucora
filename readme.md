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
- 🔌 **灵活集成** - 支持 OpenAI、Ollama 等主流服务
- 📊 **可观测性** - 完整的日志、指标、追踪支持

## 🚀 快速开始

### 安装

```bash
# 创建新项目
cargo new my-agent
cd my-agent

# 添加依赖
cargo add agentkit agentkit-runtime tokio serde_json anyhow
```

### 第一个 Agent

```rust
use agentkit::provider::OpenAiProvider;
use agentkit_runtime::{DefaultRuntime, ToolRegistry};
use agentkit_core::agent::types::AgentInput;
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
    let input = AgentInput::from("用一句话介绍 Rust");
    let output = runtime.run(input).await?;
    
    println!("{}", output.message.content);
    Ok(())
}
```

### 运行

```bash
export OPENAI_API_KEY=sk-your-key
cargo run
```

## 📚 文档

| 文档类型 | 说明 | 链接 |
|----------|------|------|
| 📘 用户指南 | 完整使用文档 | [查看](docs/user_guide.md) |
| 🏃 快速入门 | 10 分钟上手教程 | [查看](docs/quick_start.md) |
| 🍳 示例集合 | 实用代码示例 | [查看](docs/cookbook.md) |
| ❓ 常见问题 | 问题解答 | [查看](docs/faq.md) |
| 📖 API 参考 | 完整 API 文档 | [查看](https://docs.rs/agentkit) |

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
├── agentkit            # 主库（实现聚合）
├── agentkit-runtime    # 运行时实现
├── agentkit-cli        # 命令行工具
├── agentkit-server     # HTTP 服务器
├── agentkit-mcp        # MCP 协议支持
└── agentkit-a2a        # A2A 协议支持
```

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
