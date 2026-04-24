# 向量检索 (Embedding & Retrieval)

向量检索系统提供文本嵌入（Embedding）和语义检索（Retrieval）能力，是 RAG（检索增强生成）的基础。

## 模块结构

```
agentkit-embed/src/          # Embedding Provider
├── openai.rs                # OpenAI 嵌入服务
├── ollama.rs                # Ollama 嵌入服务
└── cache.rs                 # 带缓存的嵌入服务

agentkit-retrieval/src/      # VectorStore 实现
├── in_memory.rs             # 内存向量存储
├── chroma.rs                # ChromaDB 向量存储
├── chroma_persistent.rs     # ChromaDB 持久化版
├── memory.rs                # 内存向量存储（另一种实现）
└── qdrant.rs                # Qdrant 向量存储
```

## Embedding Provider

### OpenAiEmbedding

```rust
use agentkit::embed::OpenAiEmbedding;

let embedding = OpenAiEmbedding::from_env()?;
let vector = embedding.embed("Hello World").await?;
println!("向量维度: {}", vector.len());  // 通常 1536 维
```

### OllamaEmbedding

```rust
use agentkit::embed::OllamaEmbedding;

let embedding = OllamaEmbedding::from_env();
let vector = embedding.embed("Hello World").await?;
```

### CachedEmbeddingProvider

带缓存的嵌入 Provider，避免重复计算相同文本。

```rust
use agentkit::embed::{CachedEmbeddingProvider, OpenAiEmbedding};

let inner = OpenAiEmbedding::from_env()?;
let cached = CachedEmbeddingProvider::new(inner);

// 首次调用：计算并缓存
let v1 = cached.embed("Hello").await?;
// 第二次调用：直接返回缓存结果
let v2 = cached.embed("Hello").await?;
```

---

## VectorStore

### InMemoryVectorStore

内存向量存储，适合开发和测试。

```rust
use agentkit::retrieval::InMemoryVectorStore;
use agentkit_core::retrieval::{VectorRecord, VectorQuery};

let store = InMemoryVectorStore::new();

// 插入向量
let records = vec![
    VectorRecord {
        id: "doc1".to_string(),
        embedding: vec![0.1, 0.2, 0.3],
        metadata: serde_json::json!({"text": "Hello World"}),
    },
];
store.upsert(records).await?;

// 语义搜索
let query = VectorQuery {
    embedding: vec![0.15, 0.25, 0.35],
    top_k: 5,
    filter: None,
};
let results = store.search(query).await?;
for r in results {
    println!("ID: {}, Score: {}", r.id, r.score);
}
```

### ChromaVectorStore / ChromaPersistentVectorStore

ChromaDB 向量存储（内存版 / 持久化版）。

```rust
use agentkit::retrieval::{ChromaVectorStore, ChromaPersistentVectorStore};

// 内存版
let store = ChromaVectorStore::new("collection_name")?;

// 持久化版（数据保存到磁盘）
let store = ChromaPersistentVectorStore::new("./chroma_data", "collection_name")?;
```

### QdrantVectorStore

Qdrant 向量数据库。

```rust
use agentkit::retrieval::QdrantVectorStore;

let store = QdrantVectorStore::new(
    "http://localhost:6333",
    "collection_name",
    1536,  // 向量维度
)?;
```

---

## 完整 RAG 流程

RAG（检索增强生成）结合 Embedding 和 Retrieval：

```rust
use agentkit::embed::OpenAiEmbedding;
use agentkit::retrieval::InMemoryVectorStore;
use agentkit_core::retrieval::{VectorRecord, VectorQuery};

// 1. 初始化
let embedding = OpenAiEmbedding::from_env()?;
let store = InMemoryVectorStore::new();

// 2. 索引文档
let documents = vec![
    "Rust 是一门系统编程语言",
    "Python 适合快速开发和数据科学",
    "Go 适合构建微服务",
];

for (i, doc) in documents.iter().enumerate() {
    let vector = embedding.embed(doc).await?;
    store.upsert(vec![VectorRecord {
        id: format!("doc_{}", i),
        embedding: vector,
        metadata: serde_json::json!({"text": doc}),
    }]).await?;
}

// 3. 检索
let query_vector = embedding.embed("什么语言适合系统编程？").await?;
let results = store.search(VectorQuery {
    embedding: query_vector,
    top_k: 2,
    filter: None,
}).await?;

for r in results {
    let text = r.metadata["text"].as_str().unwrap();
    println!("相关文档: {} (相关度: {:.3})", text, r.score);
}
```

---

## 选择指南

| 场景 | 推荐组合 |
|------|---------|
| 开发/测试 | `OpenAiEmbedding` + `InMemoryVectorStore` |
| 本地部署 | `OllamaEmbedding` + `ChromaPersistentVectorStore` |
| 生产环境 | `OpenAiEmbedding` + `QdrantVectorStore` |
| 带缓存 | `CachedEmbeddingProvider` + `InMemoryVectorStore` |
