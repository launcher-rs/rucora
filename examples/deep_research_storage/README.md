# 深度研究存储示例 (deep-research-storage)

## 原理

存储示例展示了如何使用 `ResearchLibrary` trait 来存储和检索历史研究成果。通过将研究结果持久化，可以实现知识的积累和复用。

## 核心组件

- **InMemoryResearchLibrary**: 内存实现的研究库
- **ResearchLibrary trait**: 研究库接口（save, search, get, list, delete）
- **ResearchReport**: 研究报告结构

## 功能

1. **保存研究**: 将研究报告保存到库中
2. **搜索历史**: 通过关键词搜索历史研究
3. **查看详情**: 获取特定研究报告
4. **列表管理**: 查看所有已存储的研究

## ⚠️ 重要说明

> **当前版本中，`DefaultResearchEngine` + `StandardStrategy` 是一个模拟/占位实现，不会调用任何 LLM API 或外部工具。**

运行此示例时，你只会看到存储和检索的操作日志，不会看到任何 API 调用或工具调用信息，这是因为：

1. **研究策略是模拟的**：`StandardStrategy::search()` 不会调用 LLM，而是直接生成硬编码的模拟数据（dummy `InfoPiece`），源码中已标注 `"模拟搜索过程（实际实现需要调用搜索工具）"`。
2. **评分系统会在第一轮立即判定完成**：由于模拟数据的 `relevance_score` 默认为 `1.0`，综合评分达到 `0.94`，远超阈值 `0.6`，因此 `should_continue()` 返回 `false`，只执行一轮就退出。
3. **策略内部的调试日志为 `debug` 级别**，而示例配置的日志级别为 `INFO`，因此不会输出。

此示例的**主要目的是演示 `ResearchLibrary` 的存储和检索功能**（save / search / list），而非真正的深度研究能力。

### 如果需要真正的深度研究

请使用 [`rucora-deep-research`](../rucora-deep-research/) 示例，它使用多阶段 Agent 架构（`DefaultExecution` + `ToolRegistry`），真正集成了 Tavily 搜索、网页抓取、LLM 调用等能力。

## 意义

1. **知识积累**: 避免重复研究，积累领域知识
2. **历史追溯**: 方便回顾和引用之前的研究成果
3. **构建知识库**: 长期使用可以构建个人/团队知识库

## 运行

```bash
cargo run --example deep-research-storage
```

## 环境变量

需要设置以下环境变量：
- `OPENAI_API_KEY`: OpenAI API 密钥

## 扩展

除 `InMemoryResearchLibrary` 外，rucora 还提供 `FileResearchLibrary` 用于持久化存储到文件系统。可以实现自定义的 `ResearchLibrary` 来对接数据库或其他存储系统。