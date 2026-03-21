//! Retrieval（语义检索）实现模块
//!
//! # 概述
//!
//! 本模块包含 VectorStore 的具体实现，用于向量存储和相似度搜索。
//!
//! # 支持的 VectorStore 类型
//!
//! | 类型 | 说明 | 使用场景 |
//! |------|------|----------|
//! | [`InMemoryVectorStore`] | 内存向量存储 | 测试、演示、小规模 |
//! | [`ChromaVectorStore`] | Chroma 向量数据库 | 生产环境、大规模检索 |
//! | [`ChromaPersistentStore`] | Chroma 持久化存储 | 需要持久化的场景 |
//!
//! # 使用示例
//!
//! ## InMemoryVectorStore (测试用)
//!
//! ```rust,no_run
//! use agentkit::retrieval::InMemoryVectorStore;
//! use agentkit_core::retrieval::{VectorStore, VectorRecord, VectorQuery};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let store = InMemoryVectorStore::new();
//!
//! store.upsert(vec![
//!     VectorRecord::new("doc1", vec![1.0, 0.0]).with_text("文档 1"),
//! ]).await?;
//!
//! let results = store.search(
//!     VectorQuery::new(vec![1.0, 0.0]).with_top_k(10)
//! ).await?;
//! # Ok(())
//! # }
//! ```

pub mod chroma;
pub mod chroma_persistent;
pub mod in_memory;

pub use chroma::ChromaVectorStore;
pub use chroma_persistent::ChromaPersistentStore;
pub use in_memory::InMemoryVectorStore;
