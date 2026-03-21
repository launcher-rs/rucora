//! agentkit-runtime - Agentkit 运行时编排层
//!
//! # 概述
//!
//! `agentkit-runtime` 是 Agentkit 框架的运行时编排层，负责：
//! - 调用 LLM Provider 进行对话
//! - 执行工具调用循环（Tool-Calling Loop）
//! - 管理工具注册和执行策略
//! - 提供流式和非流式两种执行模式
//! - 支持多种工具来源（内置工具、Skills、MCP、A2A）
//!
//! # 核心组件
//!
//! ## DefaultRuntime
//!
//! 默认的运行时实现，提供完整的 tool-calling 循环：
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use agentkit_runtime::{DefaultRuntime, ToolRegistry};
//! use agentkit_core::provider::LlmProvider;
//! use agentkit_core::agent::types::AgentInput;
//!
//! # async fn example(provider: Arc<dyn LlmProvider>) -> Result<(), Box<dyn std::error::Error>> {
//! // 创建工具注册表
//! let tools = ToolRegistry::new();
//!
//! // 创建运行时
//! let runtime = DefaultRuntime::new(provider, tools)
//!     .with_system_prompt("你是一个有用的助手")
//!     .with_max_steps(10)
//!     .with_max_tool_concurrency(3);
//!
//! // 执行对话
//! let input = AgentInput {
//!     messages: vec![],
//!     metadata: None,
//! };
//! let output = runtime.run(input).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## ToolRegistry
//!
//! 工具注册表，支持从多种来源注册和管理工具：
//!
//! ```rust
//! use agentkit_runtime::{ToolRegistry, ToolSource};
//! use agentkit::tools::{ShellTool, FileReadTool};
//!
//! // 创建注册表
//! let registry = ToolRegistry::new()
//!     .with_namespace("my_tools")  // 设置命名空间
//!     .register(ShellTool::new())
//!     .register_with_source(FileReadTool::new(), ToolSource::BuiltIn);
//!
//! // 查询工具
//! assert!(registry.get("my_tools::shell").is_some());
//!
//! // 按来源过滤
//! let builtin_tools = registry.filter_by_source(ToolSource::BuiltIn);
//! ```
//!
//! ## ToolLoader
//!
//! 统一的工具加载器，支持从不同来源加载工具：
//!
//! ```rust,no_run
//! use agentkit_runtime::loader::ToolLoader;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // 加载内置工具和 Skills
//! let loader = ToolLoader::new()
//!     .load_builtin_tools()
//!     .load_skills_from_dir("skills")
//!     .await?;
//!
//! let registry = loader.build();
//! # Ok(())
//! # }
//! ```
//!
//! # 工具来源类型
//!
//! - **BuiltIn**: 内置工具（shell、file、http 等）
//! - **Skill**: 从 Skills 目录加载的技能
//! - **Mcp**: 从 MCP 服务器加载的工具
//! - **A2A**: 从 A2A 协议加载的工具
//! - **Custom**: 用户自定义工具
//!
//! # 流式执行
//!
//! 支持流式输出 token 和工具调用事件：
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use futures_util::StreamExt;
//! use agentkit_runtime::{DefaultRuntime, ToolRegistry, ChannelEvent};
//! use agentkit_core::provider::LlmProvider;
//! use agentkit_core::agent::types::AgentInput;
//!
//! # async fn example(provider: Arc<dyn LlmProvider>) -> Result<(), Box<dyn std::error::Error>> {
//! let runtime = DefaultRuntime::new(provider, ToolRegistry::new());
//! let input = AgentInput { messages: vec![], metadata: None };
//!
//! // 流式执行
//! let mut stream = runtime.run_stream(input);
//! while let Some(event) = stream.next().await {
//!     match event {
//!         Ok(ChannelEvent::TokenDelta(delta)) => {
//!             print!("{}", delta.delta);
//!         }
//!         Ok(ChannelEvent::ToolCall(call)) => {
//!             println!("调用工具：{}", call.name);
//!         }
//!         _ => {}
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Feature 标志
//!
//! - `builtin-tools`: 启用内置工具加载（默认启用）
//! - `skills`: 启用 Skills 支持（默认启用）
//! - `mcp`: 启用 MCP 协议支持
//! - `a2a`: 启用 A2A 协议支持
//! - `full`: 启用所有功能
//!
//! # 架构设计
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                    DefaultRuntime                        │
//! ├─────────────────────────────────────────────────────────┤
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │
//! │  │  Provider   │  │   Tools     │  │    Policy       │  │
//! │  │  (LLM API)  │──│  (Registry) │──│  (Allow/Deny)   │  │
//! │  └─────────────┘  └─────────────┘  └─────────────────┘  │
//! │         │                │                  │            │
//! │         └────────────────┴──────────────────┘            │
//! │                           │                              │
//! │                  ┌────────▼────────┐                     │
//! │                  │ Tool Execution  │                     │
//! │                  │    Pipeline     │                     │
//! │                  └─────────────────┘                     │
//! └─────────────────────────────────────────────────────────┘
//! ```

/// 默认运行时实现和构建器
pub mod default_runtime;

/// 统一工具加载器（支持 BuiltIn/Skills/MCP/A2A）
pub mod loader;

/// 工具调用策略（Policy）
pub mod policy;

/// 工具执行管道
pub mod tool_execution;

/// 工具注册表（支持多来源和元数据）
pub mod tool_registry;

/// 轨迹持久化与回放
pub mod trace;

/// 工具执行辅助函数
pub mod utils;

// 重新导出核心类型
pub use agentkit_core::{
    agent::types::{AgentInput, AgentOutput},
    channel::types::{ChannelEvent, DebugEvent, ErrorEvent, TokenDeltaEvent},
    error::{AgentError, ToolError},
    provider::LlmProvider,
    runtime::{NoopRuntimeObserver, Runtime, RuntimeObserver},
    tool::Tool,
};

// 导出运行时类型
pub use default_runtime::{DefaultRuntime, DefaultRuntimeBuilder, RuntimeConfig};

// 导出加载器类型
pub use loader::{load_all_tools, ToolLoadStats, ToolLoader};

// 导出策略类型
pub use policy::{
    AllowAllToolPolicy, CommandPolicyConfig, DefaultToolPolicy, ToolCallContext, ToolPolicy,
};

// 导出注册表类型
pub use tool_registry::{ToolMetadata, ToolRegistry, ToolSource, ToolWrapper};
