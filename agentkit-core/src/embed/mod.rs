//! 向量嵌入（Embedding）模块。
//!
//! 提供文本向量化的抽象接口，用于语义搜索、RAG 等场景。

pub mod r#trait;

/// 重新导出 trait 和工具函数。
pub use r#trait::*;
