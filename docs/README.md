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

### 技能系统

| 文档 | 说明 |
|------|------|
| [Skill 配置规范](skill_yaml_spec.md) | 配置文件完整说明 |
| [Skill 配置示例](skill_yaml_examples.md) | 实际使用示例 |
| [配置优化总结](SKILL_CONFIG_COMPLETE.md) | 完整优化说明 |

### 开发指南

| 文档 | 说明 |
|------|------|
| [对话设计](conversation_guide.md) | 对话系统指南 |
| [Provider 设计](provider_default_model.md) | LLM Provider 实现 |
| [运行时设计](runtime_agent_model_design.md) | Runtime 实现细节 |

### 项目文档

| 文档 | 说明 |
|------|------|
| [INDEX.md](INDEX.md) | 完整文档索引 |
| [CHANGELOG.md](CHANGELOG.md) | 版本更新记录 |
| [DOCUMENTATION_OPTIMIZATION.md](DOCUMENTATION_OPTIMIZATION.md) | 文档优化说明 |

## 🚀 快速开始

### 安装

```bash
cargo add agentkit
```

### 基本使用

```rust
use agentkit::provider::OpenAiProvider;
use agentkit::agent::DefaultAgent;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let provider = OpenAiProvider::from_env()?;
    
    let agent = DefaultAgent::builder()
        .provider(provider)
        .model("gpt-4o-mini")
        .system_prompt("你是有用的助手")
        .build();
    
    let output = agent.run("你好").await?;
    println!("{}", output.text().unwrap_or("无回复"));
    
    Ok(())
}
```

### 使用 Skills

```rust
use agentkit::skills::{SkillLoader, skills_to_tools, SkillExecutor};

// 加载 Skills
let mut loader = SkillLoader::new("skills/");
let skills = loader.load_from_dir().await?;

// 转换为 Tools
let executor = Arc::new(SkillExecutor::new());
let tools = skills_to_tools(&skills, executor, skills_dir);

// 注册到 Agent
let agent = DefaultAgent::builder()
    .provider(provider)
    .model("gpt-4o-mini")
    .system_prompt("你是有用的助手")
    .tools(tools)
    .build();
```

## 🔧 资源配置

### 环境变量

| 变量 | 说明 | 示例 |
|------|------|------|
| `OPENAI_API_KEY` | OpenAI API 密钥 | `sk-...` |
| `ANTHROPIC_API_KEY` | Anthropic API 密钥 | `sk-ant-...` |
| `GOOGLE_API_KEY` | Google Gemini API 密钥 | `...` |
| `OPENAI_BASE_URL` | 自定义 API 地址 | `http://localhost:11434` |

### 支持的 Provider

| Provider | 环境变量 | 文档 |
|----------|----------|------|
| OpenAI | `OPENAI_API_KEY` | [用户指南](user_guide.md) |
| Anthropic | `ANTHROPIC_API_KEY` | [用户指南](user_guide.md) |
| Google Gemini | `GOOGLE_API_KEY` | [用户指南](user_guide.md) |
| Ollama | `OPENAI_BASE_URL` | [快速开始](quick_start.md) |

## 📝 更新日志

查看 [CHANGELOG.md](CHANGELOG.md) 了解最新版本和变更。

## 🤝 贡献

欢迎贡献代码、文档或建议！

## 📄 许可证

AgentKit 使用 MIT 许可证。
