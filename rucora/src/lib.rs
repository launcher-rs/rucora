//! # rucora
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
//! rucora = "0.1"
//! tokio = { version = "1", features = ["full"] }
//! anyhow = "1"
//! ```
//!
//! ### 2. 编写代码
//!
//! ```rust,no_run
//! use rucora::provider::OpenAiProvider;
//! use rucora::agent::DefaultAgent;
//! use rucora::prelude::Agent;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let provider = OpenAiProvider::new("https://api.openai.com/v1","sk-*******************");
//!     
//!     let agent = DefaultAgent::builder()
//!         .provider(provider)
//!         .model("gpt-4o-mini")
//!         .system_prompt("你是有用的助手")
//!         .build();
//!     
//!     let output = agent.run("你好".into()).await?;
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
//! ```rust,ignore
//! use rucora::agent::DefaultAgent;
//!
//! let agent = DefaultAgent::builder()
//!     .provider(provider)
//!     .model("gpt-4o-mini")
//!     .system_prompt("你是有用的助手")
//!     .build();
//!
//! let output = agent.run("北京天气怎么样？".into()).await?;
//! ```
//!
//! ### Tool（工具）
//!
//! 工具提供具体能力，如执行命令、读取文件、HTTP 请求等。
//!
//! ```rust,ignore
//! use rucora::tools::{ShellTool, FileReadTool};
//! use rucora::agent::DefaultAgent;
//!
//! let agent = DefaultAgent::builder()
//!     .provider(provider)
//!     .model("gpt-4o-mini")
//!      .tool(ShellTool::new())
//!      .tool(FileReadTool::new())
//!     .build();
//! ```

// ===== 模块导出 =====

pub use rucora_core as core;

// Agent 模块
pub mod agent;

// 子模块重新导出（通过 feature 控制）

// Provider 模块
#[cfg(feature = "providers")]
pub use rucora_providers as provider;

// Tools 模块
#[cfg(feature = "tools")]
pub use rucora_tools as tools;

// Skills 模块（可选）
#[cfg(feature = "skills")]
pub use rucora_skills as skills;

// Memory 模块
pub mod memory;

// Retrieval 模块
#[cfg(feature = "retrieval")]
pub use rucora_retrieval as retrieval;

// Embedding 模块
#[cfg(feature = "embed")]
pub use rucora_embed as embed;

// RAG 模块
pub mod rag;

// Deep Research 模块
pub mod deep_research;

// 上下文压缩模块
pub mod compact;

// 重新导出压缩引擎类型
pub use compact::{CompressionConfig, CompressionStrategy, LayeredCompressor};

// Conversation 模块
pub mod conversation;

// Prompt 模块
pub mod prompt;

// Middleware 模块
pub mod middleware;

// MCP 模块（可选）
#[cfg(feature = "mcp")]
pub use rucora_mcp as mcp;

// A2A 模块（可选）
#[cfg(feature = "a2a")]
pub use rucora_a2a as a2a;

// ===== 便捷导出 =====

/// 常用类型和 trait 的快速访问
///
/// 使用 `use rucora::prelude::*;` 可以快速导入常用类型。
pub mod prelude {
    pub use crate::agent::DefaultAgent;
    pub use crate::agent::{AgentStream, StreamExt};
    #[cfg(feature = "providers")]
    pub use crate::provider::OpenAiProvider;
    pub use rucora_core::agent::{Agent, AgentInput, AgentOutput};
    pub use rucora_core::channel::types::{ChannelEvent, TokenDeltaEvent};
    pub use rucora_core::error::{AgentError, ProviderError, ToolError};
    pub use rucora_core::provider::LlmProvider;
    pub use rucora_core::provider::types::LlmParams;
    pub use rucora_core::tool::Tool;
}
