//! Embedding Provider 实现。
//!
//! 本模块包含各种嵌入提供者的具体实现：
//! - OpenAI Embedding Provider (text-embedding-ada-002, text-embedding-3-*)
//! - Ollama Embedding Provider (本地模型，如 nomic-embed-text)
//!
//! 使用示例：
//! ```rust,ignore
//! use agentkit::embed::{OpenAiEmbeddingProvider, EmbeddingProvider};
//!
//! let provider = OpenAiEmbeddingProvider::from_env().unwrap();
//! let embedding = provider.embed("Hello world").await.unwrap();
//! ```

pub mod ollama;
pub mod openai;

/// 重新导出 core 的 EmbeddingProvider trait。
pub use agentkit_core::embed::EmbeddingProvider;

/// 重新导出具体实现。
pub use ollama::OllamaEmbeddingProvider;
pub use openai::OpenAiEmbeddingProvider;
