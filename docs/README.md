# AgentKit 文档中心

> AgentKit 是一个用 Rust 编写的高性能、类型安全的 LLM 应用开发框架

## 📚 文档导航

### 新手入门

| 文档 | 说明 | 适合人群 |
|------|------|----------|
| [快速开始](quick_start.md) | 5 分钟上手 AgentKit | 新用户 |
| [用户指南](user_guide.md) | 完整的使用指南 | 所有用户 |
| [示例集合](cookbook.md) | 实际使用示例 | 实践者 |
| [常见问题](faq.md) | 常见问题解答 | 所有人 |

### 核心概念

| 文档 | 说明 |
|------|------|
| [设计文档](design.md) | 系统设计理念 |
| [Agent 与 Runtime](agent_runtime_relationship.md) | 理解核心架构 |
| [快速参考](QUICK_REFERENCE.md) | API 快速查询 |
| [架构改进](ARCHITECTURE_IMPROVEMENT.md) | 架构演进方案 |

### 技能系统

| 文档 | 说明 |
|------|------|
| [Skill 配置规范](skill_yaml_spec.md) | 配置文件完整说明 |
| [Skill 配置示例](skill_yaml_examples.md) | 实际使用示例 |

### 开发指南

| 文档 | 说明 |
|------|------|
| [对话设计](conversation_guide.md) | 对话系统指南 |
| [Provider 设计](provider_default_model.md) | LLM Provider 实现 |
| [运行时设计](runtime_agent_model_design.md) | Runtime 实现细节 |
| [内存指南](memory_guide.md) | 内存系统使用 |
| [中间件指南](middleware_guide.md) | 中间件开发 |
| [自动对话](agent_auto_conversation.md) | 自动对话功能 |

### 其他

| 文档 | 说明 |
|------|------|
| [示例说明](examples.md) | 示例项目说明 |
| [故障排查](TROUBLESHOOTING.md) | 常见问题解决 |

## 🚀 快速开始

### 安装

```bash
cargo add agentkit
```

### 基本使用

```rust
use agentkit::provider::OpenAiProvider;
use agentkit::agent::DefaultAgent;
use agentkit::prelude::Agent;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let provider = OpenAiProvider::from_env()?;

    let agent = DefaultAgent::builder()
        .provider(provider)
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

AgentKit 使用 MIT 许可证。
