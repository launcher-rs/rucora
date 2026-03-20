use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use agentkit_core::{embed::EmbeddingProvider, error::ProviderError};
use async_trait::async_trait;

pub struct CachedEmbeddingProvider<P> {
    inner: Arc<P>,
    cache: Mutex<HashMap<String, Vec<f32>>>,
}

impl<P> CachedEmbeddingProvider<P> {
    pub fn new(inner: P) -> Self {
        Self {
            inner: Arc::new(inner),
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn new_arc(inner: Arc<P>) -> Self {
        Self {
            inner,
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn inner(&self) -> Arc<P> {
        self.inner.clone()
    }

    fn validate_dim(&self, v: &[f32]) -> Result<(), ProviderError>
    where
        P: EmbeddingProvider,
    {
        if let Some(dim) = self.inner.embedding_dim() {
            if v.len() != dim {
                return Err(ProviderError::Message(format!(
                    "embedding_dim 校验失败：expected={} got={}",
                    dim,
                    v.len()
                )));
            }
        }
        Ok(())
    }
}

#[async_trait]
impl<P> EmbeddingProvider for CachedEmbeddingProvider<P>
where
    P: EmbeddingProvider,
{
    async fn embed(&self, text: &str) -> Result<Vec<f32>, ProviderError> {
        if let Ok(cache) = self.cache.lock() {
            if let Some(v) = cache.get(text) {
                return Ok(v.clone());
            }
        }

        let v = self.inner.embed(text).await?;
        self.validate_dim(&v)?;

        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(text.to_string(), v.clone());
        }

        Ok(v)
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, ProviderError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let mut out: Vec<Vec<f32>> = vec![Vec::new(); texts.len()];
        let mut missing_texts: Vec<String> = Vec::new();
        let mut missing_pos: Vec<usize> = Vec::new();

        if let Ok(cache) = self.cache.lock() {
            for (i, t) in texts.iter().enumerate() {
                if let Some(v) = cache.get(t) {
                    out[i] = v.clone();
                } else {
                    missing_texts.push(t.clone());
                    missing_pos.push(i);
                }
            }
        } else {
            missing_texts.extend_from_slice(texts);
            missing_pos.extend(0..texts.len());
        }

        if !missing_texts.is_empty() {
            let got = self.inner.embed_batch(&missing_texts).await?;
            if got.len() != missing_texts.len() {
                return Err(ProviderError::Message(
                    "embed_batch 返回的向量数量与输入不一致".to_string(),
                ));
            }

            for (j, v) in got.into_iter().enumerate() {
                self.validate_dim(&v)?;
                let pos = missing_pos[j];
                out[pos] = v.clone();
                if let Ok(mut cache) = self.cache.lock() {
                    cache.insert(missing_texts[j].clone(), v);
                }
            }
        }

        Ok(out)
    }

    fn embedding_dim(&self) -> Option<usize> {
        self.inner.embedding_dim()
    }
}
