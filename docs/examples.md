# AgentKit 示例索引

> 完整的示例代码列表和说明

## 快速开始

### Hello World - 最简单的 Agent

**文件**: [`agentkit/examples/01_hello_world.rs`](../agentkit/examples/01_hello_world.rs)

**说明**: 展示如何用最少的代码创建一个 Agent 应用。

**运行**:
```bash
export OPENAI_API_KEY=sk-your-key
cargo run --example 01_hello_world
```

**学习点**:
- 创建 Provider
- 创建 Agent
- 运行对话

**代码量**: ~80 行

---

## 对话示例

### 基础对话 - 支持多轮对话

**文件**: [`agentkit/examples/02_basic_chat.rs`](../agentkit/examples/02_basic_chat.rs)

**说明**: 展示如何创建支持多轮对话的 Agent，记住对话历史。

**运行**:
```bash
export OPENAI_API_KEY=sk-your-key
cargo run --example 02_basic_chat
```

**学习点**:
- 启用对话历史
- 设置最大消息数
- 多轮对话测试

**代码量**: ~70 行

### 带工具对话 - 调用工具

**文件**: [`agentkit/examples/03_chat_with_tools.rs`](../agentkit/examples/03_chat_with_tools.rs)

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
| 01_hello_world | 使用 OpenAI Provider | 01_hello_world.rs |
| 10_custom_provider | 自定义 Provider 实现 | 10_custom_provider.rs |
| 11_resilient_provider | 弹性 Provider（重试/退避） | 11_resilient_provider.rs |

### Agent 相关

| 示例 | 说明 | 文件 |
|------|------|------|
| 01_hello_world | 基础 Agent | 01_hello_world.rs |
| 02_basic_chat | 多轮对话 Agent | 02_basic_chat.rs |
| 03_chat_with_tools | 带工具的 Agent | 03_chat_with_tools.rs |
| 04_extractor | 信息提取 Agent | 04_extractor.rs |
| 05_conversation | 对话历史管理 | 05_conversation.rs |
| 15_react_agent | ReAct Agent | 15_react_agent.rs |
| 16_reflect_agent | Reflect Agent | 16_reflect_agent.rs |
| 17_supervisor_agent | Supervisor Agent | 17_supervisor_agent.rs |

### Tools 相关

| 示例 | 说明 | 文件 |
|------|------|------|
| 03_chat_with_tools | ShellTool, DatetimeTool | 03_chat_with_tools.rs |
| 19_code_assistant | 代码助手 | 19_code_assistant.rs |

### Skills 相关

| 示例 | 说明 | 文件 |
|------|------|------|
| agentkit-skills-example | 完整 Skills 集成 | examples/agentkit-skills-example/ |

### 高级功能

| 示例 | 说明 | 文件 |
|------|------|------|
| 06_memory | 记忆系统 | 06_memory.rs |
| 07_rag | RAG 检索增强生成 | 07_rag.rs |
| 08_middleware | 中间件 | 08_middleware.rs |
| 09_prompt | Prompt 模板 | 09_prompt.rs |
| 12_mcp | MCP 协议 | 12_mcp.rs |
| 13_task_decomposition | 任务分解 | 13_task_decomposition.rs |
| 18_research_assistant | 研究助手 | 18_research_assistant.rs |
| 20_custom_agent_with_middleware | 自定义 Agent + 中间件 | 20_custom_agent_with_middleware.rs |
| 21_unified_conversation | 统一对话模式 | 21_unified_conversation.rs |
| 24_context_compression | 上下文压缩 | 24_context_compression.rs |

---

## 按难度分类

### 入门级 ⭐

适合第一次使用 AgentKit 的用户。

- [01_hello_world.rs](../agentkit/examples/01_hello_world.rs) - Hello World
- [02_basic_chat.rs](../agentkit/examples/02_basic_chat.rs) - 基础对话

### 进阶级 ⭐⭐

适合已经了解基础用法的用户。

- [03_chat_with_tools.rs](../agentkit/examples/03_chat_with_tools.rs) - 带工具对话
- [04_extractor.rs](../agentkit/examples/04_extractor.rs) - 信息提取
- [05_conversation.rs](../agentkit/examples/05_conversation.rs) - 对话管理
- [agentkit-skills-example](../examples/agentkit-skills-example/) - Skills 集成

### 高级 ⭐⭐⭐

适合需要实现复杂功能的用户。

- [15_react_agent.rs](../agentkit/examples/15_react_agent.rs) - ReAct Agent
- [16_reflect_agent.rs](../agentkit/examples/16_reflect_agent.rs) - Reflect Agent
- [17_supervisor_agent.rs](../agentkit/examples/17_supervisor_agent.rs) - Supervisor Agent
- [07_rag.rs](../agentkit/examples/07_rag.rs) - RAG
- [13_task_decomposition.rs](../agentkit/examples/13_task_decomposition.rs) - 任务分解

---

## 运行所有示例

```bash
# Hello World
cargo run --example 01_hello_world

# 基础对话
cargo run --example 02_basic_chat

# 带工具对话
cargo run --example 03_chat_with_tools

# 对话历史管理
cargo run --example 05_conversation

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

- [Skill 配置规范](skill_yaml_spec.md)
- [Skill 配置示例](skill_yaml_examples.md)
- [Agent 自动对话](agent_auto_conversation.md)
