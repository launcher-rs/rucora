//! agentkit-runtime - Agentkit 运行时编排层
//!
//! # 概述
//!
//! `agentkit::runtime` 是 Agentkit 框架的运行时编排层，负责：
//! - 调用 LLM Provider 进行对话
//! - 执行工具调用循环（Tool-Calling Loop）
//! - 管理工具注册和执行策略
//! - 提供流式和非流式两种执行模式
//!
//! # 核心组件
//!
//! ## DefaultRuntime
//!
//! 默认的运行时实现，提供完整的 tool-calling 循环：
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use agentkit::runtime::{DefaultRuntime, ToolRegistry};
//! use agentkit::core::provider::LlmProvider;
//! use agentkit::core::agent::types::AgentInput;
//!
//! # async fn example(provider: Arc<dyn LlmProvider>) -> Result<(), Box<dyn std::error::Error>> {
//! // 创建工具注册表
//! let tools = ToolRegistry::new();
//!
//! // 创建运行时
//! let runtime = DefaultRuntime::new(provider, tools)
//!     .with_system_prompt("你是一个有用的助手")
//!     .with_max_steps(10);
//!
//! // 执行对话
//! let input = AgentInput::new("你好");
//! let output = runtime.run(input).await?;
//! # Ok(())
//! # }
//! ```

// Runtime 子模块
pub mod default_runtime;
pub mod loader;
pub mod policy;
pub mod tool_execution;
pub mod tool_registry;
pub mod trace;
pub mod utils;

// 重新导出主要类型
pub use agentkit_core::runtime::{Runtime, RuntimeObserver};
pub use default_runtime::DefaultRuntime;
pub use loader::ToolLoader;
pub use policy::{DefaultToolPolicy, ToolPolicy};
pub use tool_registry::{ToolRegistry, ToolSource};
