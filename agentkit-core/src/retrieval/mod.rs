//! 向量数据库（Vector Store）模块。
//!
//! 提供向量存储、检索和管理的统一接口，支持语义搜索和 RAG 场景。

pub mod r#trait;

/// 重新导出 trait 和类型。
pub use r#trait::*;
