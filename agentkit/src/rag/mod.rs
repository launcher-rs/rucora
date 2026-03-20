use agentkit_core::{
    embed::EmbeddingProvider,
    error::ProviderError,
    retrieval::{SearchResult, VectorQuery, VectorRecord, VectorStore},
};
use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub struct TextChunk {
    pub id: String,
    pub text: String,
    pub metadata: Option<Value>,
}

pub fn chunk_text(
    doc_id: &str,
    text: &str,
    max_chars: usize,
    overlap_chars: usize,
) -> Vec<TextChunk> {
    let max_chars = max_chars.max(1);
    let overlap_chars = overlap_chars.min(max_chars.saturating_sub(1));

    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();
    let mut start = 0usize;
    let mut idx = 0usize;

    while start < chars.len() {
        let end = (start + max_chars).min(chars.len());
        let chunk_text: String = chars[start..end].iter().collect();
        let id = format!("{}:{}", doc_id, idx);

        out.push(TextChunk {
            id,
            text: chunk_text,
            metadata: Some(
                json!({"doc_id": doc_id, "chunk_index": idx, "start_char": start, "end_char": end}),
            ),
        });

        if end == chars.len() {
            break;
        }

        idx += 1;
        start = end.saturating_sub(overlap_chars);
        if start >= end {
            start = end;
        }
    }

    out
}

pub async fn index_chunks<P, S>(
    embedder: &P,
    store: &S,
    chunks: &[TextChunk],
) -> Result<(), ProviderError>
where
    P: EmbeddingProvider,
    S: VectorStore,
{
    if chunks.is_empty() {
        return Ok(());
    }

    let texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
    let vectors = embedder.embed_batch(&texts).await?;

    if vectors.len() != chunks.len() {
        return Err(ProviderError::Message(
            "embed_batch 返回的向量数量与输入不一致".to_string(),
        ));
    }

    let records: Vec<VectorRecord> = chunks
        .iter()
        .zip(vectors)
        .map(|(c, v)| {
            let mut r = VectorRecord::new(c.id.clone(), v).with_text(c.text.clone());
            if let Some(md) = &c.metadata {
                r = r.with_metadata(md.clone());
            }
            r
        })
        .collect();

    store.upsert(records).await
}

pub async fn index_text<P, S>(
    embedder: &P,
    store: &S,
    doc_id: &str,
    text: &str,
    max_chars: usize,
    overlap_chars: usize,
) -> Result<Vec<TextChunk>, ProviderError>
where
    P: EmbeddingProvider,
    S: VectorStore,
{
    let chunks = chunk_text(doc_id, text, max_chars, overlap_chars);
    index_chunks(embedder, store, &chunks).await?;
    Ok(chunks)
}

#[derive(Debug, Clone)]
pub struct Citation {
    pub doc_id: Option<String>,
    pub chunk_id: String,
    pub score: f32,
    pub text: Option<String>,
    pub metadata: Option<Value>,
}

impl Citation {
    pub fn render(&self) -> String {
        match &self.doc_id {
            Some(d) => format!("{}:{}", d, self.chunk_id),
            None => self.chunk_id.clone(),
        }
    }
}

fn search_result_to_citation(r: SearchResult) -> Citation {
    let doc_id = r
        .metadata
        .as_ref()
        .and_then(|m| m.get("doc_id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Citation {
        doc_id,
        chunk_id: r.id,
        score: r.score,
        text: r.text,
        metadata: r.metadata,
    }
}

pub async fn retrieve<P, S>(
    embedder: &P,
    store: &S,
    query_text: &str,
    top_k: usize,
) -> Result<Vec<Citation>, ProviderError>
where
    P: EmbeddingProvider,
    S: VectorStore,
{
    let vector = embedder.embed(query_text).await?;
    let query = VectorQuery::new(vector).with_top_k(top_k);
    let results = store.search(query).await?;
    Ok(results.into_iter().map(search_result_to_citation).collect())
}
