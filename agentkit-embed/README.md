# AgentKit Embed

Embedding providers for AgentKit.

## Overview

This crate provides embedding provider implementations for AgentKit, enabling text-to-vector conversion for semantic search and RAG applications.

## Supported Providers

| Provider | Description |
|----------|-------------|
| OpenAiEmbedProvider | OpenAI embedding models |
| OllamaEmbedProvider | Ollama local embedding models |

## Installation

```toml
[dependencies]
agentkit-embed = "0.1"
```

Or via the main AgentKit crate:

```toml
[dependencies]
agentkit = { version = "0.1", features = ["embed"] }
```

## Usage

### OpenAI Embedding

```rust
use agentkit_embed::openai::OpenAiEmbedProvider;
use agentkit_core::embed::EmbeddingProvider;

let provider = OpenAiEmbedProvider::from_env()?
    .with_model("text-embedding-3-small");

let embeddings = provider.embed(&["Hello, world!", "Rust is great."]).await?;

println!("Embedding dimension: {}", embeddings[0].len());
```

### Ollama Embedding

```rust
use agentkit_embed::ollama::OllamaEmbedProvider;

let provider = OllamaEmbedProvider::new("http://localhost:11434")
    .with_model("nomic-embed-text");

let embeddings = provider.embed(&["text to embed"]).await?;
```

### Embedding Cache

```rust
use agentkit_embed::cache::EmbeddingCache;

let mut cache = EmbeddingCache::with_capacity(1000);

// Cache will automatically store and retrieve embeddings
let embeddings = cache.get_or_compute("Hello, world!", |text| async move {
    // Compute embedding
    vec![0.1, 0.2, 0.3]
}).await?;
```

## Features

| Feature | Description |
|---------|-------------|
| `openai` | OpenAI Embedding Provider (default) |
| `ollama` | Ollama Embedding Provider |
| `all` | Enable all embedding providers |

## Environment Variables

| Variable | Description |
|----------|-------------|
| `OPENAI_API_KEY` | OpenAI API Key |
| `OPENAI_BASE_URL` | OpenAI Base URL |
| `OLLAMA_BASE_URL` | Ollama Base URL |

## License

MIT
