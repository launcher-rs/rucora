//! 内存向量存储实现
//!
//! # 概述
//!
//! 本模块提供基于内存的简单向量存储，用于测试和演示。
//! 使用余弦相似度计算向量相似性。
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use agentkit::retrieval::InMemoryVectorStore;
//! use agentkit_core::retrieval::{VectorRecord, VectorQuery};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let store = InMemoryVectorStore::new();
//!
//! // 插入向量
//! store.upsert(vec![
//!     VectorRecord::new("doc1", vec![1.0, 0.0]).with_text("文档 1"),
//!     VectorRecord::new("doc2", vec![0.0, 1.0]).with_text("文档 2"),
//! ]).await?;
//!
//! // 搜索
//! let results = store.search(
//!     VectorQuery::new(vec![1.0, 0.0]).with_top_k(10)
//! ).await?;
//! # Ok(())
//! # }
//! ```

use agentkit_core::error::ProviderError;
use agentkit_core::retrieval::{SearchResult, VectorQuery, VectorRecord, VectorStore};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 内存向量存储
///
/// 使用 HashMap 存储向量，支持基本的增删查改操作。
/// 相似度计算使用余弦相似度。
#[derive(Debug, Default, Clone)]
pub struct InMemoryVectorStore {
    records: Arc<RwLock<HashMap<String, VectorRecord>>>,
}

impl InMemoryVectorStore {
    /// 创建新的内存向量存储
    pub fn new() -> Self {
        Self {
            records: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 计算余弦相似度
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.is_empty() || b.is_empty() || a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if magnitude_a == 0.0 || magnitude_b == 0.0 {
            return 0.0;
        }

        dot_product / (magnitude_a * magnitude_b)
    }
}

#[async_trait]
impl VectorStore for InMemoryVectorStore {
    async fn upsert(&self, records: Vec<VectorRecord>) -> Result<(), ProviderError> {
        let mut store = self.records.write().await;
        for record in records {
            store.insert(record.id.clone(), record);
        }
        Ok(())
    }

    async fn delete(&self, ids: Vec<String>) -> Result<(), ProviderError> {
        let mut store = self.records.write().await;
        for id in ids {
            store.remove(&id);
        }
        Ok(())
    }

    async fn get(&self, ids: Vec<String>) -> Result<Vec<VectorRecord>, ProviderError> {
        let store = self.records.read().await;
        Ok(ids.iter().filter_map(|id| store.get(id).cloned()).collect())
    }

    async fn search(&self, query: VectorQuery) -> Result<Vec<SearchResult>, ProviderError> {
        let store = self.records.read().await;

        // 计算所有向量的相似度
        let mut results: Vec<SearchResult> = store
            .values()
            .map(|record| {
                let score = Self::cosine_similarity(&query.vector, &record.vector);
                SearchResult {
                    id: record.id.clone(),
                    score,
                    text: record.text.clone(),
                    metadata: record.metadata.clone(),
                    vector: None, // VectorQuery 不支持 include_vectors
                }
            })
            .collect();

        // 按相似度排序（降序）
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // 应用阈值过滤
        if let Some(threshold) = query.score_threshold {
            results.retain(|r| r.score >= threshold);
        }

        // 限制返回数量
        results.truncate(query.top_k);

        Ok(results)
    }

    async fn clear(&self) -> Result<(), ProviderError> {
        let mut store = self.records.write().await;
        store.clear();
        Ok(())
    }

    async fn count(&self) -> Result<usize, ProviderError> {
        let store = self.records.read().await;
        Ok(store.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_in_memory_vector_store_basic() {
        let store = InMemoryVectorStore::new();

        // 插入
        store
            .upsert(vec![
                VectorRecord::new("doc1", vec![1.0, 0.0]).with_text("文档 1"),
                VectorRecord::new("doc2", vec![0.0, 1.0]).with_text("文档 2"),
            ])
            .await
            .unwrap();

        // 计数
        assert_eq!(store.count().await.unwrap(), 2);

        // 获取
        let records = store.get(vec!["doc1".to_string()]).await.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "doc1");

        // 搜索
        let results = store
            .search(VectorQuery::new(vec![1.0, 0.0]).with_top_k(10))
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "doc1"); // 最相似

        // 删除
        store.delete(vec!["doc1".to_string()]).await.unwrap();
        assert_eq!(store.count().await.unwrap(), 1);

        // 清空
        store.clear().await.unwrap();
        assert_eq!(store.count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_in_memory_vector_store_metadata() {
        let store = InMemoryVectorStore::new();

        store
            .upsert(vec![
                VectorRecord::new("a", vec![1.0, 0.0]).with_metadata(json!({"k": 1})),
                VectorRecord::new("b", vec![0.9, 0.1]).with_metadata(json!({"k": 2})),
            ])
            .await
            .unwrap();

        let results = store
            .search(VectorQuery::new(vec![1.0, 0.0]).with_top_k(3))
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
        assert!(results[0].score > results[1].score);
    }

    #[tokio::test]
    async fn test_cosine_similarity() {
        // 相同向量
        let sim = InMemoryVectorStore::cosine_similarity(&[1.0, 0.0], &[1.0, 0.0]);
        assert!((sim - 1.0).abs() < 0.001);

        // 正交向量
        let sim = InMemoryVectorStore::cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]);
        assert!(sim.abs() < 0.001);

        // 相反向量
        let sim = InMemoryVectorStore::cosine_similarity(&[1.0, 0.0], &[-1.0, 0.0]);
        assert!((sim + 1.0).abs() < 0.001);
    }
}


