//! Retrieval（语义检索）抽象模块
//!
//! # 概述
//!
//! Retrieval 用于向量存储和相似度搜索，支持：
//! - 插入/更新向量（upsert）
//! - 删除向量（delete）
//! - 获取向量（get）
//! - 相似度搜索（search）
//! - 清空存储（clear）
//! - 计数（count）
//!
//! 在 core 层，我们只定义抽象接口，不绑定具体实现。
//!
//! # 核心类型
//!
//! ## VectorStore trait
//!
//! [`VectorStore`] trait 定义了向量存储的接口：
//!
//! ```rust,no_run
//! use agentkit_core::retrieval::{VectorStore, VectorRecord, VectorQuery, SearchResult};
//! use agentkit_core::error::ProviderError;
//! use async_trait::async_trait;
//!
//! # async fn example(store: &dyn VectorStore) -> Result<(), ProviderError> {
//! // 插入向量
//! store.upsert(vec![
//!     VectorRecord::new("id1".to_string(), vec![0.1; 128])
//!         .with_text("Hello".to_string()),
//! ]).await?;
//!
//! // 搜索
//! let results = store.search(
//!     VectorQuery::new(vec![0.1; 128])
//!         .with_top_k(10)
//! ).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## VectorRecord
//!
//! 向量记录，包含：
//! - `id`: 记录 ID
//! - `vector`: 向量数据
//! - `text`: 可选的原始文本
//! - `metadata`: 可选元数据
//!
//! ## VectorQuery
//!
//! 向量查询，包含：
//! - `vector`: 查询向量
//! - `top_k`: 返回数量
//! - `filter`: 可选过滤条件
//! - `score_threshold`: 可选分数阈值
//!
//! ## SearchResult
//!
//! 搜索结果，包含：
//! - `id`: 记录 ID
//! - `score`: 相似度分数
//! - `text`: 可选的原始文本
//! - `metadata`: 可选元数据
//! - `vector`: 可选向量数据
//!
//! # 使用示例
//!
//! ## 实现简单 VectorStore
//!
//! ```rust,no_run
//! use agentkit_core::retrieval::{VectorStore, VectorRecord, VectorQuery, SearchResult};
//! use agentkit_core::error::ProviderError;
//! use async_trait::async_trait;
//!
//! struct SimpleVectorStore;
//!
//! #[async_trait]
//! impl VectorStore for SimpleVectorStore {
//!     async fn upsert(&self, records: Vec<VectorRecord>) -> Result<(), ProviderError> {
//!         // 实现插入逻辑
//!         Ok(())
//!     }
//!
//!     async fn delete(&self, ids: Vec<String>) -> Result<(), ProviderError> {
//!         // 实现删除逻辑
//!         Ok(())
//!     }
//!
//!     async fn get(&self, ids: Vec<String>) -> Result<Vec<VectorRecord>, ProviderError> {
//!         // 实现获取逻辑
//!         Ok(vec![])
//!     }
//!
//!     async fn search(&self, query: VectorQuery) -> Result<Vec<SearchResult>, ProviderError> {
//!         // 实现搜索逻辑
//!         Ok(vec![])
//!     }
//!
//!     async fn clear(&self) -> Result<(), ProviderError> {
//!         // 实现清空逻辑
//!         Ok(())
//!     }
//!
//!     async fn count(&self) -> Result<usize, ProviderError> {
//!         // 实现计数逻辑
//!         Ok(0)
//!     }
//! }
//! ```
//!
//! # 常见 VectorStore 实现
//!
//! - Chroma
//! - Pinecone
//! - Weaviate
//! - Milvus
//! - Qdrant
//! - 本地 SQLite + 向量扩展

pub mod r#trait;

/// 重新导出 retrieval 相关 trait
pub use r#trait::*;
