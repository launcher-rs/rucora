//! # AgentKit
//!
//! 用 Rust 编写的高性能、类型安全的 LLM 应用开发框架
//!
//! ## 特性
//!
//! - ⚡ **极速性能** - Rust 原生，零成本抽象
//! - 🔒 **类型安全** - 编译时错误检查，运行时更可靠
//! - 💰 **成本监控** - 内置 Token 计数和成本管理
//! - 🧰 **丰富工具** - 12+ 内置工具（Shell/File/HTTP/Git/Memory 等）
//! - 🔌 **灵活集成** - 支持 10+ LLM Provider（OpenAI、Anthropic、Gemini、Ollama 等）
//! - 📊 **可观测性** - 完整的日志、指标、追踪支持
//! - 🧠 **Agent 架构** - 思考与执行分离，支持自定义 Agent
//!
//! ## 快速开始
//!
//! ### 1. 添加依赖
//!
//! ```toml
//! [dependencies]
//! agentkit = "0.1"
//! tokio = { version = "1", features = ["full"] }
//! anyhow = "1"
//! ```
//!
//! ### 2. 设置环境变量
//!
//! ```bash
//! # 使用 OpenAI
//! export OPENAI_API_KEY=sk-your-key
//!
//! # 或使用 Ollama（本地）
//! export OPENAI_BASE_URL=http://localhost:11434
//! ```
//!
//! ### 3. 编写代码
//!
//! ```rust,no_run
//! use agentkit::provider::OpenAiProvider;
//! use agentkit::agent::DefaultAgent;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let provider = OpenAiProvider::from_env()?;
//!     
//!     let agent = DefaultAgent::builder()
//!         .provider(provider)
//!         .model("gpt-4o-mini")
//!         .system_prompt("你是有用的助手")
//!         .build();
//!     
//!     let output = agent.run("你好").await?;
//!     println!("{}", output.text().unwrap_or("无回复"));
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### 4. 运行
//!
//! ```bash
//! cargo run
//! ```
//!
//! ## 核心概念
//!
//! ### Agent（智能体）
//!
//! Agent 负责思考和决策。它接收用户输入，分析需求，决定是否需要调用工具。
//!
//! ```rust,no_run
//! use agentkit::agent::DefaultAgent;
//!
//! let agent = DefaultAgent::builder()
//!     .provider(provider)
//!     .model("gpt-4o-mini")
//!     .system_prompt("你是有用的助手")
//!     .build();
//!
//! let output = agent.run("北京天气怎么样？").await?;
//! ```
//!
//! ### Tool（工具）
//!
//! 工具提供具体能力，如执行命令、读取文件、HTTP 请求等。
//!
//! ```rust,no_run
//! use agentkit::tools::{ShellTool, FileReadTool};
//! use agentkit::agent::DefaultAgent;
//!
//! let agent = DefaultAgent::builder()
//!     .provider(provider)
//!     .model("gpt-4o-mini")
//!     .tool(ShellTool)
//!     .tool(FileReadTool)
//!     .build();
//! ```
//!
//! ### Skill（技能）
//!
//! 技能是可配置的自动化任务，通过配置文件定义。
//!
//! ```rust,no_run
//! use agentkit::skills::{SkillLoader, skills_to_tools, SkillExecutor};
//! use std::sync::Arc;
//!
//! // 加载 Skills
//! let mut loader = SkillLoader::new("skills/");
//! let skills = loader.load_from_dir().await?;
//!
//! // 转换为 Tools
//! let executor = Arc::new(SkillExecutor::new());
//! let tools = skills_to_tools(&skills, executor, skills_dir);
//!
//! // 注册到 Agent
//! let agent = DefaultAgent::builder()
//!     .provider(provider)
//!     .tools(tools)
//!     .build();
//! ```
//!
//! ## 学习路径
//!
//! ### 新手
//! 1. 运行 [Hello World](#快速开始) 示例
//! 2. 阅读 [快速开始](docs/quick_start.md)
//! 3. 查看 [用户指南](docs/user_guide.md)
//! 4. 参考 [示例集合](docs/cookbook.md)
//!
//! ### 开发者
//! 1. 阅读 [设计文档](docs/design.md)
//! 2. 学习 [Agent 与 Runtime](docs/agent_runtime_relationship.md)
//! 3. 参考 [快速参考](docs/QUICK_REFERENCE.md)
//!
//! ### 技能开发者
//! 1. 阅读 [Skill 配置规范](docs/skill_yaml_spec.md)
//! 2. 参考 [Skill 配置示例](docs/skill_yaml_examples.md)
//!
//! ## 项目结构
//!
//! ```text
//! agentkit
//! ├── core          - 核心抽象层（重新导出 agentkit-core）
//! ├── runtime       - 运行时（重新导出 agentkit-runtime，需要 `runtime` feature）
//! ├── agent         - Agent 实现（增强的 DefaultAgent，支持 Tools/MCP/A2A/Skills）
//! ├── provider      - LLM Provider 实现（OpenAI/Ollama/Router）
//! ├── tools         - 工具实现（Shell/File/HTTP/Git/Memory）
//! ├── skills        - 技能实现（Echo/Rhai/Command）
//! ├── mcp           - MCP 协议集成（需要 `mcp` feature）
//! ├── a2a           - A2A 协议集成（需要 `a2a` feature）
//! ├── memory        - 记忆实现（InMemory/File）
//! ├── retrieval     - 检索实现（Chroma）
//! ├── embed         - Embedding 实现（OpenAI/Ollama）
//! ├── rag           - RAG 管线（Chunking/Indexing/Retrieval）
//! ├── conversation  - 对话历史管理
//! └── config        - 统一配置系统
//! ```
//!
//! ## 相关文档
//!
//! - [完整文档](docs/README.md)
//! - [示例集合](docs/cookbook.md)
//! - [常见问题](docs/faq.md)
//! - [更新日志](docs/CHANGELOG.md)

// ===== 模块导出 =====

pub use agentkit_core as core;

// Agent 模块
pub mod agent;

// 子模块重新导出（通过 feature 控制）

// Provider 模块
#[cfg(feature = "providers")]
pub use agentkit_providers as provider;

// Tools 模块
#[cfg(feature = "tools")]
pub use agentkit_tools as tools;

// Skills 模块（可选）
#[cfg(feature = "skills")]
pub use agentkit_skills as skills;

// Memory 模块
pub mod memory;

// Retrieval 模块
#[cfg(feature = "retrieval")]
pub use agentkit_retrieval as retrieval;

// Embedding 模块
#[cfg(feature = "embed")]
pub use agentkit_embed as embed;

// RAG 模块
pub mod rag;

// 上下文压缩模块
pub mod compact;

// Conversation 模块
pub mod conversation;

// Prompt 模块
pub mod prompt;

// Middleware 模块
pub mod middleware;

// MCP 模块（可选）
#[cfg(feature = "mcp")]
pub use agentkit_mcp as mcp;

// A2A 模块（可选）
#[cfg(feature = "a2a")]
pub use agentkit_a2a as a2a;

// ===== 便捷导出 =====

/// 常用类型和 trait 的快速访问
///
/// 使用 `use agentkit::prelude::*;` 可以快速导入常用类型。
pub mod prelude {
    pub use crate::agent::DefaultAgent;
    pub use crate::provider::OpenAiProvider;
    pub use agentkit_core::agent::{Agent, AgentInput, AgentOutput};
    pub use agentkit_core::channel::types::{ChannelEvent, TokenDeltaEvent};
    pub use agentkit_core::error::{AgentError, ProviderError, ToolError};
    pub use agentkit_core::provider::LlmProvider;
    pub use agentkit_core::tool::Tool;
}
