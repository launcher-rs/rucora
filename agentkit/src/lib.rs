//! agentkit - Agentkit 框架主库
//!
//! # 概述
//!
//! `agentkit` 是 Agentkit 框架的聚合入口 crate，提供：
//! - 统一导出 core（抽象层）与 runtime（编排层）
//! - 具体实现（Provider/Tools/Skills/Memory/Retrieval）
//! - 统一配置系统
//! - 便捷的 prelude 模块
//!
//! # 模块结构
//!
//! ```text
//! agentkit
//! ├── core          - 核心抽象层（重新导出 agentkit-core）
//! ├── runtime       - 运行时（重新导出 agentkit-runtime）
//! ├── provider      - LLM Provider 实现（OpenAI/Ollama/Router）
//! ├── tools         - 工具实现（Shell/File/HTTP/Git/Memory）
//! ├── skills        - 技能实现（Echo/Rhai/Command）
//! ├── memory        - 记忆实现（InMemory/File）
//! ├── retrieval     - 检索实现（Chroma）
//! ├── embed         - Embedding 实现（OpenAI/Ollama）
//! ├── rag           - RAG 管线（Chunking/Indexing/Retrieval）
//! ├── conversation  - 对话历史管理（新增）
//! └── config        - 统一配置系统
//! ```
//!
//! # 快速开始
//!
//! ## 使用 prelude 简化导入
//!
//! ```rust
//! use agentkit::prelude::*;
//!
//! // 现在可以直接使用：
//! // - Runtime trait
//! // - AgentInput, AgentOutput
//! // - ChannelEvent, TokenDeltaEvent
//! // - ProviderError, ToolError, SkillError
//! // - LlmProvider trait
//! // - Tool trait
//! ```
//!
//! ## 创建运行时
//!
//! ```rust,no_run
//! use agentkit::prelude::*;
//! use agentkit::provider::OpenAiProvider;
//! use agentkit_runtime::{DefaultRuntime, ToolRegistry};
//! use std::sync::Arc;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // 创建 Provider
//! let provider = OpenAiProvider::from_env()?;
//!
//! // 创建工具注册表
//! let tools = ToolRegistry::new();
//!
//! // 创建运行时
//! let runtime = DefaultRuntime::new(Arc::new(provider), tools)
//!     .with_system_prompt("你是一个有用的助手")
//!     .with_max_steps(10);
//! # Ok(())
//! # }
//! ```
//!
//! ## 加载 Skills
//!
//! ```rust,no_run
//! use agentkit::skills::load_skills_from_dir;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let skills = load_skills_from_dir("skills").await?;
//!
//! // 将 skills 转换为 tools
//! let tools = skills.as_tools();
//! # Ok(())
//! # }
//! ```
//!
//! ## 使用配置系统
//!
//! ```rust,no_run
//! use agentkit::config::AgentkitConfig;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // 从环境变量和配置文件加载
//! let profile = AgentkitConfig::load().await?;
//!
//! // 构建 provider
//! let provider = AgentkitConfig::build_provider(&profile)?;
//! # Ok(())
//! # }
//! ```
//!
//! # 功能模块
//!
//! ## Provider（模型提供者）
//!
//! 支持多种 LLM Provider：
//!
//! - [`provider::OpenAiProvider`]: OpenAI API 兼容服务
//! - [`provider::OllamaProvider`]: Ollama 本地模型
//! - [`provider::RouterProvider`]: 多 Provider 路由
//! - [`provider::ResilientProvider`]: 带重试机制的 Provider
//!
//! ## Tools（工具）
//!
//! 内置 12+ 种工具：
//!
//! | 工具 | 说明 | 分类 |
//! |------|------|------|
//! | [`tools::ShellTool`] | 执行系统命令 | System |
//! | [`tools::CmdExecTool`] | 受限命令执行 | System |
//! | [`tools::GitTool`] | Git 操作 | System |
//! | [`tools::FileReadTool`] | 读取文件 | File |
//! | [`tools::FileWriteTool`] | 写入文件 | File |
//! | [`tools::FileEditTool`] | 编辑文件 | File |
//! | [`tools::HttpRequestTool`] | HTTP 请求 | Network |
//! | [`tools::WebFetchTool`] | 获取网页 | Network |
//! | [`tools::BrowseTool`] | 浏览网页 | Browser |
//! | [`tools::MemoryStoreTool`] | 存储记忆 | Memory |
//! | [`tools::MemoryRecallTool`] | 检索记忆 | Memory |
//!
//! ## Skills（技能）
//!
//! 技能是对 Tool/Provider 的高级封装：
//!
//! - [`skills::EchoSkill`]: 回显技能（示例）
//! - [`skills::RhaiSkill`]: Rhai 脚本技能
//! - [`skills::CommandSkill`]: 命令模板技能
//!
//! ## Memory（记忆）
//!
//! 支持多种记忆存储：
//!
//! - [`memory::InMemoryMemory`]: 进程内记忆
//! - [`memory::FileMemory`]: 文件记忆
//!
//! ## Retrieval（检索）
//!
//! 支持向量数据库：
//!
//! - [`retrieval::ChromaVectorStore`]: Chroma 向量库
//! - [`retrieval::ChromaPersistentStore`]: Chroma 持久化存储
//!
//! ## Embedding（向量嵌入）
//!
//! 支持多种 Embedding Provider：
//!
//! - [`embed::OpenAiEmbedding`]: OpenAI Embedding
//! - [`embed::OllamaEmbedding`]: Ollama Embedding
//! - [`embed::CachedEmbeddingProvider`]: 带缓存的 Provider
//!
//! ## RAG（检索增强生成）
//!
//! 提供完整的 RAG 管线：
//!
//! - [`rag::chunk_text`]: 文本分块
//! - [`rag::index_chunks`]: 索引分块
//! - [`rag::index_text`]: 索引文本
//! - [`rag::retrieve`]: 检索引用
//! - [`rag::Citation`]: 引用格式
//!
//! ## Config（配置）
//!
//! 统一配置系统：
//!
//! - 支持 YAML/TOML 配置文件
//! - 支持 Profile 切换（dev/prod）
//! - 支持环境变量覆盖
//!
//! ```rust,no_run
//! use agentkit::config::AgentkitConfig;
//!
//! # async fn example() {
//! // 设置配置文件路径
//! std::env::set_var("AGENTKIT_CONFIG", "config.yaml");
//!
//! // 设置 Profile
//! std::env::set_var("AGENTKIT_PROFILE", "prod");
//!
//! // 加载配置
//! let profile = AgentkitConfig::load().await.unwrap();
//! # }
//! ```
//!
//! # Feature 标志
//!
//! - `mcp`: 启用 MCP 协议支持
//! - `a2a`: 启用 A2A 协议支持
//!
//! # 完整示例
//!
//! ```rust,no_run
//! use agentkit::prelude::*;
//! use agentkit::provider::OpenAiProvider;
//! use agentkit::skills::load_skills_from_dir;
//! use agentkit_runtime::{DefaultRuntime, ToolRegistry};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // 1. 创建 Provider
//!     let provider = OpenAiProvider::from_env()?;
//!
//!     // 2. 加载 Skills
//!     let skills = load_skills_from_dir("skills").await?;
//!
//!     // 3. 创建工具注册表
//!     let mut tools = ToolRegistry::new();
//!     for tool in skills.as_tools() {
//!         tools = tools.register_arc(tool);
//!     }
//!
//!     // 4. 创建运行时
//!     let runtime = DefaultRuntime::new(Arc::new(provider), tools)
//!         .with_system_prompt("你是一个有用的助手")
//!         .with_max_steps(10);
//!
//!     // 5. 执行对话
//!     let input = AgentInput {
//!         messages: vec![],
//!         metadata: None,
//!     };
//!
//!     let output = runtime.run(input).await?;
//!     println!("助手回复：{}", output.message.content);
//!
//!     Ok(())
//! }
//! ```
//!
//! # 相关文档
//!
//! - [agentkit-core](../agentkit_core/index.html): 核心抽象层
//! - [agentkit-runtime](../agentkit_runtime/index.html): 运行时
//! - [Agentkit GitHub](https://github.com/agentkit-rs/agentkit)

/// 导出 core 抽象层（traits + 共享类型）
pub use agentkit_core as core;

/// 常用导入集合（prelude）
///
/// # 使用方式
///
/// ```rust
/// use agentkit::prelude::*;
/// ```
///
/// 这个模块重新导出了最常用的类型和 trait，避免用户手动导入多个模块。
///
/// # 导出的类型
///
/// - [`Runtime`]: Runtime trait
/// - [`AgentInput`], [`AgentOutput`]: Agent 输入输出
/// - [`ChannelEvent`], [`TokenDeltaEvent`]: 事件类型
/// - [`ProviderError`], [`ToolError`], [`SkillError`]: 错误类型
/// - [`LlmProvider`], [`Tool`]: 核心 trait
pub mod prelude {
    /// Runtime trait（用于支持可替换 runtime）
    pub use crate::core::runtime::Runtime;

    /// Core 常用类型与错误
    pub use crate::core::{
        agent::types::*, channel::types::*, error::*, memory::types::*, provider::types::*,
        skill::types::*, tool::types::*,
    };

    /// Core 抽象层常用 trait
    pub use crate::core::{provider::LlmProvider, tool::Tool};
}

/// Provider（模型提供者）实现与示例
///
/// 本模块包含各种 LLM 提供者的具体实现：
///
/// - [`provider::OpenAiProvider`]: OpenAI API 兼容服务
/// - [`provider::OllamaProvider`]: Ollama 本地模型
/// - [`provider::RouterProvider`]: 多 Provider 路由
/// - [`provider::ResilientProvider`]: 带重试机制的 Provider
pub mod provider;

/// Embedding（向量嵌入）实现与示例
///
/// 本模块包含 Embedding Provider 的实现：
///
/// - [`embed::OpenAiEmbedding`]: OpenAI Embedding
/// - [`embed::OllamaEmbedding`]: Ollama Embedding
/// - [`embed::CachedEmbeddingProvider`]: 带缓存的 Provider
pub mod embed;

/// Retrieval（语义检索）实现与示例
///
/// 本模块包含 VectorStore 的实现：
///
/// - [`retrieval::ChromaVectorStore`]: Chroma 向量库
/// - [`retrieval::ChromaPersistentStore`]: Chroma 持久化存储
pub mod retrieval;

/// RAG（检索增强生成）管线
///
/// 提供完整的 RAG 功能：
///
/// - [`rag::chunk_text`]: 文本分块
/// - [`rag::index_chunks`]: 索引分块
/// - [`rag::retrieve`]: 检索引用
pub mod rag;

/// Memory（记忆）实现与示例
///
/// 本模块包含 Memory 的实现：
///
/// - [`memory::InMemoryMemory`]: 进程内记忆
/// - [`memory::FileMemory`]: 文件记忆
pub mod memory;

/// 统一配置系统
///
/// 支持：
/// - 配置文件（YAML/TOML）
/// - Profile 切换（dev/prod）
/// - 环境变量覆盖
pub mod config;

/// Conversation（对话历史）管理
///
/// 提供对话历史管理功能：
///
/// - [`conversation::ConversationManager`]: 对话管理器
/// - 消息窗口限制
/// - Token 限制
/// - 消息压缩
pub mod conversation;

/// Prompt 模板系统
///
/// 提供 Prompt 模板功能：
///
/// - [`prompt::PromptTemplate`]: Prompt 模板
/// - [`prompt::PromptBuilder`]: Prompt 构建器
/// - 变量替换
/// - 条件渲染
pub mod prompt;

/// Token 计数和成本管理
///
/// 提供 Token 计数和 API 成本管理功能：
///
/// - [`cost::TokenCounter`]: Token 计数器
/// - [`cost::CostTracker`]: 成本追踪器
/// - 预算控制
/// - 使用统计
pub mod cost;

/// 中间件系统
///
/// 提供请求/响应拦截功能：
///
/// - [`middleware::Middleware`]: 中间件 trait
/// - [`middleware::MiddlewareChain`]: 中间件链
/// - [`middleware::LoggingMiddleware`]: 日志中间件
/// - [`middleware::CacheMiddleware`]: 缓存中间件
/// - [`middleware::RateLimitMiddleware`]: 限流中间件
/// - [`middleware::MetricsMiddleware`]: 指标中间件
pub mod middleware;

/// MCP（Model Context Protocol）集成（可选）
///
/// 需要启用 `mcp` feature。
#[cfg(feature = "mcp")]
pub use agentkit_mcp as mcp;

/// A2A（Agent-to-Agent）集成（可选）
///
/// 需要启用 `a2a` feature。
#[cfg(feature = "a2a")]
pub use agentkit_a2a as a2a;

/// Skills（技能）实现与示例
///
/// 本模块包含具体的技能实现：
///
/// - [`skills::EchoSkill`]: 回显技能（示例）
/// - [`skills::RhaiSkill`]: Rhai 脚本技能
/// - [`skills::CommandSkill`]: 命令模板技能
/// - [`skills::load_skills_from_dir`]: 从目录加载 skills
pub mod skills;

/// Tools（工具）实现与示例
///
/// 本模块包含 12+ 种工具的实现：
///
/// ## 系统工具
/// - [`tools::ShellTool`]: 执行系统命令
/// - [`tools::CmdExecTool`]: 受限命令执行
/// - [`tools::GitTool`]: Git 操作
///
/// ## 文件工具
/// - [`tools::FileReadTool`]: 读取文件
/// - [`tools::FileWriteTool`]: 写入文件
/// - [`tools::FileEditTool`]: 编辑文件
///
/// ## 网络工具
/// - [`tools::HttpRequestTool`]: HTTP 请求
/// - [`tools::WebFetchTool`]: 获取网页
///
/// ## 浏览器工具
/// - [`tools::BrowseTool`]: 浏览网页
/// - [`tools::BrowserOpenTool`]: 打开浏览器
///
/// ## 记忆工具
/// - [`tools::MemoryStoreTool`]: 存储记忆
/// - [`tools::MemoryRecallTool`]: 检索记忆
pub mod tools;
