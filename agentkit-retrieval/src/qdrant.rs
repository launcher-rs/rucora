//! Qdrant 向量数据库实现。
//!
//! 使用 Qdrant HTTP API 进行向量存储和检索。
//! 需要指定 QDRANT_URL 和 QDRANT_API_KEY（可选）环境变量。

use std::env;

use agentkit_core::{
    error::ProviderError,
    retrieval::{SearchResult, VectorQuery, VectorRecord, VectorStore},
};
use async_trait::async_trait;
use serde_json::{Value, json};

/// Qdrant Vector Store。
pub struct QdrantVectorStore {
    client: reqwest::Client,
    base_url: String,
    collection: String,
    api_key: Option<String>,
}

impl QdrantVectorStore {
    /// 从环境变量创建 Store。
    ///
    /// 环境变量：
    /// - QDRANT_URL: Qdrant 服务地址（默认 http://localhost:6333）
    /// - QDRANT_API_KEY: API 密钥（可选）
    /// - QDRANT_COLLECTION: 集合名称（默认 default）
    pub fn from_env() -> Result<Self, ProviderError> {
        let base_url =
            env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6333".to_string());
        let api_key = env::var("QDRANT_API_KEY").ok();
        let collection = env::var("QDRANT_COLLECTION").unwrap_or_else(|_| "default".to_string());

        Ok(Self::new(base_url, collection, api_key))
    }

    /// 创建 Store。
    pub fn new(
        base_url: impl Into<String>,
        collection: impl Into<String>,
        api_key: Option<String>,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            collection: collection.into(),
            api_key,
        }
    }

    /// 设置集合名称。
    pub fn with_collection(mut self, collection: impl Into<String>) -> Self {
        self.collection = collection.into();
        self
    }

    fn build_request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!(
            "{}/collections/{}{}",
            self.base_url.trim_end_matches('/'),
            self.collection,
            path
        );
        let mut req = self.client.request(method, url);
        if let Some(key) = &self.api_key {
            req = req.header("api-key", key);
        }
        req
    }

    /// 检查集合是否存在。
    pub async fn collection_exists(&self) -> Result<bool, ProviderError> {
        let resp = self
            .build_request(reqwest::Method::GET, "")
            .send()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        Ok(resp.status().is_success())
    }

    /// 创建集合。
    pub async fn create_collection(&self, vector_dim: usize) -> Result<(), ProviderError> {
        let body = json!({
            "vectors": {
                "size": vector_dim,
                "distance": "Cosine"
            }
        });

        let resp = self
            .build_request(reqwest::Method::PUT, "")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Message(format!(
                "创建 Qdrant 集合失败: {}",
                text
            )));
        }

        Ok(())
    }
}

#[async_trait]
impl VectorStore for QdrantVectorStore {
    async fn upsert(&self, records: Vec<VectorRecord>) -> Result<(), ProviderError> {
        let points: Vec<Value> = records
            .into_iter()
            .map(|r| {
                let mut payload = json!({});
                if let Some(text) = r.text {
                    payload["text"] = json!(text);
                }
                if let Some(metadata) = r.metadata {
                    if let Some(obj) = metadata.as_object() {
                        for (k, v) in obj {
                            payload[k] = v.clone();
                        }
                    }
                }

                json!({
                    "id": r.id,
                    "vector": r.vector,
                    "payload": payload
                })
            })
            .collect();

        let body = json!({ "points": points });

        let resp = self
            .build_request(reqwest::Method::PUT, "/points")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Message(format!(
                "Qdrant upsert 失败: {}",
                text
            )));
        }

        Ok(())
    }

    async fn delete(&self, ids: Vec<String>) -> Result<(), ProviderError> {
        // Qdrant 支持多种 ID 格式，这里使用 point id 列表
        let body = json!({
            "points": ids
        });

        let resp = self
            .build_request(reqwest::Method::POST, "/points/delete")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Message(format!(
                "Qdrant delete 失败: {}",
                text
            )));
        }

        Ok(())
    }

    async fn get(&self, ids: Vec<String>) -> Result<Vec<VectorRecord>, ProviderError> {
        let body = json!({
            "ids": ids,
            "with_payload": true,
            "with_vector": true
        });

        let resp = self
            .build_request(reqwest::Method::POST, "/points")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Message(format!("Qdrant get 失败: {}", text)));
        }

        let data: Value = resp
            .json()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        let points = data
            .get("result")
            .and_then(|r| r.as_array())
            .unwrap_or(&Vec::new())
            .clone();

        let records: Vec<VectorRecord> = points
            .into_iter()
            .filter_map(|p| {
                let id = p.get("id")?.as_str()?.to_string();
                let vector = p
                    .get("vector")
                    .and_then(|v| v.as_array())?
                    .iter()
                    .filter_map(|x| x.as_f64().map(|f| f as f32))
                    .collect();

                let payload = p.get("payload")?;
                let text = payload
                    .get("text")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string());

                Some(VectorRecord {
                    id,
                    vector,
                    text,
                    metadata: Some(payload.clone()),
                })
            })
            .collect();

        Ok(records)
    }

    async fn search(&self, query: VectorQuery) -> Result<Vec<SearchResult>, ProviderError> {
        let mut body = json!({
            "vector": query.vector,
            "limit": query.top_k,
            "with_payload": true,
            "with_vector": false
        });

        // 添加分数阈值（Qdrant 使用 score_threshold）
        if let Some(threshold) = query.score_threshold {
            body["score_threshold"] = json!(threshold);
        }

        // 添加过滤条件
        if let Some(filter) = query.filter {
            body["filter"] = filter;
        }

        let resp = self
            .build_request(reqwest::Method::POST, "/points/search")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Message(format!(
                "Qdrant search 失败: {}",
                text
            )));
        }

        let data: Value = resp
            .json()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        let results = data
            .get("result")
            .and_then(|r| r.as_array())
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|r| {
                let id = r.get("id")?.as_str()?.to_string();
                let score = r.get("score")?.as_f64()? as f32;
                let payload = r.get("payload")?;
                let text = payload
                    .get("text")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string());

                Some(SearchResult {
                    id,
                    score,
                    vector: None, // Qdrant search 默认不返回 vector
                    text,
                    metadata: Some(payload.clone()),
                })
            })
            .collect();

        Ok(results)
    }

    async fn clear(&self) -> Result<(), ProviderError> {
        let body = json!({ "filter": {} });

        let resp = self
            .build_request(reqwest::Method::POST, "/points/delete")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Message(format!(
                "Qdrant clear 失败: {}",
                text
            )));
        }

        Ok(())
    }

    async fn count(&self) -> Result<usize, ProviderError> {
        let resp = self
            .build_request(reqwest::Method::GET, "/points/count")
            .send()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Message(format!(
                "Qdrant count 失败: {}",
                text
            )));
        }

        let data: Value = resp
            .json()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        let count = data
            .get("result")
            .and_then(|r| r.get("count"))
            .and_then(|c| c.as_u64())
            .unwrap_or(0) as usize;

        Ok(count)
    }
}


