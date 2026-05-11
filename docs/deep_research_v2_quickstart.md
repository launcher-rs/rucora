# rucora Deep Research 0.2 快速开始指南

## 什么是 Deep Research？

Deep Research（深度研究）是一种 AI 辅助的研究能力，它能够：
- 自动搜索相关信息
- 阅读和理解网页内容
- 综合多方观点
- 生成结构化的研究报告
- 正确标注信息来源

## rucora 0.2 提供的选择

根据你的需求，可以选择不同的研究模式：

| 模式 | 速度 | 深度 | 适用场景 |
|------|------|------|----------|
| **Fast** | 快 (30秒-3分钟) | 基础 | 快速事实查询 |
| **Standard** | 中 (5-15分钟) | 中等 | 常规研究任务 |
| **Agentic** | 慢 (10-30分钟) | 深入 | 复杂问题分析 |
| **Library** | 视情况 | 可累积 | 长期研究主题 |
| **Academic** | 慢 (15-30分钟) | 学术级 | 论文/报告准备 |

## 快速开始

### 1. 准备工作

确保已安装 Rust 和必要的依赖：

```bash
# 检查 Rust
rustc --version
cargo --version
```

### 2. 快速研究模式（推荐首次尝试）

```bash
# 克隆项目
cd rucora

# 运行快速研究示例
cargo run -p rucora-deep-research-fast

# 或使用交互模式
cargo run -p rucora-deep-research-fast -- -i
```

### 3. 标准研究模式

```bash
# 运行标准研究示例
cargo run -p rucora-deep-research

# 指定研究主题
echo "人工智能在教育中的应用" | cargo run -p rucora-deep-research
```

### 4. 高级研究模式

```bash
# Agentic 自主研究（需要更长运行时间）
cargo run -p rucora-deep-research-agentic

# 研究库模式（需要配置存储路径）
RUCORA_LIBRARY_PATH=./library cargo run -p rucora-deep-research-library
```

## 环境配置

### 创建 .env 文件

在示例目录下创建 `.env` 文件：

```bash
# OpenAI 配置示例
OPENAI_API_KEY=sk-your-key-here
OPENAI_DEFAULT_MODEL=gpt-4o-mini

# 或使用 Ollama（本地）
OLLAMA_BASE_URL=http://localhost:11434/v1
OLLAMA_DEFAULT_MODEL=qwen2.5:14b

# 可选：Tavily 搜索 API
TAVILY_API_KEY=your-tavily-key

# 可选：Serper Google 搜索
SERPER_API_KEY=your-serper-key
```

## 配置文件说明

### 研究策略配置

```toml
# ~/.rucora/research.toml
[default]
strategy = "standard"          # 默认策略
max_iterations = 10           # 最大迭代次数
timeout = 600                 # 超时时间（秒）

[strategy.fast]
max_searches = 3              # 快速模式最多搜索次数
max_output = 1000             # 最大输出字数

[strategy.agentic]
confidence_threshold = 0.9    # 置信度阈值
allow_dynamic_strategy = true # 允许动态调整策略

[engines]
priority = ["tavily", "serper", "arxiv"]
fallback = ["duckduckgo", "websearch"]

[library]
enabled = true
path = "~/.rucora/research_library"
```

## 常见问题

### Q: 哪种模式最适合我？

- **快速事实查询** → 使用 Fast 模式
- **常规研究任务** → 使用 Standard 模式
- **复杂问题需要深入分析** → 使用 Agentic 模式
- **需要积累知识库** → 使用 Library 模式
- **学术论文研究** → 使用 Academic 模式

### Q: 研究时间太长怎么办？

1. 减少 `max_iterations` 配置
2. 使用 Fast 模式
3. 检查网络连接
4. 使用更快的模型

### Q: 如何获取更好的搜索结果？

1. 配置 Tavily 或 Serper API（更精准）
2. 使用更强大的模型
3. 调整搜索关键词

### Q: 报告保存在哪里？

默认保存在当前目录下：
- `research_report_{主题}.md`

可以在配置中指定输出目录：

```toml
[output]
directory = "./research_results"
```

## 示例输出

### Fast 模式输出示例

```markdown
# 快速研究：人工智能在医疗领域的应用

**研究时间**: 2026-05-11 14:30
**模式**: Fast（快速）

## 核心发现

1. **诊断辅助**：AI 系统可帮助医生分析医学影像，提高诊断准确率
   - 来源: https://example.com/ai-diagnosis

2. **药物研发**：机器学习加速新药研发流程
   - 来源: https://example.com/drug-discovery

## 总结

AI 在医疗领域有广泛应用前景，主要体现在诊断、药物研发和患者管理等方面。
```

### Standard 模式输出示例

```markdown
# 深度研究：中东局势分析

**研究时间**: 2026-05-11 14:00-14:45
**模式**: Standard（标准）

## 执行摘要

[200字核心发现]

## 一、背景与概述

[详细背景分析]

## 二、当前状况分析

[最新动态、关键事件]

... (完整的 8-10 章节报告)
```

## 进阶使用

### 编程方式使用

```rust
use rucora::deep_research::{ResearchEngine, ResearchConfig, ResearchStrategy};

#[tokio::main]
async fn main() -> Result<()> {
    let config = ResearchConfig {
        strategy: ResearchStrategy::Standard,
        max_iterations: 10,
        ..Default::default()
    };
    
    let engine = ResearchEngine::new(provider, config);
    let report = engine.research("你的研究主题").await?;
    
    println!("{}", report.to_markdown());
    Ok(())
}
```

### 自定义策略

```rust
use rucora::deep_research::SearchStrategy;

struct MyCustomStrategy;

impl SearchStrategy for MyCustomStrategy {
    fn name(&self) -> &'static str {
        "my-custom-strategy"
    }
    
    async fn search(&self, ...) {
        // 实现自定义搜索逻辑
    }
}
```

## 相关文档

- [Deep Research 0.2 实施计划](./deep_research_v2_plan.md)
- [实现思路详解](./deep_research_v2_implementation.md)
- [用户指南](./user_guide.md)
- [Tools 系统](./tools_guide.md)