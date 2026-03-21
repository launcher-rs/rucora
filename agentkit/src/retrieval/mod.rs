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
//! | [`ChromaVectorStore`] | Chroma 向量数据库 | 生产环境、大规模检索 |
//! | [`ChromaPersistentStore`] | Chroma 持久化存储 | 需要持久化的场景 |
//!
//! # 使用示例
//!
//! ## ChromaVectorStore
//!
//! ```rust,no_run
//! use agentkit::retrieval::ChromaVectorStore;
//! use agentkit_core::retrieval::{VectorStore, VectorRecord, VectorQuery};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // 从环境变量加载配置
//! let store = ChromaVectorStore::from_env()?;
//!
//! // 插入向量
//! store.upsert(vec![
//!     VectorRecord::new("doc1".to_string(), vec![0.1; 128])
//!         .with_text("Hello, World!".to_string()),
//! ]).await?;
//!
//! // 搜索
//! let results = store.search(
//!     VectorQuery::new(vec![0.1; 128])
//!         .with_top_k(10)
//! ).await?;
//!
//! for result in results {
//!     println!("ID: {}, Score: {}", result.id, result.score);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # 环境变量
//!
//! | 变量名 | 说明 | 默认值 |
//! |--------|------|--------|
//! | `CHROMA_URL` | Chroma 服务地址 | `http://localhost:8000` |
//! | `CHROMA_COLLECTION` | 集合名称 | `default` |
//! | `CHROMA_TENANT` | 租户名称 | `default_tenant` |
//! | `CHROMA_DATABASE` | 数据库名称 | `default_database` |
//!
//! # RAG 集成
//!
//! Retrieval 模块与 RAG 管线紧密集成：
//!
//! ```rust,no_run
//! use agentkit::retrieval::ChromaVectorStore;
//! use agentkit::embed::OpenAiEmbedding;
//! use agentkit::rag::{index_text, retrieve};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let embedder = OpenAiEmbedding::from_env()?;
//! let store = ChromaVectorStore::from_env()?;
//!
//! // 索引文本
//! let chunks = index_text(
//!     &embedder,
//!     &store,
//!     "doc1",
//!     "这是一段长文本...",
//!     500,  // 每块最大字符数
//!     50,   // 重叠字符数
//! ).await?;
//!
//! // 检索相关片段
//! let citations = retrieve(
//!     &embedder,
//!     &store,
//!     "查询问题",
//!     5,  // top_k
//! ).await?;
//!
//! for cite in citations {
//!     println!("引用：{}", cite.text.unwrap_or_default());
//! }
//! # Ok(())
//! # }
//! ```

pub mod chroma;
pub mod chroma_persistent;

pub use chroma::ChromaVectorStore;
pub use chroma_persistent::ChromaPersistentStore;
