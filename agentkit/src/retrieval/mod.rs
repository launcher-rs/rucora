//! Retrieval（语义检索）实现。
//!
//! 本模块包含各种向量数据库的具体实现：
//! - 内存存储：基于 HashMap 的简单实现，适合开发和测试
//! - Qdrant：生产级向量数据库，支持分布式部署
//! - Chroma HTTP：远程 Chroma 服务器客户端
//! - Chroma Persistent：本地嵌入式持久化存储
//!
//! 使用示例：
//! ```rust,ignore
//! use agentkit::retrieval::{InMemoryVectorStore, VectorStore, VectorRecord};
//!
//! let store = InMemoryVectorStore::new();
//! let record = VectorRecord::new("id1", vec![0.1, 0.2, 0.3])
//!     .with_text("Hello world");
//! store.upsert(vec![record]).await.unwrap();
//! ```

pub mod chroma;
pub mod chroma_persistent;
pub mod memory;
pub mod qdrant;

/// 重新导出 core 的 Retrieval trait 和类型。
pub use agentkit_core::retrieval::*;

/// 重新导出具体实现。
pub use chroma::ChromaVectorStore;
pub use chroma_persistent::ChromaPersistentStore;
pub use memory::InMemoryVectorStore;
pub use qdrant::QdrantVectorStore;
