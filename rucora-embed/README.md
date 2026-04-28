# rucora Embed

rucora 的 Embedding Provider 实现。

## 概述

本 crate 为 rucora 提供 Embedding Provider 实现，用于将文本转换为向量，支持语义搜索和 RAG 应用。

## 支持的 Provider

| Provider | 说明 |
|----------|------|
| OpenAiEmbeddingProvider | OpenAI Embedding 模型 |
| OllamaEmbeddingProvider | Ollama 本地 Embedding 模型 |

## 安装

```toml
[dependencies]
rucora-embed = "0.1"
```

或通过主 rucora crate：

```toml
[dependencies]
rucora = { version = "0.1", features = ["embed"] }
```

## 使用方式

### OpenAI Embedding

```rust
use rucora_embed::openai::OpenAiEmbeddingProvider;
use rucora_core::embed::EmbeddingProvider;

let provider = OpenAiEmbeddingProvider::from_env()?
    .with_model("text-embedding-3-small");

let embeddings = provider.embed(&["你好，世界！", "Rust 很棒。"]).await?;

println!("Embedding 维度：{}", embeddings[0].len());
```

### Ollama Embedding

```rust
use rucora_embed::ollama::OllamaEmbeddingProvider;

let provider = OllamaEmbeddingProvider::new("http://localhost:11434")
    .with_model("nomic-embed-text");

let embeddings = provider.embed(&["要嵌入的文本"]).await?;
```

### Embedding 缓存

```rust
use rucora_embed::cache::EmbeddingCache;

let mut cache = EmbeddingCache::with_capacity(1000);

// 缓存会自动存储和检索 Embedding
let embeddings = cache.get_or_compute("你好，世界！", |text| async move {
    // 计算 Embedding
    vec![0.1, 0.2, 0.3]
}).await?;
```

## Feature 配置

| Feature | 说明 |
|---------|------|
| `openai` | OpenAI Embedding Provider（默认） |
| `ollama` | Ollama Embedding Provider |
| `all` | 启用所有 Embedding Provider |

## 环境变量

| 变量 | 说明 |
|------|------|
| `OPENAI_API_KEY` | OpenAI API Key |
| `OPENAI_BASE_URL` | OpenAI Base URL |
| `OLLAMA_BASE_URL` | Ollama Base URL |

## 许可证

MIT
