//! 向量数据库（Vector Store）抽象。
//!
//! 提供向量存储、检索和管理的统一接口，支持语义搜索和 RAG 场景。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::ProviderError;

/// 向量记录。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorRecord {
    /// 唯一标识符。
    pub id: String,
    /// 向量数据。
    pub vector: Vec<f32>,
    /// 关联的文本内容（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// 元数据（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl VectorRecord {
    /// 创建新的向量记录。
    pub fn new(id: impl Into<String>, vector: Vec<f32>) -> Self {
        assert!(!vector.is_empty(), "VectorRecord vector must not be empty");
        Self {
            id: id.into(),
            vector,
            text: None,
            metadata: None,
        }
    }

    /// 设置文本内容。
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// 设置元数据。
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// 搜索结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// 记录 ID。
    pub id: String,
    /// 相似度分数（通常 0-1，越大越相似）。
    pub score: f32,
    /// 向量数据（可选，取决于 store 配置）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector: Option<Vec<f32>>,
    /// 关联的文本内容。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// 元数据。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// 查询条件。
#[derive(Debug, Clone, Default)]
pub struct VectorQuery {
    /// 查询向量。
    pub vector: Vec<f32>,
    /// 返回结果数量（默认 10）。
    pub top_k: usize,
    /// 最小相似度阈值（可选，过滤低相似度结果）。
    pub score_threshold: Option<f32>,
    /// 元数据过滤条件（可选，JSON 格式）。
    pub filter: Option<serde_json::Value>,
}

impl VectorQuery {
    /// 创建新的查询。
    pub fn new(vector: Vec<f32>) -> Self {
        Self {
            vector,
            top_k: 10,
            score_threshold: None,
            filter: None,
        }
    }

    /// 设置返回数量。
    pub fn with_top_k(mut self, top_k: usize) -> Self {
        assert!(top_k > 0, "top_k must be greater than 0");
        assert!(top_k <= 1000, "top_k must not exceed 1000");
        self.top_k = top_k;
        self
    }

    /// 设置相似度阈值。
    pub fn with_score_threshold(mut self, threshold: f32) -> Self {
        self.score_threshold = Some(threshold);
        self
    }

    /// 设置过滤条件。
    pub fn with_filter(mut self, filter: serde_json::Value) -> Self {
        self.filter = Some(filter);
        self
    }
}

/// 向量数据库抽象。
///
/// 支持基本的增删改查和相似度搜索。
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// 添加或更新记录。
    async fn upsert(&self, records: Vec<VectorRecord>) -> Result<(), ProviderError>;

    /// 根据 ID 删除记录。
    async fn delete(&self, ids: Vec<String>) -> Result<(), ProviderError>;

    /// 根据 ID 查询记录。
    async fn get(&self, ids: Vec<String>) -> Result<Vec<VectorRecord>, ProviderError>;

/// 向量相似度搜索。
     async fn search(&self, query: VectorQuery) -> Result<Vec<SearchResult>, ProviderError>;

     /// 根据文本进行混合搜索（关键词 + 向量）。
     ///
     /// 默认实现返回空结果，具体实现可重写此方法以支持文本搜索。
     ///
     /// # 参数
     ///
     /// - `text`: 查询文本
     /// - `top_k`: 返回结果数量
     /// - `score_threshold`: 最小相似度阈值
     async fn search_by_text(
         &self,
         text: &str,
         top_k: usize,
         score_threshold: Option<f32>,
     ) -> Result<Vec<SearchResult>, ProviderError> {
         let _ = (text, top_k, score_threshold);
         Ok(Vec::new())
     }

     /// 更新现有记录。
     ///
     /// 如果记录不存在，默认实现先删除再添加（upsert 语义）。
     /// 具体实现可重写以提供原子更新操作。
     ///
     /// # 参数
     ///
     /// - `record`: 要更新的向量记录
     async fn update(&self, record: VectorRecord) -> Result<(), ProviderError> {
         self.delete(vec![record.id.clone()]).await?;
         self.upsert(vec![record]).await
     }

     /// 清空所有数据。
    async fn clear(&self) -> Result<(), ProviderError>;

    /// 获取记录数量。
    async fn count(&self) -> Result<usize, ProviderError>;
}
