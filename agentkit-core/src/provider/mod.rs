//! LLM Provider 抽象模块
//!
//! # 概述
//!
//! 本模块定义了 LLM Provider 的抽象接口，用于与大型语言模型进行交互。
//! Provider 是 Agent 与 LLM 之间的桥梁，负责：
//! - 发送聊天请求
//! - 接收聊天响应
//! - 支持流式输出
//! - 处理工具调用
//!
//! # 核心类型
//!
//! ## LlmProvider trait
//!
//! [`LlmProvider`] 是所有 Provider 必须实现的接口：
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
//!         // 实现非流式聊天逻辑
//!         unimplemented!()
//!     }
//!
//!     fn stream_chat(&self, request: ChatRequest) -> Result<BoxStream<'static, Result<ChatStreamChunk, ProviderError>>, ProviderError> {
//!         // 实现流式聊天逻辑
//!         unimplemented!()
//!     }
//! }
//! ```
//!
//! ## ChatRequest
//!
//! 聊天请求，包含：
//! - 消息历史（`messages`）
//! - 模型名称（`model`）
//! - 工具定义（`tools`）
//! - 温度参数（`temperature`）
//! - 最大 token 数（`max_tokens`）
//! - 响应格式（`response_format`）
//!
//! ## ChatResponse
//!
//! 聊天响应，包含：
//! - 助手消息（`message`）
//! - 工具调用列表（`tool_calls`）
//! - 使用统计（`usage`）
//! - 完成原因（`finish_reason`）
//!
//! ## ChatMessage
//!
//! 聊天消息，包含：
//! - 角色（`role`: System/User/Assistant/Tool）
//! - 内容（`content`）
//! - 名称（`name`，可选）
//!
//! ## Role
//!
//! 消息角色枚举：
//! - `System`: 系统提示词
//! - `User`: 用户消息
//! - `Assistant`: 助手回复
//! - `Tool`: 工具结果
//!
//! # 使用示例
//!
//! ## 非流式聊天
//!
//! ```rust,no_run
//! use agentkit_core::provider::{LlmProvider, types::*};
//!
//! # async fn example(provider: &dyn LlmProvider) -> Result<(), Box<dyn std::error::Error>> {
//! let request = ChatRequest {
//!     messages: vec![
//!         ChatMessage {
//!             role: Role::System,
//!             content: "你是一个有用的助手".to_string(),
//!             name: None,
//!         },
//!         ChatMessage {
//!             role: Role::User,
//!             content: "你好".to_string(),
//!             name: None,
//!         },
//!     ],
//!     model: Some("gpt-4".to_string()),
//!     tools: None,
//!     temperature: Some(0.7),
//!     max_tokens: None,
//!     response_format: None,
//!     metadata: None,
//! };
//!
//! let response = provider.chat(request).await?;
//! println!("助手回复：{}", response.message.content);
//! # Ok(())
//! # }
//! ```
//!
//! ## 流式聊天
//!
//! ```rust,no_run
//! use agentkit_core::provider::{LlmProvider, types::*};
//! use futures_util::StreamExt;
//!
//! # async fn example(provider: &dyn LlmProvider) -> Result<(), Box<dyn std::error::Error>> {
//! let request = ChatRequest {
//!     messages: vec![
//!         ChatMessage {
//!             role: Role::User,
//!             content: "讲个故事".to_string(),
//!             name: None,
//!         },
//!     ],
//!     model: Some("gpt-4".to_string()),
//!     tools: None,
//!     temperature: None,
//!     max_tokens: None,
//!     response_format: None,
//!     metadata: None,
//! };
//!
//! let mut stream = provider.stream_chat(request)?;
//! while let Some(chunk) = stream.next().await {
//!     let chunk = chunk?;
//!     if let Some(delta) = chunk.delta {
//!         print!("{}", delta);
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # 实现 Provider
//!
//! 实现自定义 Provider 需要：
//!
//! 1. 实现 `LlmProvider` trait
//! 2. 处理 HTTP 请求（通常使用 reqwest）
//! 3. 解析响应数据
//! 4. 处理错误情况
//!
//! ## OpenAI Provider 示例
//!
//! ```rust,no_run
//! use agentkit_core::provider::{LlmProvider, types::*};
//! use agentkit_core::error::ProviderError;
//! use async_trait::async_trait;
//! use futures_util::stream::BoxStream;
//! use reqwest::Client;
//! use serde_json::json;
//!
//! struct OpenAiProvider {
//!     client: Client,
//!     api_key: String,
//!     base_url: String,
//! }
//!
//! #[async_trait]
//! impl LlmProvider for OpenAiProvider {
//!     async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
//!         // 构建请求体
//!         let body = json!({
//!             "model": request.model.unwrap_or_default(),
//!             "messages": request.messages,
//!         });
//!
//!         // 发送请求
//!         let response = self.client
//!             .post(format!("{}/chat/completions", self.base_url))
//!             .header("Authorization", format!("Bearer {}", self.api_key))
//!             .json(&body)
//!             .send()
//!             .await
//!             .map_err(|e| ProviderError::Message(e.to_string()))?;
//!
//!         // 解析响应
//!         let data: serde_json::Value = response.json().await
//!             .map_err(|e| ProviderError::Message(e.to_string()))?;
//!
//!         // 提取结果
//!         // ...
//!
//!         unimplemented!()
//!     }
//!
//!     fn stream_chat(&self, request: ChatRequest) -> Result<BoxStream<'static, Result<ChatStreamChunk, ProviderError>>, ProviderError> {
//!         // 实现流式逻辑
//!         unimplemented!()
//!     }
//! }
//! ```
//!
//! # 错误处理
//!
//! Provider 错误通常包括：
//! - 网络错误（连接超时、DNS 解析失败）
//! - API 错误（认证失败、限流、服务不可用）
//! - 解析错误（响应格式不正确）
//!
//! 所有错误都通过 [`ProviderError`] 返回。

/// Provider trait 定义
pub mod r#trait;

/// Provider 相关类型定义
pub mod types;

/// 重新导出 provider 相关 trait，方便 `agentkit_core::provider::*` 使用
pub use r#trait::*;

/// 重新导出 provider 相关类型，方便使用
pub use types::*;
