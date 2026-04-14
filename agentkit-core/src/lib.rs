//! agentkit-core - Agentkit 核心抽象层
//!
//! # 概述
//!
//! `agentkit-core` 是 Agentkit 框架的核心抽象层，只包含 trait、类型定义、错误类型和事件模型。
//! 它不包含任何具体实现（如 Provider、Tool、Runtime 的具体实现），便于第三方实现与长期兼容。
//!
//! # 设计目标
//!
//! - **接口与实现解耦**: 只定义 trait 与核心类型，第三方可以只依赖 core 自己实现 Provider/Tool/VectorStore 等
//! - **稳定抽象**: 提供稳定的接口，避免频繁变更影响上层实现
//! - **最小依赖**: 不绑定具体实现，保持轻量级
//!
//! # 核心模块
//!
//! ## Agent（智能体）
//!
//! Agent 是智能体的核心，负责：
//! - 思考、决策、规划
//! - 接收输入（消息、任务、上下文）
//! - 返回决策结果
//!
//! 相关类型：
//! - [`agent::types::AgentInput`][]: Agent 输入类型
//! - [`agent::types::AgentOutput`][]: Agent 输出类型
//! - [`agent::Agent`][]: Agent trait
//! - [`agent::AgentExecutor`][]: Agent 执行器 trait
//!
//! ## Channel（通信渠道）
//!
//! 统一的事件模型，用于在运行时、工具、技能之间传递消息。
//!
//! 相关类型：
//! - [`channel::types::ChannelEvent`][]: 统一事件类型
//! - [`channel::types::TokenDeltaEvent`][]: Token 流式输出事件
//! - [`channel::types::DebugEvent`][]: 调试事件
//! - [`channel::types::ErrorEvent`][]: 错误事件
//!
//! ## Provider（LLM 提供者）
//!
//! LLM Provider 抽象，定义与大型语言模型交互的接口。
//!
//! 相关 trait：
//! - [`provider::LlmProvider`][]: LLM Provider trait
//!
//! 相关类型：
//! - [`provider::types::ChatRequest`][]: 聊天请求
//! - [`provider::types::ChatResponse`][]: 聊天响应
//! - [`provider::types::ChatMessage`][]: 聊天消息
//! - [`provider::types::Role`][]: 消息角色（System/User/Assistant/Tool）
//!
//! ## Tool（工具）
//!
//! 工具是可以被 Agent 调用的"可执行能力"，例如：读取文件、访问网页、查询数据库等。
//!
//! 相关 trait：
//! - [`tool::Tool`][]: Tool trait
//!
//! 相关类型：
//! - [`tool::types::ToolDefinition`][]: 工具定义
//! - [`tool::types::ToolCall`][]: 工具调用
//! - [`tool::types::ToolResult`][]: 工具结果
//! - [`tool::ToolCategory`][]: 工具分类
//!
//! ## Skill（技能）
//!
//! 技能是对 Tool/Provider/Memory 的组合封装，提供更高层次的抽象。
//!
//! 相关 trait：
//! - [`skill::Skill`][]: Skill trait
//!
//! 相关类型：
//! - [`skill::types::SkillContext`][]: 技能上下文
//! - [`skill::types::SkillOutput`][]: 技能输出
//!
//! ## Memory（记忆）
//!
//! 记忆抽象，用于添加和检索长期记忆。
//!
//! 相关 trait：
//! - [`memory::Memory`][]: Memory trait
//!
//! 相关类型：
//! - [`memory::types::MemoryItem`][]: 记忆项
//! - [`memory::types::MemoryQuery`][]: 记忆查询
//!
//! ## Embedding（向量嵌入）
//!
//! 向量嵌入抽象，用于文本向量化。
//!
//! 相关 trait：
//! - [`embed::EmbeddingProvider`][]: Embedding Provider trait
//!
//! ## Retrieval（语义检索）
//!
//! 向量存储与相似度搜索抽象。
//!
//! 相关 trait：
//! - [`retrieval::VectorStore`][]: VectorStore trait
//!
//! 相关类型：
//! - [`retrieval::VectorRecord`][]: 向量记录
//! - [`retrieval::VectorQuery`][]: 向量查询
//! - [`retrieval::SearchResult`][]: 搜索结果
//!
//! # 使用示例
//!
//! ## 实现自定义 Provider
//!
//! ```rust,no_run
//! use agentkit_core::provider::{LlmProvider, types::*};
//! use agentkit_core::error::ProviderError;
//! use async_trait::async_trait;
//! use futures_util::stream::BoxStream;
//!
//! struct MyProvider;
//!
//! #[async_trait]
//! impl LlmProvider for MyProvider {
//!     async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
//!         // 实现聊天逻辑
//!         Ok(ChatResponse {
//!             message: ChatMessage {
//!                 role: Role::Assistant,
//!                 content: "Hello!".to_string(),
//!                 name: None,
//!             },
//!             tool_calls: vec![],
//!             usage: None,
//!             finish_reason: None,
//!         })
//!     }
//!
//!     fn stream_chat(&self, request: ChatRequest) -> Result<BoxStream<'static, Result<ChatStreamChunk, ProviderError>>, ProviderError> {
//!         // 实现流式聊天逻辑
//!         unimplemented!()
//!     }
//! }
//! ```
//!
//! ## 实现自定义 Tool
//!
//! ```rust,no_run
//! use agentkit_core::tool::{Tool, ToolCategory};
//! use agentkit_core::error::ToolError;
//! use async_trait::async_trait;
//! use serde_json::{Value, json};
//!
//! struct EchoTool;
//!
//! #[async_trait]
//! impl Tool for EchoTool {
//!     fn name(&self) -> &str {
//!         "echo"
//!     }
//!
//!     fn description(&self) -> Option<&str> {
//!         Some("回显输入内容")
//!     }
//!
//!     fn categories(&self) -> &'static [ToolCategory] {
//!         &[ToolCategory::Basic]
//!     }
//!
//!     fn input_schema(&self) -> Value {
//!         json!({
//!             "type": "object",
//!             "properties": {
//!                 "text": {"type": "string", "description": "要回显的文本"}
//!             },
//!             "required": ["text"]
//!         })
//!     }
//!
//!     async fn call(&self, input: Value) -> Result<Value, ToolError> {
//!         let text = input.get("text").and_then(|v| v.as_str()).unwrap_or("");
//!         Ok(json!({"echo": text}))
//!     }
//! }
//! ```
//!
//! # 错误处理
//!
//! 统一的错误类型定义：
//!
//! - [`ProviderError`][]: Provider 错误
//! - [`ToolError`][]: Tool 错误
//! - [`SkillError`][]: Skill 错误
//! - [`AgentError`][]: Agent/Runtime 错误
//! - [`MemoryError`][]: Memory 错误
//! - [`ChannelError`][]: Channel 错误
//!
//! 所有错误类型都实现了 [`error::DiagnosticError`] trait，提供结构化诊断信息。
//!
//! # 事件模型
//!
//! [`channel::types::ChannelEvent`] 是统一的事件类型，支持：
//!
//! - `Message`: 对话消息事件
//! - `TokenDelta`: Token 流式输出事件
//! - `ToolCall`: 工具调用事件
//! - `ToolResult`: 工具结果事件
//! - `Skill`: 技能相关事件
//! - `Memory`: 记忆相关事件
//! - `Debug`: 调试事件
//! - `Error`: 错误事件
//! - `Raw`: 原始事件（用于透传）

/// Agent 核心抽象（运行入口）
pub mod agent;

/// 通信渠道抽象（事件发送与订阅）
pub mod channel;

/// 向量嵌入抽象（文本向量化）
pub mod embed;

/// 记忆抽象（添加与检索）
pub mod memory;

/// LLM 提供者抽象（对话/流式对话等）
pub mod provider;

/// 语义检索抽象（向量存储与相似度搜索）
pub mod retrieval;

/// 技能抽象（更高层的可复用能力单元）
pub mod skill;

/// 工具抽象（名称、输入 schema、执行）
pub mod tool;

/// 统一错误类型定义
pub mod error;

/// 结构化错误分类器
pub mod error_classifier;

/// Prompt 注入防护扫描器
pub mod injection_guard;

// 重新导出常用类型
pub use agent::types::{AgentInput, AgentOutput};
pub use channel::types::ChannelEvent;
pub use error::{AgentError, ChannelError, MemoryError, ProviderError, SkillError, ToolError};
pub use error_classifier::{ClassifiedError, ErrorClassifier, ErrorContext, FailoverReason};
pub use injection_guard::{ContentScannable, InjectionGuard, ScanResult, Threat, ThreatType};
pub use provider::LlmProvider;
pub use tool::Tool;
