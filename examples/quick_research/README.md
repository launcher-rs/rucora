# 快速研究示例 (quick_research)

## 原理

快速研究示例展示了如何使用 rucora 框架进行轻量级的研究任务。该示例使用 `ToolAgent` 配合搜索工具，在 30 秒到 3 分钟内获取主题的简要信息。

## 核心组件

- **ToolAgent**: 带工具调用能力的智能体
- **TavilyTool**: 网络搜索工具，用于快速获取信息
- **DatetimeTool**: 日期时间工具

## 意义

1. **快速入门**: 展示 rucora 最简单的使用方式
2. **适合场景**: 简单事实查询、新闻摘要、基础信息收集
3. **性能优化**: 单轮或少轮交互，快速返回结果

## 运行

```bash
cargo run --example quick_research
```

## 环境变量

需要设置以下环境变量：
- `OPENAI_API_KEY`: OpenAI API 密钥
- `TAVILY_API_KEYS`: Tavily 搜索 API 密钥（可选，用于网络搜索）

## 扩展

可以替换为其他搜索工具（如 Serpapi、ContentSearchTool）来获取不同来源的信息。