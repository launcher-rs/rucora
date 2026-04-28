//! 内存向量存储实现。
//!
//! 基于 HashMap 的简单实现，适合开发和测试场景。
//! 不适用于生产环境（无持久化，无分布式）。

use std::collections::HashMap;
use std::sync::RwLock;

use async_trait::async_trait;
use rucora_core::{
    error::ProviderError,
    retrieval::{SearchResult, VectorQuery, VectorRecord, VectorStore},
};

/// 内存向量存储。
pub struct InMemoryVectorStore {
    data: RwLock<HashMap<String, VectorRecord>>,
}

impl Default for InMemoryVectorStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryVectorStore {
    /// 创建空的内存存储。
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }

    /// 计算余弦相似度。
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        dot_product / (norm_a * norm_b)
    }
}

#[async_trait]
impl VectorStore for InMemoryVectorStore {
    async fn upsert(&self, records: Vec<VectorRecord>) -> Result<(), ProviderError> {
        let mut data = self
            .data
            .write()
            .map_err(|_| ProviderError::Message("无法获取写锁".to_string()))?;
        for record in records {
            data.insert(record.id.clone(), record);
        }
        Ok(())
    }

    async fn delete(&self, ids: Vec<String>) -> Result<(), ProviderError> {
        let mut data = self
            .data
            .write()
            .map_err(|_| ProviderError::Message("无法获取写锁".to_string()))?;
        for id in ids {
            data.remove(&id);
        }
        Ok(())
    }

    async fn get(&self, ids: Vec<String>) -> Result<Vec<VectorRecord>, ProviderError> {
        let data = self
            .data
            .read()
            .map_err(|_| ProviderError::Message("无法获取读锁".to_string()))?;
        let mut results = Vec::new();
        for id in ids {
            if let Some(record) = data.get(&id) {
                results.push(record.clone());
            }
        }
        Ok(results)
    }

    async fn search(&self, query: VectorQuery) -> Result<Vec<SearchResult>, ProviderError> {
        let data = self
            .data
            .read()
            .map_err(|_| ProviderError::Message("无法获取读锁".to_string()))?;

        let query_vector = &query.vector;
        let mut scores: Vec<(String, f32)> = data
            .values()
            .map(|record| {
                let score = Self::cosine_similarity(query_vector, &record.vector);
                (record.id.clone(), score)
            })
            .collect();

        // 按相似度降序排序
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // 应用阈值过滤和数量限制
        let threshold = query.score_threshold.unwrap_or(0.0);
        let top_k = query.top_k;

        let mut results = Vec::new();
        for (id, score) in scores.iter().take(top_k) {
            if *score < threshold {
                break;
            }
            if let Some(record) = data.get(id) {
                results.push(SearchResult {
                    id: id.clone(),
                    score: *score,
                    vector: Some(record.vector.clone()),
                    text: record.text.clone(),
                    metadata: record.metadata.clone(),
                });
            }
        }

        Ok(results)
    }

    async fn clear(&self) -> Result<(), ProviderError> {
        let mut data = self
            .data
            .write()
            .map_err(|_| ProviderError::Message("无法获取写锁".to_string()))?;
        data.clear();
        Ok(())
    }

    async fn count(&self) -> Result<usize, ProviderError> {
        let data = self
            .data
            .read()
            .map_err(|_| ProviderError::Message("无法获取读锁".to_string()))?;
        Ok(data.len())
    }
}
