# rucora 示例索引

> 完整的示例代码列表和说明

所有示例位于 [`rucora/examples/`](../../rucora/examples/) 目录，可通过 `cargo run --example <名称>` 运行。

## 快速开始

### Hello World - 最简单的 Agent

**文件**: [`rucora/examples/01_hello_world.rs`](../../rucora/examples/01_hello_world.rs)

**说明**: 展示如何用最少的代码创建一个对话 Agent。

**运行**:
```bash
export OPENAI_API_KEY=sk-your-key
cargo run --example 01_hello_world
```

**学习点**:
- 创建 Provider
- 创建 Agent
- 运行对话

---

## 对话示例

### 基础对话 - 支持多轮对话

**文件**: [`rucora/examples/02_basic_chat.rs`](../../rucora/examples/02_basic_chat.rs)

**说明**: 展示如何使用 ChatAgent 进行交互式多轮对话。

**运行**:
```bash
export OPENAI_API_KEY=sk-your-key
cargo run --example 02_basic_chat
```

**学习点**:
- 对话历史
- 多轮对话测试

### 带工具对话 - 调用工具

**文件**: [`rucora/examples/03_chat_with_tools.rs`](../../rucora/examples/03_chat_with_tools.rs)

**说明**: 展示如何创建支持工具调用的 Agent。

**运行**:
```bash
export OPENAI_API_KEY=sk-your-key
cargo run --example 03_chat_with_tools
```

**学习点**:
- 注册工具
- 工具调用
- 工具结果处理

---

## Skills 示例

### Agent + Skills 完整示例

**文件**: [`examples/rucora-skills-example/src/main.rs`](../../examples/rucora-skills-example/src/main.rs)

**说明**: 展示 Agent 如何自动调用 Skills 完成任务。

**运行**:
```bash
cd examples/rucora-skills-example
cargo run
```

**学习点**:
- Skills 加载
- Skills 转换为 Tools
- 构建系统提示词
- Agent 自动调用 Skills

---

## 全部示例列表

### 入门级 ⭐

| 示例 | 说明 | 文件 |
|------|------|------|
| 01_hello_world | Hello World | [`01_hello_world.rs`](../../rucora/examples/01_hello_world.rs) |
| 02_basic_chat | 基础对话 | [`02_basic_chat.rs`](../../rucora/examples/02_basic_chat.rs) |
| 03_chat_with_tools | 带工具的对话 | [`03_chat_with_tools.rs`](../../rucora/examples/03_chat_with_tools.rs) |

### 进阶级 ⭐⭐

| 示例 | 说明 | 文件 |
|------|------|------|
| 04_extractor | 提取器 | [`04_extractor.rs`](../../rucora/examples/04_extractor.rs) |
| 05_conversation | 对话管理 | [`05_conversation.rs`](../../rucora/examples/05_conversation.rs) |
| 06_memory | 记忆功能 | [`06_memory.rs`](../../rucora/examples/06_memory.rs) |
| 07_rag | RAG 检索增强 | [`07_rag.rs`](../../rucora/examples/07_rag.rs) |
| 08_middleware | 中间件 | [`08_middleware.rs`](../../rucora/examples/08_middleware.rs) |
| 09_prompt | 提示词管理 | [`09_prompt.rs`](../../rucora/examples/09_prompt.rs) |
| 10_custom_provider | 自定义 Provider | [`10_custom_provider.rs`](../../rucora/examples/10_custom_provider.rs) |
| 11_resilient_provider | 弹性 Provider | [`11_resilient_provider.rs`](../../rucora/examples/11_resilient_provider.rs) |
| 12_mcp | MCP 协议（需 `--all-features`） | [`12_mcp.rs`](../../rucora/examples/12_mcp.rs) |

### 高级 ⭐⭐⭐

| 示例 | 说明 | 文件 |
|------|------|------|
| 13_task_decomposition | 任务分解 | [`13_task_decomposition.rs`](../../rucora/examples/13_task_decomposition.rs) |
| 15_react_agent | ReAct Agent | [`15_react_agent.rs`](../../rucora/examples/15_react_agent.rs) |
| 16_reflect_agent | 反思 Agent | [`16_reflect_agent.rs`](../../rucora/examples/16_reflect_agent.rs) |
| 17_supervisor_agent | 主管 Agent | [`17_supervisor_agent.rs`](../../rucora/examples/17_supervisor_agent.rs) |
| 18_research_assistant | 研究助手 | [`18_research_assistant.rs`](../../rucora/examples/18_research_assistant.rs) |
| 19_code_assistant | 代码助手 | [`19_code_assistant.rs`](../../rucora/examples/19_code_assistant.rs) |
| 20_custom_agent_with_middleware | 自定义 Agent + 中间件 | [`20_custom_agent_with_middleware.rs`](../../rucora/examples/20_custom_agent_with_middleware.rs) |
| 21_unified_conversation | 统一对话 | [`21_unified_conversation.rs`](../../rucora/examples/21_unified_conversation.rs) |
| 22_summary_agent | 摘要 Agent | [`22_summary_agent.rs`](../../rucora/examples/22_summary_agent.rs) |
| 24_context_compression | 上下文压缩 | [`24_context_compression.rs`](../../rucora/examples/24_context_compression.rs) |
| 25_streaming_agent | 流式 Agent | [`25_streaming_agent.rs`](../../rucora/examples/25_streaming_agent.rs) |
| 26_macros | 宏 | [`26_macros.rs`](../../rucora/examples/26_macros.rs) |
| 27_concurrent_translate | 并发翻译 | [`27_concurrent_translate.rs`](../../rucora/examples/27_concurrent_translate.rs) |
| 28_concurrent_translate_with_retry | 并发翻译 + 重试 | [`28_concurrent_translate_with_retry.rs`](../../rucora/examples/28_concurrent_translate_with_retry.rs) |

### 独立示例应用

| 示例 | 说明 | 路径 |
|------|------|------|
| a2a-client | A2A 客户端 | [`examples/a2a-client/`](../../examples/a2a-client/) |
| a2a-server | A2A 服务端 | [`examples/a2a-server/`](../../examples/a2a-server/) |
| rucora-skills-example | Skills 集成 | [`examples/rucora-skills-example/`](../../examples/rucora-skills-example/) |
| rucora-deep-research | 深度研究 | [`examples/rucora-deep-research/`](../../examples/rucora-deep-research/) |

---

## 代码片段

### 创建 Provider

```rust
use rucora::provider::OpenAiProvider;

// 从环境变量加载
let provider = OpenAiProvider::from_env()?;

// 或直接指定
let provider = OpenAiProvider::new("sk-your-key")?;
```

### 创建 Agent

```rust
use rucora::agent::ToolAgent;

let agent = ToolAgent::builder(provider)
    .model("gpt-4o-mini")
    .system_prompt("你是有用的助手")
    .build();
```

### 运行对话

```rust
let output = agent.run("你好".into()).await?;
println!("{}", output.text().unwrap_or("无回复"));
```

### 启用对话历史

```rust
use rucora::agent::ChatAgent;

let agent = ChatAgent::builder(provider)
    .model("gpt-4o-mini")
    .system_prompt("你是有用的助手")
    .with_conversation(true)  // 启用对话历史
    .build();
```

### 注册工具

```rust
use rucora::agent::ToolAgent;
use rucora::tools::{DatetimeTool, EchoTool};

let agent = ToolAgent::builder(provider)
    .model("gpt-4o-mini")
    .system_prompt("你是有用的助手")
    .tool(DatetimeTool)
    .tool(EchoTool)
    .build();
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

use rucora::...;

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
