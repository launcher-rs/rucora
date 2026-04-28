//! 向量嵌入（Embedding）功能抽象。
//!
//! 该模块定义文本向量化的 trait，类似于 LlmProvider，
//! 用于将文本转换为向量表示，支持语义搜索、RAG 等场景。

use async_trait::async_trait;

use crate::error::ProviderError;

/// 嵌入提供者抽象。
///
/// 该 trait 的目标：统一文本向量化的接口，
/// 支持不同嵌入模型（OpenAI text-embedding-ada-002、本地模型等）。
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// 将单条文本转换为向量。
    ///
    /// # 参数
    /// * `text` - 要向量化的文本
    ///
    /// # 返回
    /// * `Result<Vec<f32>, ProviderError>` - 向量数据（通常是 384/768/1536 维）或错误
    async fn embed(&self, text: &str) -> Result<Vec<f32>, ProviderError>;

    /// 批量文本向量化（推荐）。
    ///
    /// 大多数嵌入 API 支持批量请求，效率高于多次单条调用。
    /// 默认实现为串行单条处理，具体 Provider 可优化为批量 API 调用。
    ///
    /// # 参数
    /// * `texts` - 要向量化的文本列表
    ///
    /// # 返回
    /// * `Result<Vec<Vec<f32>>, ProviderError>` - 向量列表，顺序与输入一致
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, ProviderError> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            let embedding = self.embed(text).await?;
            results.push(embedding);
        }
        Ok(results)
    }

    /// 获取嵌入向量的维度。
    ///
    /// 返回 None 表示维度未知或可变。
    fn embedding_dim(&self) -> Option<usize> {
        None
    }
}

/// 计算余弦相似度。
///
/// 返回值为 [-1.0, 1.0]，值越大表示相似度越高。
/// 对于归一化的向量（如 OpenAI embedding），返回值即为点积。
///
/// # Errors
///
/// 当两个向量维度不一致时返回 [`ProviderError`]。
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> Result<f32, ProviderError> {
    if a.len() != b.len() {
        return Err(ProviderError::Message(format!(
            "向量维度不匹配: {} vs {}",
            a.len(),
            b.len()
        )));
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return Ok(0.0);
    }

    Ok(dot_product / (norm_a * norm_b))
}

/// 向量搜索辅助函数。
///
/// 对候选向量按与查询向量的相似度排序，返回前 k 个结果的索引和分数。
///
/// # Errors
///
/// 当计算相似度失败时返回 [`ProviderError`]。
pub fn vector_search(
    query: &[f32],
    candidates: &[Vec<f32>],
    top_k: usize,
) -> Result<Vec<(usize, f32)>, ProviderError> {
    let mut scores: Vec<(usize, f32)> = candidates
        .iter()
        .enumerate()
        .map(|(idx, vec)| {
            let sim = cosine_similarity(query, vec).unwrap_or(0.0);
            (idx, sim)
        })
        .collect();

    // 按相似度降序排序
    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scores.truncate(top_k);

    Ok(scores)
}
