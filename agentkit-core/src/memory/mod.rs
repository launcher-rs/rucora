//! Memory（记忆）抽象模块
//!
//! # 概述
//!
//! Memory（记忆）用于存储和检索长期记忆，支持：
//! - 添加记忆（add）
//! - 检索记忆（query）
//! - 记忆分类（core/daily/conversation 等）
//!
//! 在 core 层，我们只定义抽象接口，不绑定具体实现。
//!
//! # 核心类型
//!
//! ## Memory trait
//!
//! [`Memory`] trait 定义了记忆存储和检索的接口：
//!
//! ```rust,no_run
//! use agentkit_core::memory::{Memory, MemoryItem, MemoryQuery};
//! use agentkit_core::error::MemoryError;
//! use async_trait::async_trait;
//!
//! # async fn example(memory: &dyn Memory) -> Result<(), MemoryError> {
//! // 添加记忆
//! memory.add(MemoryItem {
//!     id: "user_name".to_string(),
//!     content: "Alice".to_string(),
//!     metadata: None,
//! }).await?;
//!
//! // 检索记忆
//! let results = memory.query(MemoryQuery {
//!     text: "user".to_string(),
//!     limit: 10,
//! }).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## MemoryItem
//!
//! 记忆项，包含：
//! - `id`: 记忆 ID
//! - `content`: 记忆内容
//! - `metadata`: 可选元数据
//!
//! ## MemoryQuery
//!
//! 记忆查询，包含：
//! - `text`: 查询文本
//! - `limit`: 返回数量限制
//!
//! # 使用示例
//!
//! ## 实现简单记忆
//!
//! ```rust,no_run
//! use agentkit_core::memory::{Memory, MemoryItem, MemoryQuery};
//! use agentkit_core::error::MemoryError;
//! use async_trait::async_trait;
//!
//! struct SimpleMemory;
//!
//! #[async_trait]
//! impl Memory for SimpleMemory {
//!     async fn add(&self, item: MemoryItem) -> Result<(), MemoryError> {
//!         // 实现添加逻辑
//!         Ok(())
//!     }
//!
//!     async fn query(&self, query: MemoryQuery) -> Result<Vec<MemoryItem>, MemoryError> {
//!         // 实现查询逻辑
//!         Ok(vec![])
//!     }
//! }
//! ```
//!
//! ## 记忆分类
//!
//! ```rust
//! use agentkit_core::memory::MemoryItem;
//! use serde_json::json;
//!
//! // Core 记忆（永久）
//! let core = MemoryItem {
//!     id: "core:user_name".to_string(),
//!     content: "Alice".to_string(),
//!     metadata: Some(json!({"category": "core"})),
//! };
//!
//! // Daily 记忆（会话）
//! let daily = MemoryItem {
//!     id: "daily:last_topic".to_string(),
//!     content: "Rust 编程".to_string(),
//!     metadata: Some(json!({"category": "daily"})),
//! };
//! ```

pub mod r#trait;
pub mod types;

/// 重新导出 memory 相关 trait
pub use r#trait::*;

/// 重新导出 memory 相关类型
pub use types::*;
