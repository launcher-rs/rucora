# 研究库示例 (rucora-deep-research-library)

## 原理

研究库示例展示了如何使用 `ResearchLibrary` trait 来存储和检索历史研究成果。通过将研究结果持久化，可以实现知识的积累和复用。

## 核心组件

- **InMemoryResearchLibrary**: 内存实现的研究库
- **ResearchLibrary trait**: 研究库接口（save, search, get, list, delete）
- **ResearchReport**: 研究报告结构

## 功能

1. **保存研究**: 将研究报告保存到库中
2. **搜索历史**: 通过关键词搜索历史研究
3. **查看详情**: 获取特定研究报告
4. **列表管理**: 查看所有已存储的研究

## 意义

1. **知识积累**: 避免重复研究，积累领域知识
2. **历史追溯**: 方便回顾和引用之前的研究成果
3. **构建知识库**: 长期使用可以构建个人/团队知识库

## 运行

```bash
cargo run --example rucora-deep-research-library
```

## 环境变量

需要设置以下环境变量：
- `OPENAI_API_KEY`: OpenAI API 密钥

## 扩展

除 `InMemoryResearchLibrary` 外，rucora 还提供 `FileResearchLibrary` 用于持久化存储到文件系统。可以实现自定义的 `ResearchLibrary` 来对接数据库或其他存储系统。