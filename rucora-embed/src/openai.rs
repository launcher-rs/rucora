//! OpenAI Embedding Provider 实现。
//!
//! 约定：
//! - API Key 从 `OPENAI_API_KEY` 环境变量读取
//! - Base URL 默认 `https://api.openai.com/v1`，也可通过 `OPENAI_BASE_URL` 覆盖
//! - 默认使用 `text-embedding-ada-002` 模型

use std::env;

use async_trait::async_trait;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use rucora_core::{embed::EmbeddingProvider, error::ProviderError};
use serde_json::{Value, json};

/// OpenAI Embedding Provider。
pub struct OpenAiEmbeddingProvider {
    client: reqwest::Client,
    base_url: String,
    model: String,
    embedding_dim: Option<usize>,
}

impl OpenAiEmbeddingProvider {
    /// 从环境变量创建 Provider。
    pub fn from_env() -> Result<Self, ProviderError> {
        let api_key = env::var("OPENAI_API_KEY")
            .map_err(|_| ProviderError::Message("缺少环境变量 OPENAI_API_KEY".to_string()))?;
        let base_url =
            env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".to_string());

        let embedding_model = env::var("EMBEDDING_MODEL")
            .map_err(|_| ProviderError::Message("缺少环境变量 EMBEDDING_MODEL".to_string()))?;

        Ok(Self::new(base_url, api_key, embedding_model))
    }

    /// 创建 Provider。
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        let api_key = api_key.into();
        let model = model.into();
        let base_url = base_url.into();

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Ok(v) = HeaderValue::from_str(&format!("Bearer {api_key}")) {
            headers.insert(AUTHORIZATION, v);
        }

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .expect("reqwest client build failed");

        // 根据模型确定维度
        let embedding_dim = match model.as_str() {
            "text-embedding-ada-002" | "text-embedding-3-small" => Some(1536),
            "text-embedding-3-large" => Some(3072),
            _ => None,
        };

        Self {
            client,
            base_url,
            model,
            embedding_dim,
        }
    }

    /// 设置模型（用于切换不同嵌入模型）。
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        // 重新计算维度
        self.embedding_dim = match self.model.as_str() {
            "text-embedding-ada-002" | "text-embedding-3-small" => Some(1536),
            "text-embedding-3-large" => Some(3072),
            _ => None,
        };
        self
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAiEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, ProviderError> {
        let url = format!("{}/embeddings", self.base_url.trim_end_matches('/'));

        let body = json!({
            "model": self.model,
            "input": text,
        });

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        let status = resp.status();
        let data: Value = resp
            .json()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        if !status.is_success() {
            return Err(ProviderError::Message(format!(
                "OpenAI embedding 请求失败：status={status} body={data}"
            )));
        }

        // 解析响应：data[0].embedding
        let embedding = data
            .get("data")
            .and_then(|d| d.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("embedding"))
            .and_then(|e| e.as_array())
            .ok_or_else(|| ProviderError::Message("OpenAI 响应缺少 embedding 数据".to_string()))?
            .iter()
            .filter_map(|v| v.as_f64().map(|f| f as f32))
            .collect();

        Ok(embedding)
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, ProviderError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let url = format!("{}/embeddings", self.base_url.trim_end_matches('/'));

        let body = json!({
            "model": self.model,
            "input": texts,
        });

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        let status = resp.status();
        let data: Value = resp
            .json()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        if !status.is_success() {
            return Err(ProviderError::Message(format!(
                "OpenAI embedding 批量请求失败：status={status} body={data}"
            )));
        }

        // 解析响应：data[].embedding
        let data_array = data
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| ProviderError::Message("OpenAI 响应缺少 data 数组".to_string()))?;

        let mut results = Vec::with_capacity(texts.len());
        for item in data_array {
            let embedding = item
                .get("embedding")
                .and_then(|e| e.as_array())
                .ok_or_else(|| {
                    ProviderError::Message("OpenAI 响应缺少 embedding 数据".to_string())
                })?
                .iter()
                .filter_map(|v| v.as_f64().map(|f| f as f32))
                .collect();
            results.push(embedding);
        }

        Ok(results)
    }

    fn embedding_dim(&self) -> Option<usize> {
        self.embedding_dim
    }
}
