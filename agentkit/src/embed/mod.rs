//! Embedding（向量嵌入）实现模块
//!
//! # 概述
//!
//! 本模块包含 Embedding Provider 的具体实现，用于将文本转换为向量表示。
//!
//! # 支持的 Embedding Provider
//!
//! | Provider | 说明 | 使用场景 |
//! |----------|------|----------|
//! | [`OpenAiEmbedding`] | OpenAI Embedding API | 高质量、生产环境 |
//! | [`OllamaEmbedding`] | Ollama Embedding | 本地部署、免费 |
//! | [`CachedEmbeddingProvider`] | 带缓存的 Provider | 减少重复计算 |
//!
//! # 使用示例
//!
//! ## OpenAiEmbedding
//!
//! ```rust,no_run
//! use agentkit::embed::OpenAiEmbedding;
//! use agentkit_core::embed::EmbeddingProvider;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let embedder = OpenAiEmbedding::from_env()?;
//!
//! // 单条嵌入
//! let vector = embedder.embed("Hello, World!").await?;
//! println!("向量维度：{}", vector.len());
//!
//! // 批量嵌入
//! let vectors = embedder.embed_batch(&[
//!     "Hello".to_string(),
//!     "World".to_string(),
//! ]).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## OllamaEmbedding
//!
//! ```rust,no_run
//! use agentkit::embed::OllamaEmbedding;
//! use agentkit_core::embed::EmbeddingProvider;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let embedder = OllamaEmbedding::from_env();
//!
//! let vector = embedder.embed("Hello").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## CachedEmbeddingProvider
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use agentkit::embed::{OpenAiEmbedding, CachedEmbeddingProvider};
//! use agentkit_core::embed::EmbeddingProvider;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let inner = OpenAiEmbedding::from_env()?;
//!
//! // 包装缓存层
//! let cached = CachedEmbeddingProvider::new(inner);
//!
//! // 第一次调用会计算
//! let v1 = cached.embed("Hello").await?;
//!
//! // 第二次调用会从缓存读取
//! let v2 = cached.embed("Hello").await?;
//! # Ok(())
//! # }
//! ```
//!
//! # 环境变量
//!
//! | 变量名 | 说明 | 示例 |
//! |--------|------|------|
//! | `OPENAI_API_KEY` | OpenAI API Key | `sk-...` |
//! | `OPENAI_BASE_URL` | OpenAI Base URL | `https://api.openai.com/v1` |
//! | `OLLAMA_BASE_URL` | Ollama Base URL | `http://localhost:11434` |
//!
//! # 与 Retrieval 集成
//!
//! Embedding 通常与 Retrieval 一起使用：
//!
//! ```rust,no_run
//! use agentkit::embed::OpenAiEmbedding;
//! use agentkit::retrieval::ChromaVectorStore;
//! use agentkit_core::retrieval::{VectorStore, VectorRecord};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let embedder = OpenAiEmbedding::from_env()?;
//! let store = ChromaVectorStore::from_env()?;
//!
//! // 嵌入并存储
//! let text = "Hello, World!";
//! let vector = embedder.embed(text).await?;
//!
//! store.upsert(vec![
//!     VectorRecord::new("doc1".to_string(), vector)
//!         .with_text(text.to_string()),
//! ]).await?;
//! # Ok(())
//! # }
//! ```

pub mod cache;
pub mod ollama;
pub mod openai;

pub use cache::CachedEmbeddingProvider;
pub use ollama::OllamaEmbeddingProvider;
pub use openai::OpenAiEmbeddingProvider;

// 类型别名，便于使用
/// OpenAI Embedding 的别名
pub type OpenAiEmbedding = OpenAiEmbeddingProvider;
/// Ollama Embedding 的别名
pub type OllamaEmbedding = OllamaEmbeddingProvider;
