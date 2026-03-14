//! Ollama Embedding Provider 实现。
//!
//! 约定：
//! - Base URL 默认 `http://localhost:11434`，也可通过 `OLLAMA_BASE_URL` 覆盖
//! - endpoint 使用 `/api/embeddings`

use std::env;

use agentkit_core::{
    embed::EmbeddingProvider,
    error::ProviderError,
};
use async_trait::async_trait;
use serde_json::{Value, json};

/// Ollama Embedding Provider。
pub struct OllamaEmbeddingProvider {
    client: reqwest::Client,
    base_url: String,
    model: String,
}

impl OllamaEmbeddingProvider {
    /// 从环境变量创建 Provider。
    pub fn from_env() -> Self {
        let base_url =
            env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());
        let model = env::var("OLLAMA_EMBED_MODEL").unwrap_or_else(|_| "nomic-embed-text".to_string());
        Self::new(base_url, model)
    }

    /// 创建 Provider。
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            model: model.into(),
        }
    }

    /// 设置模型。
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, ProviderError> {
        let url = format!("{}/api/embeddings", self.base_url.trim_end_matches('/'));
        
        let body = json!({
            "model": self.model,
            "prompt": text,
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
                "Ollama embedding 请求失败：status={} body={}",
                status, data
            )));
        }

        // 解析响应：embedding 数组
        let embedding = data
            .get("embedding")
            .and_then(|e| e.as_array())
            .ok_or_else(|| {
                ProviderError::Message("Ollama 响应缺少 embedding 数据".to_string())
            })?
            .iter()
            .filter_map(|v| {
                // Ollama 可能返回不同数字类型
                v.as_f64().map(|f| f as f32)
                    .or_else(|| v.as_i64().map(|i| i as f32))
            })
            .collect();

        Ok(embedding)
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, ProviderError> {
        // Ollama 原生不支持批量嵌入，使用串行单条处理
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            let embedding = self.embed(text).await?;
            results.push(embedding);
        }
        Ok(results)
    }

    fn embedding_dim(&self) -> Option<usize> {
        // Ollama 模型维度因模型而异，常见值：
        // - nomic-embed-text: 768
        // - mxbai-embed-large: 1024
        // 由于模型可自定义，返回 None 表示未知
        None
    }
}
