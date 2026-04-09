use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use agentkit_core::{embed::EmbeddingProvider, error::ProviderError};
use async_trait::async_trait;

// EmbeddingProvider 的简单内存缓存包装器：
// - 对单条 `embed`：以 `text` 作为 key 缓存向量结果
// - 对批量 `embed_batch`：优先从缓存命中，未命中的部分再交给 inner 计算并回填缓存
// - 使用 `Mutex<HashMap<..>>` 做线程安全保护；锁获取失败时会退化为不读/不写缓存
pub struct CachedEmbeddingProvider<P> {
    // 实际执行 embedding 的底层 provider
    inner: Arc<P>,
    // 进程内缓存（key 为原始文本，value 为 embedding 向量）
    cache: Mutex<HashMap<String, Vec<f32>>>,
}

impl<P> CachedEmbeddingProvider<P> {
    // 通过传入一个 provider 创建缓存包装器（内部会包一层 Arc 便于 clone）
    pub fn new(inner: P) -> Self {
        Self {
            inner: Arc::new(inner),
            cache: Mutex::new(HashMap::new()),
        }
    }

    // 直接用已有的 Arc provider 创建缓存包装器
    pub fn new_arc(inner: Arc<P>) -> Self {
        Self {
            inner,
            cache: Mutex::new(HashMap::new()),
        }
    }

    // 获取底层 provider 的 Arc（用于外部复用/共享同一个 provider）
    pub fn inner(&self) -> Arc<P> {
        self.inner.clone()
    }

    // 若底层 provider 声明了固定维度，则对返回向量做维度校验，避免混入错误数据到缓存。
    fn validate_dim(&self, v: &[f32]) -> Result<(), ProviderError>
    where
        P: EmbeddingProvider,
    {
        if let Some(dim) = self.inner.embedding_dim()
            && v.len() != dim {
                return Err(ProviderError::Message(format!(
                    "embedding_dim 校验失败：expected={} got={}",
                    dim,
                    v.len()
                )));
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
        // 先尝试从缓存读取；如果锁获取失败，则直接跳过缓存读取（保证功能可用）。
        if let Ok(cache) = self.cache.lock()
            && let Some(v) = cache.get(text) {
                return Ok(v.clone());
            }

        // 未命中则调用底层 provider 计算
        let v = self.inner.embed(text).await?;
        // 写入缓存前先校验维度，避免缓存不合法向量
        self.validate_dim(&v)?;

        // 尝试写回缓存；锁获取失败时静默跳过（不影响返回结果）
        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(text.to_string(), v.clone());
        }

        Ok(v)
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, ProviderError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        // 输出向量与输入 texts 一一对应：out[i] 对应 texts[i]
        let mut out: Vec<Vec<f32>> = vec![Vec::new(); texts.len()];
        // missing_* 用于记录“未命中缓存”的文本及其在 out 中的位置，稍后批量补齐
        let mut missing_texts: Vec<String> = Vec::new();
        let mut missing_pos: Vec<usize> = Vec::new();

        // 尽量在一次加锁期间完成所有读取，降低锁竞争
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
            // 锁获取失败则视为全部未命中：直接走底层批量计算
            missing_texts.extend_from_slice(texts);
            missing_pos.extend(0..texts.len());
        }

        if !missing_texts.is_empty() {
            // 仅对缺失部分调用底层 provider，减少重复计算
            let got = self.inner.embed_batch(&missing_texts).await?;
            if got.len() != missing_texts.len() {
                return Err(ProviderError::Message(
                    "embed_batch 返回的向量数量与输入不一致".to_string(),
                ));
            }

            for (j, v) in got.into_iter().enumerate() {
                self.validate_dim(&v)?;
                // 将缺失项回填到对应位置，保证输出顺序与输入一致
                let pos = missing_pos[j];
                out[pos] = v.clone();
                // 同时回写缓存（失败则跳过，不影响返回）
                if let Ok(mut cache) = self.cache.lock() {
                    cache.insert(missing_texts[j].clone(), v);
                }
            }
        }

        Ok(out)
    }

    fn embedding_dim(&self) -> Option<usize> {
        // 缓存层不改变维度信息，直接透传
        self.inner.embedding_dim()
    }
}
