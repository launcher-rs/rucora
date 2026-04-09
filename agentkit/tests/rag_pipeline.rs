use agentkit::rag::{index_text, retrieve};
use agentkit_retrieval::in_memory::InMemoryVectorStore;
use agentkit_core::{embed::EmbeddingProvider, error::ProviderError};
use async_trait::async_trait;

struct DeterministicEmbeddingProvider {
    dim: usize,
}

impl DeterministicEmbeddingProvider {
    fn new(dim: usize) -> Self {
        Self { dim }
    }

    fn embed_impl(&self, text: &str) -> Vec<f32> {
        let mut v = vec![0f32; self.dim];
        if self.dim == 0 {
            return v;
        }
        for (i, b) in text.as_bytes().iter().enumerate() {
            let idx = i % self.dim;
            v[idx] += (*b as f32) / 255.0;
        }
        v
    }
}

#[async_trait]
impl EmbeddingProvider for DeterministicEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, ProviderError> {
        Ok(self.embed_impl(text))
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, ProviderError> {
        Ok(texts.iter().map(|t| self.embed_impl(t)).collect())
    }

    fn embedding_dim(&self) -> Option<usize> {
        Some(self.dim)
    }
}

#[tokio::test]
async fn rag_pipeline_should_index_retrieve_and_cite() {
    let embedder = DeterministicEmbeddingProvider::new(8);
    let store = InMemoryVectorStore::new();

    let doc_id = "doc1";
    let text = "Rust is a systems programming language. AgentKit provides tools and runtime.";

    let chunks = index_text(&embedder, &store, doc_id, text, 32, 8)
        .await
        .expect("index_text should work");
    assert!(!chunks.is_empty());

    let cites = retrieve(&embedder, &store, "AgentKit runtime", 3)
        .await
        .expect("retrieve should work");
    assert!(!cites.is_empty());

    // 至少返回一个可引用 chunk。
    let c0 = &cites[0];
    assert!(!c0.chunk_id.is_empty());
}
