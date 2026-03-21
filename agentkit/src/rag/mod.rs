//! RAG（检索增强生成）管线模块
//!
//! # 概述
//!
//! 本模块提供 RAG（Retrieval-Augmented Generation）的最小管线实现，包括：
//! - 文本分块（chunking）
//! - 索引（indexing）
//! - 检索（retrieval）
//! - 引用生成（citation）
//!
//! # RAG 流程
//!
//! ```text
//! 原始文本
//!    │
//!    ▼
//! ┌─────────────────┐
//! │  chunk_text()   │ 文本分块
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │ index_chunks()  │ 嵌入并索引
//! └────────┬────────┘
//!          │
//!          ▼
//!      向量存储
//!
//! --- 查询阶段 ---
//!
//! 用户查询
//!    │
//!    ▼
//! ┌─────────────────┐
//! │   retrieve()    │ 检索相关片段
//! └────────┬────────┘
//!          │
//!          ▼
//!   Citation 列表
//! ```
//!
//! # 使用示例
//!
//! ## 索引文本
//!
//! ```rust,no_run
//! use agentkit::embed::OpenAiEmbedding;
//! use agentkit::retrieval::ChromaVectorStore;
//! use agentkit::rag::{index_text, chunk_text};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let embedder = OpenAiEmbedding::from_env()?;
//! let store = ChromaVectorStore::from_env()?;
//!
//! // 分块并索引
//! let chunks = index_text(
//!     &embedder,
//!     &store,
//!     "doc1",           // 文档 ID
//!     "这是一段长文本...", // 原始文本
//!     500,              // 每块最大字符数
//!     50,               // 重叠字符数
//! ).await?;
//!
//! println!("索引了 {} 个块", chunks.len());
//! # Ok(())
//! # }
//! ```
//!
//! ## 检索引用
//!
//! ```rust,no_run
//! use agentkit::embed::OpenAiEmbedding;
//! use agentkit::retrieval::ChromaVectorStore;
//! use agentkit::rag::retrieve;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let embedder = OpenAiEmbedding::from_env()?;
//! let store = ChromaVectorStore::from_env()?;
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
//!
//! # 核心函数
//!
//! ## chunk_text
//!
//! 将长文本分块，支持重叠：
//!
//! ```rust
//! use agentkit::rag::chunk_text;
//!
//! let chunks = chunk_text(
//!     "doc1",      // 文档 ID
//!     "长文本...",  // 原始文本
//!     500,         // 每块最大字符数
//!     50,          // 重叠字符数
//! );
//! ```
//!
//! ## index_chunks
//!
//! 将分块嵌入并索引到向量存储：
//!
//! ```rust,no_run
//! use agentkit::rag::index_chunks;
//! # use agentkit::embed::OpenAiEmbedding;
//! # use agentkit::retrieval::ChromaVectorStore;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let embedder = OpenAiEmbedding::from_env()?;
//! let store = ChromaVectorStore::from_env()?;
//! let chunks = vec![];  // TextChunk 列表
//!
//! index_chunks(&embedder, &store, &chunks).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## retrieve
//!
//! 检索相关片段并生成引用：
//!
//! ```rust,no_run
//! use agentkit::rag::retrieve;
//! # use agentkit::embed::OpenAiEmbedding;
//! # use agentkit::retrieval::ChromaVectorStore;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let embedder = OpenAiEmbedding::from_env()?;
//! let store = ChromaVectorStore::from_env()?;
//!
//! let citations = retrieve(&embedder, &store, "查询", 5).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Citation 格式
//!
//! [`Citation`] 包含检索结果的完整信息：
//!
//! ```rust
//! use agentkit::rag::Citation;
//!
//! let citation = Citation {
//!     doc_id: Some("doc1".to_string()),
//!     chunk_id: "doc1:0".to_string(),
//!     score: 0.95,
//!     text: Some("相关片段内容".to_string()),
//!     metadata: None,
//! };
//!
//! // 渲染引用标识
//! println!("引用自：{}", citation.render());
//! ```
//!
//! # 最佳实践
//!
//! ## 分块大小
//!
//! - **小文本**（< 1000 字符）: 不需要分块
//! - **中等文本**（1000-5000 字符）: 500 字符/块，50 字符重叠
//! - **大文本**（> 5000 字符）: 1000 字符/块，100 字符重叠
//!
//! ## TopK 选择
//!
//! - **精确查询**: top_k = 3-5
//! - **模糊查询**: top_k = 5-10
//! - **探索性查询**: top_k = 10-20
//!
//! ## 性能优化
//!
//! - 使用 [`CachedEmbeddingProvider`] 减少重复嵌入
//! - 批量嵌入（`embed_batch`）优于单次嵌入
//! - 定期清理向量存储（`clear`）

use agentkit_core::{
    embed::EmbeddingProvider,
    error::ProviderError,
    retrieval::{SearchResult, VectorQuery, VectorRecord, VectorStore},
};
use serde_json::{json, Value};

/// 文本块
///
/// 包含分块后的文本及其元数据。
///
/// # 字段说明
///
/// - `id`: 块的唯一标识（格式：`{doc_id}:{chunk_index}`）
/// - `text`: 块内容
/// - `metadata`: 元数据（文档 ID、块索引、位置等）
///
/// # 示例
///
/// ```rust
/// use agentkit::rag::TextChunk;
///
/// let chunk = TextChunk {
///     id: "doc1:0".to_string(),
///     text: "Hello, World!".to_string(),
///     metadata: Some(serde_json::json!({
///         "doc_id": "doc1",
///         "chunk_index": 0,
///         "start_char": 0,
///         "end_char": 13
///     })),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct TextChunk {
    /// 块的唯一标识
    pub id: String,
    /// 块内容
    pub text: String,
    /// 元数据
    pub metadata: Option<Value>,
}

/// 将文本分块
///
/// # 参数
///
/// - `doc_id`: 文档 ID
/// - `text`: 原始文本
/// - `max_chars`: 每块最大字符数
/// - `overlap_chars`: 块间重叠字符数
///
/// # 返回值
///
/// 返回 [`TextChunk`] 列表。
///
/// # 示例
///
/// ```rust
/// use agentkit::rag::chunk_text;
///
/// let chunks = chunk_text(
///     "doc1",
///     "这是一段长文本，需要分成多个块。",
///     100,  // 每块最大 100 字符
///     10,   // 重叠 10 字符
/// );
///
/// for chunk in chunks {
///     println!("块 {}: {}", chunk.id, chunk.text);
/// }
/// ```
pub fn chunk_text(
    doc_id: &str,
    text: &str,
    max_chars: usize,
    overlap_chars: usize,
) -> Vec<TextChunk> {
    let max_chars = max_chars.max(1);
    let overlap_chars = overlap_chars.min(max_chars.saturating_sub(1));

    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();
    let mut start = 0usize;
    let mut idx = 0usize;

    while start < chars.len() {
        let end = (start + max_chars).min(chars.len());
        let chunk_text: String = chars[start..end].iter().collect();
        let id = format!("{}:{}", doc_id, idx);

        out.push(TextChunk {
            id,
            text: chunk_text,
            metadata: Some(
                json!({"doc_id": doc_id, "chunk_index": idx, "start_char": start, "end_char": end}),
            ),
        });

        if end == chars.len() {
            break;
        }

        idx += 1;
        start = end.saturating_sub(overlap_chars);
        if start >= end {
            start = end;
        }
    }

    out
}

/// 索引分块列表
///
/// # 参数
///
/// - `embedder`: Embedding Provider
/// - `store`: VectorStore
/// - `chunks`: [`TextChunk`] 列表
///
/// # 返回值
///
/// 成功返回 `Ok(())`，失败返回 [`ProviderError`]。
///
/// # 示例
///
/// ```rust,no_run
/// use agentkit::rag::{chunk_text, index_chunks};
/// use agentkit::embed::OpenAiEmbedding;
/// use agentkit::retrieval::ChromaVectorStore;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let embedder = OpenAiEmbedding::from_env()?;
/// let store = ChromaVectorStore::from_env()?;
///
/// let chunks = chunk_text("doc1", "长文本...", 500, 50);
/// index_chunks(&embedder, &store, &chunks).await?;
/// # Ok(())
/// # }
/// ```
pub async fn index_chunks<P, S>(
    embedder: &P,
    store: &S,
    chunks: &[TextChunk],
) -> Result<(), ProviderError>
where
    P: EmbeddingProvider,
    S: VectorStore,
{
    if chunks.is_empty() {
        return Ok(());
    }

    let texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
    let vectors = embedder.embed_batch(&texts).await?;

    if vectors.len() != chunks.len() {
        return Err(ProviderError::Message(
            "embed_batch 返回的向量数量与输入不一致".to_string(),
        ));
    }

    let records: Vec<VectorRecord> = chunks
        .iter()
        .zip(vectors)
        .map(|(c, v)| {
            let mut r = VectorRecord::new(c.id.clone(), v).with_text(c.text.clone());
            if let Some(md) = &c.metadata {
                r = r.with_metadata(md.clone());
            }
            r
        })
        .collect();

    store.upsert(records).await
}

/// 索引文本
///
///  Convenience 函数：先分块，再索引。
///
/// # 参数
///
/// - `embedder`: Embedding Provider
/// - `store`: VectorStore
/// - `doc_id`: 文档 ID
/// - `text`: 原始文本
/// - `max_chars`: 每块最大字符数
/// - `overlap_chars`: 块间重叠字符数
///
/// # 返回值
///
/// 返回 [`TextChunk`] 列表。
///
/// # 示例
///
/// ```rust,no_run
/// use agentkit::rag::index_text;
/// use agentkit::embed::OpenAiEmbedding;
/// use agentkit::retrieval::ChromaVectorStore;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let embedder = OpenAiEmbedding::from_env()?;
/// let store = ChromaVectorStore::from_env()?;
///
/// let chunks = index_text(
///     &embedder,
///     &store,
///     "doc1",
///     "长文本...",
///     500,
///     50,
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn index_text<P, S>(
    embedder: &P,
    store: &S,
    doc_id: &str,
    text: &str,
    max_chars: usize,
    overlap_chars: usize,
) -> Result<Vec<TextChunk>, ProviderError>
where
    P: EmbeddingProvider,
    S: VectorStore,
{
    let chunks = chunk_text(doc_id, text, max_chars, overlap_chars);
    index_chunks(embedder, store, &chunks).await?;
    Ok(chunks)
}

/// 引用
///
/// 包含检索结果及其元数据，用于生成回答时的引用。
///
/// # 字段说明
///
/// - `doc_id`: 文档 ID（可选）
/// - `chunk_id`: 块 ID
/// - `score`: 相似度分数
/// - `text`: 块内容（可选）
/// - `metadata`: 元数据（可选）
///
/// # 示例
///
/// ```rust
/// use agentkit::rag::Citation;
///
/// let citation = Citation {
///     doc_id: Some("doc1".to_string()),
///     chunk_id: "doc1:0".to_string(),
///     score: 0.95,
///     text: Some("相关片段内容".to_string()),
///     metadata: None,
/// };
///
/// // 渲染引用标识
/// println!("引用自：{}", citation.render());
/// ```
#[derive(Debug, Clone)]
pub struct Citation {
    /// 文档 ID（可选）
    pub doc_id: Option<String>,
    /// 块 ID
    pub chunk_id: String,
    /// 相似度分数
    pub score: f32,
    /// 块内容（可选）
    pub text: Option<String>,
    /// 元数据（可选）
    pub metadata: Option<Value>,
}

impl Citation {
    /// 渲染引用标识
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit::rag::Citation;
    ///
    /// let citation = Citation {
    ///     doc_id: Some("doc1".to_string()),
    ///     chunk_id: "0".to_string(),
    ///     score: 0.95,
    ///     text: None,
    ///     metadata: None,
    /// };
    ///
    /// assert_eq!(citation.render(), "doc1:0");
    /// ```
    pub fn render(&self) -> String {
        match &self.doc_id {
            Some(d) => format!("{}:{}", d, self.chunk_id),
            None => self.chunk_id.clone(),
        }
    }
}

/// 检索相关片段
///
/// # 参数
///
/// - `embedder`: Embedding Provider
/// - `store`: VectorStore
/// - `query_text`: 查询文本
/// - `top_k`: 返回数量
///
/// # 返回值
///
/// 返回 [`Citation`] 列表。
///
/// # 示例
///
/// ```rust,no_run
/// use agentkit::rag::retrieve;
/// use agentkit::embed::OpenAiEmbedding;
/// use agentkit::retrieval::ChromaVectorStore;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let embedder = OpenAiEmbedding::from_env()?;
/// let store = ChromaVectorStore::from_env()?;
///
/// let citations = retrieve(&embedder, &store, "查询问题", 5).await?;
///
/// for cite in citations {
///     println!("引用 [{}]: {}", cite.render(), cite.text.unwrap_or_default());
/// }
/// # Ok(())
/// # }
/// ```
pub async fn retrieve<P, S>(
    embedder: &P,
    store: &S,
    query_text: &str,
    top_k: usize,
) -> Result<Vec<Citation>, ProviderError>
where
    P: EmbeddingProvider,
    S: VectorStore,
{
    let vector = embedder.embed(query_text).await?;
    let query = VectorQuery::new(vector).with_top_k(top_k);
    let results = store.search(query).await?;
    Ok(results.into_iter().map(search_result_to_citation).collect())
}

/// 将 SearchResult 转换为 Citation
fn search_result_to_citation(r: SearchResult) -> Citation {
    let doc_id = r
        .metadata
        .as_ref()
        .and_then(|m| m.get("doc_id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Citation {
        doc_id,
        chunk_id: r.id,
        score: r.score,
        text: r.text,
        metadata: r.metadata,
    }
}
