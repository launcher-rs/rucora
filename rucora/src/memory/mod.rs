//! Memory（记忆）实现模块
//!
//! # 概述
//!
//! 本模块包含 Memory 的具体实现，用于存储和检索长期记忆。
//!
//! # 支持的 Memory 类型
//!
//! | 类型 | 说明 | 使用场景 |
//! |------|------|----------|
//! | [`InMemoryMemory`] | 进程内记忆存储 | 测试、临时会话 |
//! | [`FileMemory`] | 文件记忆存储 | 持久化、简单场景 |
//!
//! # 使用示例
//!
//! ## InMemoryMemory
//!
//! ```rust,no_run
//! use rucora::memory::InMemoryMemory;
//! use rucora_core::memory::{Memory, MemoryItem};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let memory = InMemoryMemory::new();
//!
//! // 添加记忆
//! memory.add(MemoryItem {
//!     id: "user_name".to_string(),
//!     content: "Alice".to_string(),
//!     metadata: None,
//! }).await?;
//!
//! // 检索记忆
//! let results = memory.query(rucora_core::memory::MemoryQuery {
//!     text: "user".to_string(),
//!     limit: 10,
//! }).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## FileMemory
//!
//! ```rust,no_run
//! use rucora::memory::FileMemory;
//! use rucora_core::memory::{Memory, MemoryItem};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let memory = FileMemory::new("memory.json");
//!
//! // 添加记忆（会自动保存到文件）
//! memory.add(MemoryItem {
//!     id: "preference".to_string(),
//!     content: "喜欢 Rust".to_string(),
//!     metadata: None,
//! }).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # 记忆分类
//!
//! 记忆可以通过 ID 前缀进行分类：
//!
//! - `core:`: 永久记忆（用户偏好、基本信息）
//! - `daily:`: 会话记忆（当天对话主题）
//! - `conversation:`: 对话上下文（最近的对话内容）
//!
//! ```rust
//! use rucora_core::memory::MemoryItem;
//!
//! // 永久记忆
//! let core = MemoryItem {
//!     id: "core:user_name".to_string(),
//!     content: "Alice".to_string(),
//!     metadata: None,
//! };
//!
//! // 会话记忆
//! let daily = MemoryItem {
//!     id: "daily:last_topic".to_string(),
//!     content: "Rust 编程".to_string(),
//!     metadata: None,
//! };
//! ```

pub mod file;
pub mod in_memory;

pub use file::FileMemory;
pub use in_memory::InMemoryMemory;
