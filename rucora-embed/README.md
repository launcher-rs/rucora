# rucora-embed

rucora 的 Embedding Provider 实现，用于将文本转换为向量，支持语义搜索和 RAG 应用。

## 支持的 Provider

| Provider | 说明 |
|----------|------|
| `OpenAiEmbeddingProvider` | 兼容 OpenAI API 格式的 Embedding 模型（支持 Ollama、vLLM 等） |
| `OllamaEmbeddingProvider` | Ollama 本地 Embedding 模型 |

## 安装

```toml
[dependencies]
rucora-embed = "0.4"
```

或通过主 rucora crate：

```toml
[dependencies]
rucora = { version = "0.4", features = ["embed"] }
```

## 使用方式

### OpenAI Embedding（兼容 OpenAI API 格式）

```rust
use rucora_core::embed::EmbeddingProvider;
use rucora_embed::openai::OpenAiEmbeddingProvider;

let provider = OpenAiEmbeddingProvider::new(
    "http://localhost:11434/v1",
    "ollama",
    "qwen3-embedding:4b",
);

let embedding = provider.embed("你好世界").await?;
println!("Embedding 维度：{}", embedding.len());

let embeddings = provider.embed_batch(&["你好，世界！".into(), "Rust 很棒。".into()]).await?;
for item in &embeddings {
    println!("维度：{}", item.len());
}
```

也可通过环境变量创建：

```rust
use rucora_embed::openai::OpenAiEmbeddingProvider;

let provider = OpenAiEmbeddingProvider::from_env()?
    .with_model("text-embedding-3-small");
```

### Ollama Embedding

```rust
use rucora_embed::ollama::OllamaEmbeddingProvider;

let provider = OllamaEmbeddingProvider::new("http://localhost:11434", "nomic-embed-text");

let embedding = provider.embed("要嵌入的文本").await?;
```

### Embedding 缓存

```rust
use rucora_core::embed::EmbeddingProvider;
use rucora_embed::cache::CachedEmbeddingProvider;
use rucora_embed::openai::OpenAiEmbeddingProvider;
use std::sync::Arc;

let inner = OpenAiEmbeddingProvider::from_env()?;
let provider = CachedEmbeddingProvider::new(inner);

// 首次调用会实际请求 API，后续相同文本直接从缓存返回
let embedding = provider.embed("你好世界").await?;
let cached = provider.embed("你好世界").await?; // 命中缓存
```

## EmbeddingProvider Trait 方法

| 方法 | 说明 |
|------|------|
| `embed(&self, text: &str)` | 嵌入单条文本，返回 `Vec<f32>` |
| `embed_batch(&self, texts: &[String])` | 批量嵌入，返回 `Vec<Vec<f32>>` |
| `embed_chunked(&self, texts: &[String], chunk_size: usize)` | 分块批量嵌入 |
| `embedding_dim(&self) -> Option<usize>` | 返回向量维度（如已知） |

辅助函数：

| 函数 | 说明 |
|------|------|
| `cosine_similarity(a, b)` | 计算余弦相似度 |
| `vector_search(query, candidates, top_k)` | 向量相似度搜索 |

## Feature 配置

| Feature | 说明 |
|---------|------|
| `openai` | OpenAI Embedding Provider（默认启用） |
| `ollama` | Ollama Embedding Provider |
| `all` | 启用所有 Provider |

## 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `OPENAI_API_KEY` | OpenAI API Key | — |
| `OPENAI_BASE_URL` | OpenAI Base URL | `https://api.openai.com/v1` |
| `EMBEDDING_MODEL` | Embedding 模型名称 | — |
| `OLLAMA_BASE_URL` | Ollama Base URL | `http://localhost:11434` |
| `OLLAMA_EMBED_MODEL` | Ollama Embedding 模型 | `nomic-embed-text` |

## 许可证

MIT
