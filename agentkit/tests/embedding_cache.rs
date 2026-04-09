use agentkit_embed::cache::CachedEmbeddingProvider;
use agentkit_core::{embed::EmbeddingProvider, error::ProviderError};
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

struct CountingEmbeddingProvider {
    calls: Arc<AtomicUsize>,
    dim: usize,
    wrong_dim: bool,
}

#[async_trait]
impl EmbeddingProvider for CountingEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, ProviderError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let dim = if self.wrong_dim {
            self.dim.saturating_sub(1)
        } else {
            self.dim
        };
        let mut v = vec![0f32; dim];
        if dim > 0 {
            v[0] = text.len() as f32;
        }
        Ok(v)
    }

    fn embedding_dim(&self) -> Option<usize> {
        Some(self.dim)
    }
}

#[tokio::test]
async fn cached_embedding_provider_should_cache_single_embed() {
    let calls = Arc::new(AtomicUsize::new(0));
    let inner = CountingEmbeddingProvider {
        calls: calls.clone(),
        dim: 4,
        wrong_dim: false,
    };

    let cached = CachedEmbeddingProvider::new(inner);

    let _ = cached.embed("hello").await.expect("first embed");
    let _ = cached.embed("hello").await.expect("second embed");

    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn cached_embedding_provider_should_validate_dim() {
    let calls = Arc::new(AtomicUsize::new(0));
    let inner = CountingEmbeddingProvider {
        calls: calls.clone(),
        dim: 4,
        wrong_dim: true,
    };

    let cached = CachedEmbeddingProvider::new(inner);

    let err = cached.embed("hello").await.err().expect("should err");
    let msg = err.to_string();
    assert!(msg.contains("embedding_dim"));
}
