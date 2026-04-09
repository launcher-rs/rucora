//! Chroma 向量数据库实现。
//!
//! 使用 Chroma HTTP API 进行向量存储和检索。
//! 需要指定 CHROMA_URL 环境变量（默认 http://localhost:8000）。

use std::env;

use agentkit_core::{
    error::ProviderError,
    retrieval::{SearchResult, VectorQuery, VectorRecord, VectorStore},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Chroma 集合信息。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[allow(dead_code)]
struct ChromaCollection {
    id: String,
    name: String,
}

/// Chroma Vector Store。
pub struct ChromaVectorStore {
    client: reqwest::Client,
    base_url: String,
    collection: String,
    tenant: String,
    database: String,
}

impl ChromaVectorStore {
    /// 从环境变量创建 Store。
    ///
    /// 环境变量：
    /// - CHROMA_URL: Chroma 服务地址（默认 http://localhost:8000）
    /// - CHROMA_COLLECTION: 集合名称（默认 default）
    /// - CHROMA_TENANT: 租户（默认 default_tenant）
    /// - CHROMA_DATABASE: 数据库（默认 default_database）
    pub fn from_env() -> Result<Self, ProviderError> {
        let base_url =
            env::var("CHROMA_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());
        let collection = env::var("CHROMA_COLLECTION").unwrap_or_else(|_| "default".to_string());
        let tenant = env::var("CHROMA_TENANT").unwrap_or_else(|_| "default_tenant".to_string());
        let database =
            env::var("CHROMA_DATABASE").unwrap_or_else(|_| "default_database".to_string());

        Ok(Self::new(base_url, collection, tenant, database))
    }

    /// 创建 Store。
    pub fn new(
        base_url: impl Into<String>,
        collection: impl Into<String>,
        tenant: impl Into<String>,
        database: impl Into<String>,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            collection: collection.into(),
            tenant: tenant.into(),
            database: database.into(),
        }
    }

    /// 设置集合名称。
    pub fn with_collection(mut self, collection: impl Into<String>) -> Self {
        self.collection = collection.into();
        self
    }

    /// 设置租户。
    pub fn with_tenant(mut self, tenant: impl Into<String>) -> Self {
        self.tenant = tenant.into();
        self
    }

    /// 设置数据库。
    pub fn with_database(mut self, database: impl Into<String>) -> Self {
        self.database = database.into();
        self
    }

    fn base_request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}/api/v1{}", self.base_url.trim_end_matches('/'), path);
        self.client
            .request(method, &url)
            .header("Content-Type", "application/json")
    }

    /// 检查集合是否存在。
    pub async fn collection_exists(&self) -> Result<bool, ProviderError> {
        let resp = self
            .base_request(
                reqwest::Method::GET,
                &format!(
                    "/collections/{}?tenant={}&database={}",
                    self.collection, self.tenant, self.database
                ),
            )
            .send()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        Ok(resp.status().is_success())
    }

    /// 创建集合。
    pub async fn create_collection(&self) -> Result<(), ProviderError> {
        let body = json!({
            "name": self.collection,
            "tenant": self.tenant,
            "database": self.database,
        });

        let resp = self
            .base_request(reqwest::Method::POST, "/collections")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Message(format!(
                "创建 Chroma 集合失败: {text}"
            )));
        }

        Ok(())
    }

    /// 删除集合。
    pub async fn delete_collection(&self) -> Result<(), ProviderError> {
        let resp = self
            .base_request(
                reqwest::Method::DELETE,
                &format!(
                    "/collections/{}?tenant={}&database={}",
                    self.collection, self.tenant, self.database
                ),
            )
            .send()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Message(format!(
                "删除 Chroma 集合失败: {text}"
            )));
        }

        Ok(())
    }
}

#[async_trait]
impl VectorStore for ChromaVectorStore {
    async fn upsert(&self, records: Vec<VectorRecord>) -> Result<(), ProviderError> {
        if records.is_empty() {
            return Ok(());
        }

        let ids: Vec<String> = records.iter().map(|r| r.id.clone()).collect();
        let embeddings: Vec<Vec<f32>> = records.iter().map(|r| r.vector.clone()).collect();
        let metadatas: Vec<Value> = records
            .iter()
            .map(|r| {
                let mut meta = json!({});
                if let Some(text) = &r.text {
                    meta["text"] = json!(text);
                }
                if let Some(md) = &r.metadata
                    && let Some(obj) = md.as_object()
                {
                    for (k, v) in obj {
                        meta[k] = v.clone();
                    }
                }
                meta
            })
            .collect();

        let body = json!({
            "ids": ids,
            "embeddings": embeddings,
            "metadatas": metadatas,
        });

        let resp = self
            .base_request(
                reqwest::Method::POST,
                &format!(
                    "/collections/{}/add?tenant={}&database={}",
                    self.collection, self.tenant, self.database
                ),
            )
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Message(format!(
                "Chroma upsert 失败: {text}"
            )));
        }

        Ok(())
    }

    async fn delete(&self, ids: Vec<String>) -> Result<(), ProviderError> {
        if ids.is_empty() {
            return Ok(());
        }

        let body = json!({ "ids": ids });

        let resp = self
            .base_request(
                reqwest::Method::POST,
                &format!(
                    "/collections/{}/delete?tenant={}&database={}",
                    self.collection, self.tenant, self.database
                ),
            )
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Message(format!(
                "Chroma delete 失败: {text}"
            )));
        }

        Ok(())
    }

    async fn get(&self, ids: Vec<String>) -> Result<Vec<VectorRecord>, ProviderError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let body = json!({
            "ids": ids,
            "include": ["embeddings", "metadatas"],
        });

        let resp = self
            .base_request(
                reqwest::Method::POST,
                &format!(
                    "/collections/{}/get?tenant={}&database={}",
                    self.collection, self.tenant, self.database
                ),
            )
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Message(format!("Chroma get 失败: {text}")));
        }

        let data: Value = resp
            .json()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        let result_ids = data
            .get("ids")
            .and_then(|v| v.as_array())
            .unwrap_or(&Vec::new())
            .clone();
        let result_embeddings = data
            .get("embeddings")
            .and_then(|v| v.as_array())
            .unwrap_or(&Vec::new())
            .clone();
        let result_metadatas = data
            .get("metadatas")
            .and_then(|v| v.as_array())
            .unwrap_or(&Vec::new())
            .clone();

        let mut records = Vec::new();
        for (i, id_value) in result_ids.iter().enumerate() {
            let id = id_value.as_str().unwrap_or("").to_string();
            let vector = result_embeddings
                .get(i)
                .and_then(|e| e.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_f64().map(|f| f as f32))
                        .collect()
                })
                .unwrap_or_default();

            let metadata = result_metadatas.get(i).cloned();
            let text = metadata
                .as_ref()
                .and_then(|m| m.get("text"))
                .and_then(|t| t.as_str())
                .map(|s| s.to_string());

            records.push(VectorRecord {
                id,
                vector,
                text,
                metadata,
            });
        }

        Ok(records)
    }

    async fn search(&self, query: VectorQuery) -> Result<Vec<SearchResult>, ProviderError> {
        let mut body = json!({
            "query_embeddings": vec![query.vector],
            "n_results": query.top_k,
            "include": ["metadatas", "distances"],
        });

        // 添加过滤条件（如果提供）
        if let Some(filter) = query.filter {
            body["where"] = filter;
        }

        let resp = self
            .base_request(
                reqwest::Method::POST,
                &format!(
                    "/collections/{}/query?tenant={}&database={}",
                    self.collection, self.tenant, self.database
                ),
            )
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Message(format!(
                "Chroma query 失败: {text}"
            )));
        }

        let data: Value = resp
            .json()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        // Chroma 返回嵌套数组，每个查询对应一组结果
        let ids_arrays = data
            .get("ids")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let distances_arrays = data
            .get("distances")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let metadatas_arrays = data
            .get("metadatas")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut results = Vec::new();
        for (i, id_value) in ids_arrays.iter().enumerate() {
            let id = id_value.as_str().unwrap_or("").to_string();
            // Chroma 返回的是距离（越小越相似），转换为相似度分数（越大越好）
            let distance = distances_arrays
                .get(i)
                .and_then(|d| d.as_f64())
                .unwrap_or(0.0) as f32;
            let score = 1.0 / (1.0 + distance); // 简单转换

            let metadata = metadatas_arrays.get(i).cloned();
            let text = metadata
                .as_ref()
                .and_then(|m| m.get("text"))
                .and_then(|t| t.as_str())
                .map(|s| s.to_string());

            // 应用阈值过滤
            if let Some(threshold) = query.score_threshold
                && score < threshold
            {
                continue;
            }

            results.push(SearchResult {
                id,
                score,
                vector: None, // Chroma 默认不返回向量
                text,
                metadata,
            });
        }

        Ok(results)
    }

    async fn clear(&self) -> Result<(), ProviderError> {
        // Chroma 没有直接清空 API，删除并重新创建集合
        if self.collection_exists().await? {
            self.delete_collection().await?;
            self.create_collection().await?;
        }
        Ok(())
    }

    async fn count(&self) -> Result<usize, ProviderError> {
        let resp = self
            .base_request(
                reqwest::Method::GET,
                &format!(
                    "/collections/{}/count?tenant={}&database={}",
                    self.collection, self.tenant, self.database
                ),
            )
            .send()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Message(format!(
                "Chroma count 失败: {text}"
            )));
        }

        let count: usize = resp
            .text()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?
            .parse()
            .map_err(|e: std::num::ParseIntError| ProviderError::Message(e.to_string()))?;

        Ok(count)
    }
}
