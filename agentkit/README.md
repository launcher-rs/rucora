# agentkit（聚合 crate）

`agentkit` 是“开箱即用”的聚合入口：在 `agentkit-core` 的 trait 与类型之上，提供常用实现与便捷模块。

## 你会在这里得到什么

- **Provider 实现**：OpenAI-compatible / Ollama / Router
- **Tools 实现**：HTTP、浏览器抓取、命令执行等
- **Skills**：Rhai/Command skill loader，支持将 skills 适配为 tools
- **Retrieval/Embedding/Memory**：面向 RAG 的基础实现（包含 `InMemoryVectorStore`）
- **统一配置**：`agentkit::config::AgentkitConfig`

## 推荐入口

- 设计文档：`docs/design.md`
- 示例索引：`docs/examples.md`

## 快速开始（流式 + trace）

参考 `agentkit/examples/config_stream_trace_demo.rs`。

## 常用模块

- `agentkit::provider::*`
- `agentkit::tools::*`
- `agentkit::skills::*`
- `agentkit::retrieval::*`
- `agentkit::embed::*`
- `agentkit::memory::*`
- `agentkit::config::*`
