//! 向量存储检索模块
//!
//! 提供向量存储和相似性搜索能力，支持多种后端：
//! - `chroma`: ChromaDB 向量存储（内存版）
//! - `chroma_persistent`: ChromaDB 向量存储（持久化版）
//! - `in_memory`: 内存向量存储
//! - `memory`: 内存向量存储（另一种实现）
//! - `qdrant`: Qdrant 向量存储

pub mod chroma;
/// ChromaDB 向量存储（持久化版）
pub mod chroma_persistent;
/// 内存向量存储
pub mod in_memory;
/// 内存向量存储（另一种实现）
pub mod memory;
/// Qdrant 向量存储
pub mod qdrant;
