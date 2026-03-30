# AgentKit

> 用 Rust 编写的高性能、类型安全的 LLM 应用开发框架

[![Documentation](https://img.shields.io/badge/docs-latest-blue)](docs/README.md)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)

## ✨ 特性

- ⚡ **极速性能** - Rust 原生，零成本抽象
- 🔒 **类型安全** - 编译时错误检查，运行时更可靠
- 💰 **成本监控** - 内置 Token 计数和成本管理
- 🧰 **丰富工具** - 12+ 内置工具（Shell/File/HTTP/Git/Memory 等）
- 🔌 **灵活集成** - 支持 10+ LLM Provider（OpenAI、Anthropic、Gemini、Ollama 等）
- 📊 **可观测性** - 完整的日志、指标、追踪支持
- 🧠 **Agent 架构** - 思考与执行分离，支持自定义 Agent

## 🚀 快速开始

```bash
cargo add agentkit
```

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

## 📚 文档

**完整文档请查看 [docs/README.md](docs/README.md)**

### 新手入门
- [快速开始](docs/quick_start.md) - 5 分钟上手
- [用户指南](docs/user_guide.md) - 完整功能说明
- [示例集合](docs/cookbook.md) - 实际使用示例
- [常见问题](docs/faq.md) - FAQ

### 技能系统
- [Skill 配置规范](docs/skill_yaml_spec.md) - 配置文件完整说明
- [Skill 配置示例](docs/skill_yaml_examples.md) - 实际使用示例

### 架构设计
- [设计文档](docs/design.md) - 系统设计理念
- [Agent 与 Runtime](docs/agent_runtime_relationship.md) - 核心架构说明
- [快速参考](docs/QUICK_REFERENCE.md) - API 快速查询

### 项目文档
- [更新日志](docs/CHANGELOG.md) - 版本更新记录
- [文档索引](docs/INDEX.md) - 完整文档列表

## 📦 项目结构

```
agentkit/
├── docs/                      # 文档目录
│   ├── README.md              # 文档导航
│   ├── INDEX.md               # 文档索引
│   ├── quick_start.md         # 快速开始
│   ├── user_guide.md          # 用户指南
│   ├── skill_yaml_spec.md     # Skill 配置规范
│   └── ...
├── agentkit/                  # 主库（实现聚合）
├── agentkit-core/             # 核心抽象层
├── agentkit-runtime/          # 运行时实现
├── agentkit-cli/              # 命令行工具
└── examples/                  # 示例代码
```

## 🔧 环境变量

| 变量 | 说明 | 示例 |
|------|------|------|
| `OPENAI_API_KEY` | OpenAI API 密钥 | `sk-...` |
| `ANTHROPIC_API_KEY` | Anthropic API 密钥 | `sk-ant-...` |
| `GOOGLE_API_KEY` | Google Gemini API 密钥 | `...` |
| `OPENAI_BASE_URL` | 自定义 API 地址 | `http://localhost:11434` |

## 🎯 支持的 Provider

| Provider | 环境变量 | 文档 |
|----------|----------|------|
| OpenAI | `OPENAI_API_KEY` | [用户指南](docs/user_guide.md) |
| Anthropic | `ANTHROPIC_API_KEY` | [用户指南](docs/user_guide.md) |
| Google Gemini | `GOOGLE_API_KEY` | [用户指南](docs/user_guide.md) |
| Ollama | `OPENAI_BASE_URL` | [快速开始](docs/quick_start.md) |

## 📝 更新日志

查看 [CHANGELOG.md](docs/CHANGELOG.md) 了解最新版本和变更。

## 🤝 贡献

欢迎贡献代码、文档或建议！

## 📄 许可证

AgentKit 使用 MIT 许可证。
