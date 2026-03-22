# AgentKit 项目上下文

## 项目概述

**AgentKit** 是一个用 Rust 编写的高性能、类型安全的 LLM 应用开发框架，用于构建生产级智能 Agent 应用。

### 核心特性

- ⚡ **极速性能** - Rust 原生，零成本抽象
- 🔒 **类型安全** - 编译时错误检查，运行时更可靠
- 💰 **成本监控** - 内置 Token 计数和成本管理
- 🧰 **丰富工具** - 12+ 内置工具（Shell/File/HTTP/Git/Memory 等）
- 🔌 **灵活集成** - 支持 10+ LLM Provider（OpenAI、Anthropic、Gemini、Ollama 等）
- 📊 **可观测性** - 完整的日志、指标、追踪支持
- 🧠 **Agent 架构** - 思考与执行分离，支持自定义 Agent

### 项目架构

```
agentkit/
├── agentkit-core       # 核心抽象层（traits/types，无具体实现）
├── agentkit            # 主库（实现聚合，Provider/Tools/Skills 等）
├── agentkit-runtime    # 运行时实现（编排层）
├── agentkit-cli        # 命令行工具
├── agentkit-server     # HTTP 服务器
├── agentkit-mcp        # MCP 协议支持（可选 feature）
├── agentkit-a2a        # A2A 协议支持（可选 feature）
├── agentkit-skills     # 技能系统（Rhai 脚本等）
└── examples/           # 示例代码
    ├── agentkit-examples-complete    # 完整功能示例
    └── agentkit-examples-deep-dive   # 深入研究示例
```

### Workspace 结构

根目录 `Cargo.toml` 定义了 workspace 和共享版本号：

```toml
[workspace]
resolver = "3"
members = [
    "agentkit", "agentkit-cli", "agentkit-core",
    "agentkit-runtime", "agentkit-server",
    "agentkit-mcp", "agentkit-a2a", "agentkit-skills",
    "examples/agentkit-examples-complete",
    "examples/agentkit-examples-deep-dive"
]

[workspace.package]
version = "0.1.0"
```

所有子 crate 使用 `version.workspace = true` 继承版本号。

## 构建和运行

### 系统要求

- Rust 1.70+
- Tokio 运行时
- API Key（OpenAI 或其他 Provider）

### 构建项目

```bash
# 构建整个 workspace
cargo build --workspace

# 构建特定 crate
cargo build -p agentkit
cargo build -p agentkit-runtime

# Release 构建
cargo build --release --workspace

# 检查编译（快速）
cargo check --workspace
```

### 运行示例

```bash
# 设置环境变量
export OPENAI_API_KEY=sk-your-key

# 运行完整示例
cargo run -p agentkit-examples-complete

# 运行深入研究示例
cargo run -p agentkit-examples-deep-dive

# 运行 agent 示例
cargo run -p agentkit --example agent_basic_usage
```

### 运行测试

```bash
# 运行所有测试
cargo test --workspace

# 运行特定 crate 测试
cargo test -p agentkit-core
cargo test -p agentkit

# 运行测试（显示输出）
cargo test --workspace -- --nocapture
```

### 运行基准测试

```bash
# 运行基准测试（需要 nightly）
cargo bench -p agentkit
```

### 格式化代码

```bash
# 格式化所有代码
cargo fmt --workspace

# 检查格式
cargo fmt --workspace -- --check
```

### Clippy 检查

```bash
# 运行 Clippy
cargo clippy --workspace -- -D warnings

# 自动修复
cargo clippy --workspace --fix
```

## 核心模块说明

### agentkit-core

核心抽象层，只包含 trait 和类型定义：

- **Agent trait**: 智能体抽象接口
- **Runtime trait**: 运行时编排抽象
- **LlmProvider trait**: LLM Provider 抽象
- **Tool trait**: 工具抽象
- **Skill trait**: 技能抽象
- **类型定义**: AgentInput, AgentOutput, ChatRequest, ChatResponse 等
- **错误类型**: ProviderError, ToolError, AgentError 等
- **事件模型**: ChannelEvent, TokenDeltaEvent 等

### agentkit

主库，聚合所有实现：

- **agent 模块**: `Agent` 和 `AgentBuilder`（增强的 Agent 实现）
- **provider 模块**: OpenAiProvider, AnthropicProvider, GeminiProvider 等
- **tools 模块**: ShellTool, FileReadTool, HttpRequestTool, GitTool 等
- **skills 模块**: RhaiSkill, CommandSkill, EchoSkill 等
- **memory 模块**: InMemoryMemory, FileMemory
- **retrieval 模块**: ChromaVectorStore
- **rag 模块**: RAG 管线（chunking, indexing, retrieval）
- **conversation 模块**: 对话历史管理
- **config 模块**: 统一配置系统
- **middleware 模块**: 中间件系统
- **cost 模块**: Token 计数和成本管理
- **prompt 模块**: Prompt 模板系统

### agentkit-runtime

运行时实现：

- **DefaultRuntime**: 默认运行时实现
- **ToolRegistry**: 工具注册表
- **编排逻辑**: Agent 决策执行循环

## 开发惯例

### 代码风格

- 使用 Rust 官方格式化工具 `rustfmt`
- 遵循 Rust API 命名约定
- 公共 API 必须有文档注释（`///`）
- 模块内部使用 `//!` 文档注释

### 错误处理

- 使用 `thiserror` 定义错误类型
- 所有错误类型实现 `DiagnosticError` trait
- 提供结构化诊断信息（错误类型、消息、是否可重试）

### 测试实践

- 单元测试放在对应模块的 `#[cfg(test)] mod tests`
- 集成测试放在 `tests/` 目录
- 示例代码放在 `examples/` 目录
- 使用 `anyhow::Result` 简化测试错误处理

### Feature 标志

```toml
[features]
default = ["skills"]
mcp = ["dep:agentkit-mcp"]
a2a = ["dep:agentkit-a2a"]
skills = ["dep:agentkit-skills"]
rhai-skills = ["agentkit-skills?/rhai-skills"]
```

### 依赖管理

- 所有子 crate 使用 `version.workspace = true`
- 路径依赖使用 `{ path = "../xxx" }`
- 可选依赖使用 `optional = true`

## 使用示例

### 基本用法

```rust
use agentkit::provider::OpenAiProvider;
use agentkit_runtime::{DefaultRuntime, ToolRegistry};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let provider = OpenAiProvider::from_env()?;
    
    let runtime = DefaultRuntime::new(
        Arc::new(provider),
        ToolRegistry::new()
    ).with_system_prompt("你是有用的助手");
    
    let input = agentkit_core::agent::AgentInput::new("你好");
    let output = runtime.run(input).await?;
    
    println!("{}", output.text().unwrap_or("无回复"));
    Ok(())
}
```

### 使用 Agent

```rust
use agentkit::agent::Agent;

let agent = Agent::builder()
    .provider(provider)
    .system_prompt("你是有用的助手")
    .tool(agentkit::tools::EchoTool)
    .max_steps(10)
    .build();

let output = agent.run("你好").await?;
```

### 自定义 Agent

```rust
use agentkit::core::agent::Agent as CoreAgent;
use agentkit::core::agent::{AgentContext, AgentDecision};

struct MyAgent;

#[async_trait]
impl CoreAgent for MyAgent {
    async fn think(&self, context: &AgentContext) -> AgentDecision {
        // 自定义决策逻辑
        AgentDecision::Chat { request: context.default_chat_request() }
    }
    
    fn name(&self) -> &str { "my_agent" }
}
```

### 使用 Prelude

```rust
use agentkit::prelude::*;

// 可访问：
// - Runtime trait
// - Agent, AgentBuilder
// - AgentInput, AgentOutput
// - ChannelEvent, TokenDeltaEvent
// - ProviderError, ToolError
// - LlmProvider trait
// - Tool trait
```

## 文档资源

| 文档类型 | 路径 |
|----------|------|
| 用户指南 | `docs/user_guide.md` |
| 快速入门 | `docs/quick_start.md` |
| 示例集合 | `docs/cookbook.md` |
| 常见问题 | `docs/faq.md` |
| Agent 与 Runtime 关系 | `docs/agent_runtime_relationship.md` |
| 设计文档 | `docs/design.md` |
| 快速参考 | `QUICK_REFERENCE.md` |

## 支持的 Provider

| Provider | 环境变量 |
|----------|----------|
| OpenAI | `OPENAI_API_KEY` |
| Anthropic | `ANTHROPIC_API_KEY` |
| Google Gemini | `GOOGLE_API_KEY` |
| Azure OpenAI | `AZURE_OPENAI_API_KEY` |
| OpenRouter | `OPENROUTER_API_KEY` |
| DeepSeek | `DEEPSEEK_API_KEY` |
| Moonshot | `MOONSHOT_API_KEY` |
| Ollama | `OLLAMA_BASE_URL` |

## 常见问题

### 命名冲突处理

由于 `Agent` 既是 trait 名又是结构体名：

- `agentkit::agent::Agent` - 增强的 Agent 结构体（主要使用）
- `agentkit::core::agent::Agent` 或 `CoreAgent` - Agent trait（用于实现自定义 Agent）

### 编译错误

如果遇到编译错误，尝试：

```bash
# 清理并重新构建
cargo clean
cargo build --workspace

# 更新依赖
cargo update
```

### 环境变量

确保设置必要的 API Key：

```bash
export OPENAI_API_KEY=sk-xxx
export ANTHROPIC_API_KEY=sk-ant-xxx
```
