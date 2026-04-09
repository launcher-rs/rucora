# AgentKit Retrieval

AgentKit 的向量存储与检索实现。

## 概述

本 crate 为 AgentKit 提供 VectorStore 实现，用于向量存储、相似度搜索和检索，支持语义搜索和 RAG 应用。

## 支持的 VectorStore

| VectorStore | 说明 |
|-------------|------|
| InMemoryVectorStore | 内存向量存储 |
| ChromaVectorStore | ChromaDB 集成 |
| ChromaPersistentVectorStore | 持久化 ChromaDB 存储 |
| QdrantVectorStore | Qdrant 集成 |

## 安装

```toml
[dependencies]
agentkit-retrieval = "0.1"
```

或通过主 AgentKit crate：

```toml
[dependencies]
agentkit = { version = "0.1", features = ["retrieval"] }
```

## 使用方式

### 内存向量存储

```rust
use agentkit_retrieval::in_memory::InMemoryVectorStore;
use agentkit_core::retrieval::{VectorStore, VectorRecord};

let store = InMemoryVectorStore::new();

// 添加向量
store.add(vec![VectorRecord {
    id: "doc1".to_string(),
    embedding: vec![0.1, 0.2, 0.3],
    metadata: serde_json::json!({"source": "file.txt"}),
}]).await?;

// 搜索
let results = store.search(
    &vec![0.1, 0.2, 0.3],
    5,
    None,
).await?;
```

### Chroma 向量存储

```rust
use agentkit_retrieval::chroma::ChromaVectorStore;

let store = ChromaVectorStore::new("http://localhost:8000", "my_collection")?;

// 创建集合（如果不存在）
store.ensure_collection().await?;

// 添加和搜索向量
store.add(records).await?;
let results = store.search(&query_embedding, 10, None).await?;
```

### 持久化 Chroma 向量存储

```rust
use agentkit_retrieval::chroma_persistent::ChromaPersistentVectorStore;

let store = ChromaPersistentVectorStore::new(
    "http://localhost:8000",
    "my_collection",
    "/path/to/cache"
)?;
```

### Qdrant 向量存储

```rust
use agentkit_retrieval::qdrant::QdrantVectorStore;

let store = QdrantVectorStore::new(
    "http://localhost:6333",
    "my_collection",
    1536  // 向量维度
)?;

store.ensure_collection().await?;
```

## Feature 配置

| Feature | 说明 |
|---------|------|
| `in-memory` | 内存向量存储（默认） |
| `chroma` | ChromaDB 集成 |
| `all` | 启用所有向量存储 |

## 许可证

MIT
