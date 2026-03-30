//! Agent（智能体）模块
//!
//! # 概述
//!
//! 本模块提供多种预定义的 Agent 类型，每种类型针对不同的使用场景：
//!
//! ## 基础层 Agent（常用）
//!
//! | Agent 类型 | 职责 | 适用场景 |
//! |------------|------|----------|
//! | [`SimpleAgent`] | 简单问答 | 翻译、总结、一次性任务 |
//! | [`ChatAgent`] | 纯对话 | 客服、心理咨询、闲聊 |
//! | [`ToolAgent`] | 工具调用 | 执行具体任务（默认选择） |
//!
//! ## 进阶层 Agent（复杂任务）
//!
//! | Agent 类型 | 职责 | 适用场景 |
//! |------------|------|----------|
//! | [`ReActAgent`] | 推理 + 行动 | 多步推理任务 |
//! | [`PlanAgent`] | 规划执行 | 复杂项目、工作流 |
//! | [`ReflectAgent`] | 反思迭代 | 代码生成、写作 |
//!
//! ## 专家层 Agent（专业场景）
//!
//! | Agent 类型 | 职责 | 适用场景 |
//! |------------|------|----------|
//! | [`CodeAgent`] | 代码专家 | 编程任务 |
//! | [`ResearchAgent`] | 研究分析 | 市场调研、文献综述 |
//!
//! ## 协作层 Agent（多 Agent 系统）
//!
//! | Agent 类型 | 职责 | 适用场景 |
//! |------------|------|----------|
//! | [`SupervisorAgent`] | 主管协调 | 复杂项目协作 |
//! | [`RouterAgent`] | 路由分发 | 多技能系统 |
//!
//! # 快速开始
//!
//! ## 选择 Agent 类型
//!
//! ```text
//! 是否需要工具调用？
//!   │
//!   ├─ 否 ──► 是否需要多轮对话历史？
//!   │           │
//!   │           ├─ 否 ──► SimpleAgent（简单问答）
//!   │           │
//!   │           └─ 是 ──► ChatAgent（对话）
//!   │
//!   └─ 是 ──► 需要多少步？
//!             │
//!             ├─ 1-2 步 ──► ToolAgent（默认选择）
//!             │
//!             ├─ 3-5 步 ──► ReActAgent（推理 + 行动）
//!             │
//!             ├─ 5+ 步 ──► PlanAgent（规划执行）
//!             │
//!             └─ 需要高质量 ──► ReflectAgent（反思迭代）
//! ```
//!
//! ## 使用示例
//!
//! ### SimpleAgent - 简单问答
//!
//! ```rust,no_run
//! use agentkit::agent::SimpleAgent;
//! use agentkit::provider::OpenAiProvider;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = OpenAiProvider::from_env()?;
//!
//! let agent = SimpleAgent::builder()
//!     .provider(provider)
//!     .model("gpt-4o-mini")
//!     .system_prompt("你是一个翻译助手")
//!     .temperature(0.3)
//!     .build();
//!
//! let output = agent.run("把'Hello'翻译成中文").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### ChatAgent - 多轮对话
//!
//! ```rust,no_run
//! use agentkit::agent::ChatAgent;
//! use agentkit::provider::OpenAiProvider;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = OpenAiProvider::from_env()?;
//!
//! let agent = ChatAgent::builder()
//!     .provider(provider)
//!     .model("gpt-4o-mini")
//!     .system_prompt("你是友好的助手")
//!     .with_conversation(true)
//!     .build();
//!
//! agent.run("你好").await?;
//! agent.run("今天天气怎么样？").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### ToolAgent - 工具调用（推荐默认）
//!
//! ```rust,no_run
//! use agentkit::agent::ToolAgent;
//! use agentkit::provider::OpenAiProvider;
//! use agentkit::tools::ShellTool;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = OpenAiProvider::from_env()?;
//!
//! let agent = ToolAgent::builder()
//!     .provider(provider)
//!     .model("gpt-4o-mini")
//!     .system_prompt("你是有用的助手")
//!     .tool(ShellTool)
//!     .build();
//!
//! let output = agent.run("帮我列出当前目录的文件").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### ReActAgent - 推理 + 行动
//!
//! ```rust,no_run
//! use agentkit::agent::ReActAgent;
//! use agentkit::provider::OpenAiProvider;
//! use agentkit::tools::{ShellTool, FileReadTool};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = OpenAiProvider::from_env()?;
//!
//! let agent = ReActAgent::builder()
//!     .provider(provider)
//!     .model("gpt-4o-mini")
//!     .tools(vec![ShellTool, FileReadTool])
//!     .max_steps(15)
//!     .build();
//!
//! let output = agent.run("帮我分析这个项目的代码结构").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### ReflectAgent - 反思迭代
//!
//! ```rust,no_run
//! use agentkit::agent::ReflectAgent;
//! use agentkit::provider::OpenAiProvider;
//! use agentkit::tools::FileWriteTool;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = OpenAiProvider::from_env()?;
//!
//! let agent = ReflectAgent::builder()
//!     .provider(provider)
//!     .model("gpt-4o-mini")
//!     .tool(FileWriteTool)
//!     .max_iterations(3)
//!     .build();
//!
//! let output = agent.run("帮我写一个快速排序算法").await?;
//! # Ok(())
//! # }
//! ```
//!
//! # 架构设计
//!
//! ## 决策与执行分离
//!
//! ```
//! ┌─────────────────────────────────────────────────────────┐
//! │                      Agent Trait                        │
//! │  ┌─────────────────┐    ┌─────────────────────────┐    │
//! │  │   决策层        │    │   执行层（默认实现）     │    │
//! │  │   (think)       │    │   (run/run_stream)      │    │
//! │  │   负责"做什么"   │    │   负责"怎么做"          │    │
//! │  └─────────────────┘    └─────────────────────────┘    │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! - **决策层**：每个 Agent 类型有不同的思考策略
//! - **执行层**：所有 Agent 共享相同的执行能力（[`DefaultExecution`]）
//!
//! ## 共享执行能力
//!
//! 所有 Agent 类型都组合了 [`DefaultExecution`] 来获得执行能力：
//!
//! - 工具调用循环
//! - 流式执行
//! - 并发控制
//! - 策略检查
//! - 观测器协议
//!
//! # 向后兼容
//!
//! `DefaultAgent` 作为 [`ToolAgent`] 的别名保留，但已标记为 deprecated：
//!
//! ```rust
//! // 旧代码（仍然可用）
//! use agentkit::agent::DefaultAgent;
//!
//! // 新代码（推荐）
//! use agentkit::agent::ToolAgent;
//! ```

// 执行能力模块
pub mod execution;

// 工具相关模块
pub mod policy;
pub mod tool_execution;
pub mod tool_registry;

// 结构化数据提取模块
pub mod extractor;

// 基础层 Agent
pub mod chat;
pub mod simple;
pub mod tool;

// 进阶层 Agent
pub mod react;
pub mod reflect;

// 重新导出主要类型
pub use execution::DefaultExecution;
pub use policy::{DefaultToolPolicy, ToolPolicy};
pub use tool_registry::{ToolRegistry, ToolSource, ToolWrapper};

// 基础层
pub use chat::{ChatAgent, ChatAgentBuilder};
pub use simple::{SimpleAgent, SimpleAgentBuilder};
pub use tool::{ToolAgent, ToolAgentBuilder};

// 进阶层
pub use react::{ReActAgent, ReActAgentBuilder};
pub use reflect::{ReflectAgent, ReflectAgentBuilder};

// Extractor
pub use extractor::{ExtractionError, ExtractionResponse, Extractor, ExtractorBuilder, TokenUsage};

// 向后兼容
#[deprecated(
    since = "0.2.0",
    note = "DefaultAgent 已重命名为 ToolAgent，请使用 ToolAgent"
)]
pub use tool::ToolAgent as DefaultAgent;
