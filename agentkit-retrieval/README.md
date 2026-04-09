# AgentKit Retrieval

Vector store retrieval for AgentKit.

## Overview

This crate provides VectorStore implementations for AgentKit, enabling vector storage, similarity search, and retrieval for semantic search and RAG applications.

## Supported VectorStores

| VectorStore | Description |
|-------------|-------------|
| InMemoryVectorStore | In-memory vector storage |
| ChromaVectorStore | ChromaDB integration |
| ChromaPersistentVectorStore | Persistent ChromaDB storage |
| QdrantVectorStore | Qdrant integration |

## Installation

```toml
[dependencies]
agentkit-retrieval = "0.1"
```

Or via the main AgentKit crate:

```toml
[dependencies]
agentkit = { version = "0.1", features = ["retrieval"] }
```

## Usage

### In-Memory Vector Store

```rust
use agentkit_retrieval::in_memory::InMemoryVectorStore;
use agentkit_core::retrieval::{VectorStore, VectorRecord};

let store = InMemoryVectorStore::new();

// Add vectors
store.add(vec![VectorRecord {
    id: "doc1".to_string(),
    embedding: vec![0.1, 0.2, 0.3],
    metadata: serde_json::json!({"source": "file.txt"}),
}]).await?;

// Search
let results = store.search(
    &vec![0.1, 0.2, 0.3],
    5,
    None,
).await?;
```

### Chroma Vector Store

```rust
use agentkit_retrieval::chroma::ChromaVectorStore;

let store = ChromaVectorStore::new("http://localhost:8000", "my_collection")?;

// Create collection if not exists
store.ensure_collection().await?;

// Add and search vectors
store.add(records).await?;
let results = store.search(&query_embedding, 10, None).await?;
```

### Persistent Chroma Vector Store

```rust
use agentkit_retrieval::chroma_persistent::ChromaPersistentVectorStore;

let store = ChromaPersistentVectorStore::new(
    "http://localhost:8000",
    "my_collection",
    "/path/to/cache"
)?;
```

### Qdrant Vector Store

```rust
use agentkit_retrieval::qdrant::QdrantVectorStore;

let store = QdrantVectorStore::new(
    "http://localhost:6333",
    "my_collection",
    1536  // vector dimension
)?;

store.ensure_collection().await?;
```

## Features

| Feature | Description |
|---------|-------------|
| `in-memory` | In-memory vector store (default) |
| `chroma` | ChromaDB integration |
| `persistent` | Persistent storage support |
| `all` | Enable all vector stores |

## License

MIT
