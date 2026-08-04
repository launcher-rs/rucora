# rucora 文档中心

> rucora 是一个用 Rust 编写的高性能、类型安全的 LLM 应用开发框架

## 📚 文档导航

### 新手入门

| 文档 | 说明 | 适合人群 |
|------|------|----------|
| [快速开始](guides/quick_start.md) | 5 分钟上手 rucora | 新用户 |
| [用户指南](guides/user_guide.md) | 完整的使用指南 | 所有用户 |
| [示例集合](guides/cookbook.md) | 实际使用示例 | 实践者 |
| [常见问题](guides/faq.md) | 常见问题解答 | 所有人 |
| [快速参考](guides/QUICK_REFERENCE.md) | API 快速查询 | 所有用户 |
| [故障排查](guides/TROUBLESHOOTING.md) | 常见问题解决 | 所有用户 |
| [示例说明](guides/examples.md) | 示例项目说明 | 实践者 |

### 核心概念

| 文档 | 说明 |
|------|------|
| [设计文档](design/design.md) | 系统设计理念 |
| [Agent 架构](design/agent_runtime_relationship.md) | 理解核心架构 |

### 技能系统

| 文档 | 说明 |
|------|------|
| [Skill 配置规范](guides/skill_yaml_spec.md) | 配置文件完整说明 |
| [Skill 配置示例](guides/skill_yaml_examples.md) | 实际使用示例 |

### 开发指南

| 文档 | 说明 |
|------|------|
| [对话设计](guides/conversation_guide.md) | 对话系统指南 |
| [内存指南](guides/memory_guide.md) | 内存系统使用 |
| [中间件指南](guides/middleware_guide.md) | 中间件开发 |
| [自动对话](guides/agent_auto_conversation.md) | 自动对话功能 |
| [Typestate 模式](guides/typestate_pattern.md) | 构建器为何强制设置 model |
| [发布与版本管理](guides/release_versioning.md) | crates.io 发布与版本策略 |

### Deep Research

| 文档 | 说明 |
|------|------|
| [快速开始指南](guides/deep_research_v2_quickstart.md) | 使用指南 |

### 其他

| 文档 | 说明 |
|------|------|
| [历史存档](archive/README.md) | 历史规划、提案与审查文档 |

## 🚀 快速开始

### 安装

```bash
cargo add rucora
```

### 基本使用

```rust
use rucora::agent::SimpleAgent;
use rucora::prelude::Agent;
use rucora::provider::OpenAiProvider;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let provider = OpenAiProvider::from_env()?;

    let agent = SimpleAgent::builder(provider)
        .model("gpt-4o-mini")
        .system_prompt("你是有用的助手")
        .build();

    let output = agent.run("你好".into()).await?;
    println!("{}", output.text().unwrap_or("无回复"));

    Ok(())
}
```

## 🔧 环境变量

| 变量 | 说明 | 示例 |
|------|------|------|
| `OPENAI_API_KEY` | OpenAI API 密钥 | `sk-...` |
| `ANTHROPIC_API_KEY` | Anthropic API 密钥 | `sk-ant-...` |
| `GOOGLE_API_KEY` | Google Gemini API 密钥 | `...` |
| `OPENAI_BASE_URL` | 自定义 API 地址 | `http://localhost:11434` |

## 📝 更新日志

查看 [CHANGELOG.md](../CHANGELOG.md) 了解最新版本和变更。

## 📄 许可证

rucora 使用 MIT 许可证。
