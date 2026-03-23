# AgentKit 示例

本目录包含 AgentKit 的使用示例，展示各个模块的功能和综合应用。

## 运行示例

大多数示例需要设置 API Key 环境变量：

```bash
# 使用 OpenAI
export OPENAI_API_KEY=sk-xxx

# 或使用 Anthropic
export ANTHROPIC_API_KEY=sk-ant-xxx

# 或使用 Google Gemini
export GOOGLE_API_KEY=xxx

# 或使用 OpenRouter
export OPENROUTER_API_KEY=xxx
```

## 基础示例

### ✅ 01_basic_chat - 基础聊天

最简单的对话示例，适合快速上手。

```bash
cargo run --example 01_basic_chat -p agentkit
```

**功能**：
- ✅ 自动检测可用的 Provider
- ✅ 交互式对话
- ✅ 支持多轮对话

### ✅ 02_provider - Provider 使用

展示如何使用不同的 LLM Provider。

```bash
cargo run --example 02_provider -p agentkit
```

**支持的 Provider**：
- OpenAI
- Anthropic
- Google Gemini
- OpenRouter

### ✅ 03_tools - Tool 使用

展示如何使用各种内置工具。

```bash
cargo run --example 03_tools -p agentkit
```

**工具**：
- EchoTool - 回显工具
- FileReadTool - 文件读取
- GitTool - Git 操作
- ShellTool - Shell 命令

### ✅ 04_memory - Memory 使用

展示如何使用记忆系统存储和检索信息。

```bash
cargo run --example 04_memory -p agentkit
```

**功能**：
- InMemoryMemory - 进程内记忆
- FileMemory - 文件记忆
- 记忆查询

### ✅ 05_conversation - Conversation 使用

展示如何管理对话历史。

```bash
cargo run --example 05_conversation -p agentkit
```

**功能**：
- 多轮对话管理
- 消息窗口限制
- 系统提示词

### ✅ 06_cost - Cost 使用

展示如何使用 Token 计数和成本管理。

```bash
cargo run --example 06_cost -p agentkit
```

**功能**：
- TokenCounter - Token 计数
- 成本追踪

## Provider 扩展示例

### ✅ 07_router_provider - Router Provider

展示如何在多个 Provider 之间路由请求。

```bash
export OPENAI_API_KEY=sk-xxx
export ANTHROPIC_API_KEY=sk-ant-xxx
cargo run --example 07_router_provider -p agentkit
```

**功能**：
- ✅ 自动路由（默认策略）
- ✅ 指定 Provider
- ✅ 多 Provider 注册

### ✅ 08_resilient_provider - Resilient Provider

展示如何使用带重试机制的 Provider。

```bash
export OPENAI_API_KEY=sk-xxx
cargo run --example 08_resilient_provider -p agentkit
```

**功能**：
- ✅ 自动重试
- ✅ 指数退避延迟
- ✅ 可配置的重试策略

### ✅ 09_mcp - MCP 使用

展示如何连接 MCP 服务器并使用其提供的工具。

```bash
# 启用 mcp feature
cargo run --example 09_mcp -p agentkit --features mcp
```

**功能**：
- ✅ 连接 MCP 服务器
- ✅ 获取 MCP 工具列表
- ✅ 将 MCP 工具注册到 ToolRegistry
- ✅ 与 Ollama Provider 结合使用

**MCP 服务器配置**：
- URL: `http://127.0.0.1:8000/mcp`
- Token: Bearer Token 认证

## 综合示例

### ✅ 10_research_assistant - 智能研究助手

结合 Provider、Tools、Memory、Conversation 等多个模块，创建一个完整的智能研究助手。

```bash
export OPENAI_API_KEY=sk-xxx
cargo run --example 10_research_assistant -p agentkit
```

**集成功能**：
- ✅ Provider（OpenAI）
- ✅ Tools（Echo、FileRead、Git、Shell）
- ✅ Memory（信息存储）
- ✅ Conversation（对话管理）

## 示例分类

| 分类 | 示例 | 说明 |
|------|------|------|
| **Provider** | 02_provider, 07_router_provider, 08_resilient_provider | LLM Provider 使用 |
| **Tools** | 03_tools, 09_mcp | 工具使用 |
| **Memory** | 04_memory | 记忆系统 |
| **Conversation** | 05_conversation | 对话管理 |
| **Cost** | 06_cost | 成本管理 |
| **综合** | 01_basic_chat, 10_research_assistant | 完整应用 |

## 故障排除

### 未找到 API Key

```
❌ 未找到 API Key

请设置环境变量：
  export OPENAI_API_KEY=sk-xxx
```

**解决方案**：设置相应的环境变量。

### 编译错误

确保启用了必要的 features：

```bash
# Runtime 支持
cargo run --example 01_basic_chat -p agentkit --features runtime

# MCP 支持
cargo run --example 09_mcp -p agentkit --features mcp
```

## 更多资源

- [用户指南](../../docs/user_guide.md)
- [快速入门](../../docs/quick_start.md)
- [API 文档](https://docs.rs/agentkit)
- [GitHub](https://github.com/agentkit-rs/agentkit)
