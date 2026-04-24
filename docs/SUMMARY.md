# AgentKit 文档目录

> AgentKit 是一个用 Rust 编写的高性能、类型安全的 LLM 应用开发框架

---

## 📚 新手入门

| 文档 | 说明 |
|------|------|
| [快速开始](quick_start.md) | 5 分钟上手 AgentKit |
| [示例索引](examples.md) | 20+ 完整示例代码 |

---

## 🏗️ 核心模块

| 文档 | 说明 |
|------|------|
| [核心抽象](core_abstractions.md) | agentkit-core 全貌：trait、类型、错误模型、事件系统 |
| [Agent 类型](agent_types.md) | 5 种 Agent 详解：SimpleAgent、ChatAgent、ToolAgent、ReActAgent、ReflectAgent |
| [Provider 指南](providers_guide.md) | 8 种 LLM Provider 及弹性 Provider（ResilientProvider） |
| [工具参考](tools_guide.md) | 17+ 内置工具分类说明，自定义 Tool 实现指南 |

---

## 🔧 高级功能

| 文档 | 说明 |
|------|------|
| [技能系统](skills_guide.md) | Skills 加载、执行、自动集成（YAML/TOML/JSON 配置） |
| [记忆系统](memory_guide.md) | Memory 抽象、InMemoryMemory、FileMemory、AdvancedMemory |
| [向量检索](embedding_retrieval.md) | Embedding Provider + VectorStore + RAG 管线 |
| [RAG 管线](rag_guide.md) | 文本分块、索引、检索、引用生成 |
| [Prompt 模板](prompt_guide.md) | 模板语法、变量、条件渲染、循环 |
| [上下文压缩](context_compression.md) | 分层压缩引擎、TokenCounter、ContextManager |

---

## 🔌 协议集成

| 文档 | 说明 |
|------|------|
| [MCP 协议](mcp_guide.md) | Model Context Protocol 集成（Stdio/HTTP 传输） |
| [A2A 协议](a2a_guide.md) | Agent-to-Agent 协议（客户端/服务器） |

---

## 🛡️ 可靠性

| 文档 | 说明 |
|------|------|
| [错误处理](error_handling.md) | 统一错误类型、错误分类器、注入防护 |

---

## 💬 对话管理

| 文档 | 说明 |
|------|------|
| [对话历史](agent_auto_conversation.md) | 内置对话历史管理（with_conversation / max_history_messages） |

---

## 📋 配置规范

| 文档 | 说明 |
|------|------|
| [Skill YAML 规范](skill_yaml_spec.md) | 技能配置文件完整格式说明 |
| [Skill YAML 示例](skill_yaml_examples.md) | 实际 Skill 配置示例 |

---

## 💡 最佳实践

| 文档 | 说明 |
|------|------|
| [错误处理](error_handling.md) | 统一错误类型、错误分类器、注入防护、重试机制 |
| [中间件](middleware_guide.md) | Middleware 系统、4 种内置中间件 |

---

## 🗂️ 项目结构

```
agentkit/
├── agentkit-core           # 核心抽象层（traits + types）
├── agentkit                # 主 crate（统一入口，重新导出所有子模块）
├── agentkit-providers      # LLM Provider 实现
├── agentkit-tools          # 内置工具集合
├── agentkit-skills         # 技能系统
├── agentkit-embed          # Embedding Provider
├── agentkit-retrieval      # 向量存储与检索
├── agentkit-mcp            # MCP 协议集成
├── agentkit-a2a            # A2A 协议集成
└── examples/               # 示例项目
    ├── agentkit/examples/  # 20+ 功能示例
    ├── agentkit-skills-example/
    ├── agentkit-deep-research/
    ├── a2a-client/
    └── a2a-server/
```
