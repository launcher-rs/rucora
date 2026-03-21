//! Embedding（向量嵌入）抽象模块
//!
//! # 概述
//!
//! Embedding 用于将文本转换为向量表示，支持：
//! - 单条文本嵌入（embed）
//! - 批量文本嵌入（embed_batch）
//! - 维度查询（embedding_dim）
//!
//! 在 core 层，我们只定义抽象接口，不绑定具体实现。
//!
//! # 核心类型
//!
//! ## EmbeddingProvider trait
//!
//! [`EmbeddingProvider`] trait 定义了向量嵌入的接口：
//!
//! ```rust,no_run
//! use agentkit_core::embed::EmbeddingProvider;
//! use agentkit_core::error::ProviderError;
//! use async_trait::async_trait;
//!
//! # async fn example(provider: &dyn EmbeddingProvider) -> Result<(), ProviderError> {
//! // 单条嵌入
//! let vector = provider.embed("Hello, world!").await?;
//!
//! // 批量嵌入
//! let vectors = provider.embed_batch(&[
//!     "Hello".to_string(),
//!     "World".to_string(),
//! ]).await?;
//!
//! // 查询维度
//! if let Some(dim) = provider.embedding_dim() {
//!     println!("向量维度：{}", dim);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # 使用示例
//!
//! ## 实现简单 Embedding Provider
//!
//! ```rust,no_run
//! use agentkit_core::embed::EmbeddingProvider;
//! use agentkit_core::error::ProviderError;
//! use async_trait::async_trait;
//!
//! struct SimpleEmbeddingProvider;
//!
//! #[async_trait]
//! impl EmbeddingProvider for SimpleEmbeddingProvider {
//!     async fn embed(&self, text: &str) -> Result<Vec<f32>, ProviderError> {
//!         // 实现嵌入逻辑（这里返回伪向量）
//!         Ok(vec![0.1; 128])
//!     }
//!
//!     async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, ProviderError> {
//!         let mut vectors = Vec::with_capacity(texts.len());
//!         for _ in texts {
//!             vectors.push(self.embed("dummy").await?);
//!         }
//!         Ok(vectors)
//!     }
//!
//!     fn embedding_dim(&self) -> Option<usize> {
//!         Some(128)
//!     }
//! }
//! ```
//!
//! # 常见 Embedding Provider
//!
//! - OpenAI Embedding API
//! - Ollama Embedding
//! - Sentence Transformers（本地）
//! - 其他第三方服务

pub mod r#trait;

/// 重新导出 embed 相关 trait
pub use r#trait::*;
