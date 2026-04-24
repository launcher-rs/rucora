# RAG (检索增强生成) 指南

AgentKit 提供了完整的 RAG (Retrieval-Augmented Generation) 管道，支持文档分块、向量化、索引、检索和引用溯源。

## 概述

RAG 系统通过以下步骤增强 LLM 回答：
1. **文档分块** - 将长文档切分为可管理的文本块
2. **向量化** - 使用 Embedding 模型将文本转换为向量
3. **索引存储** - 将向量存入向量数据库
4. **语义检索** - 根据用户查询检索相关上下文
5. **引用生成** - 提供带引用来源的 LLM 回答

## 架构

RAG 系统分布在多个 crate 中：

| Crate | 模块 | 职责 |
|-------|------|------|
| `agentkit` | `rag` | 核心 RAG 管道（分块、索引、检索、引用） |
| `agentkit-core` | `embed` | `EmbeddingProvider` trait |
| `agentkit-core` | `retrieval` | `VectorStore` trait |
| `agentkit-embed` | - | Embedding 提供者实现 |
| `agentkit-retrieval` | - | 向量存储实现 |

## 核心类型

### TextChunk

表示分块后的文本片段：

```rust
pub struct TextChunk {
    pub id: String,              // 格式: "{doc_id}:{chunk_index}"
    pub text: String,            // 块内容
    pub metadata: Option<Value>, // 包含 doc_id, chunk_index, start_char, end_char
}
```

### Citation

表示检索到的结果及引用来源：

```rust
pub struct Citation {
    pub doc_id: Option<String>,  // 文档 ID
    pub chunk_id: String,        // 块 ID (例如 "doc1:0")
    pub score: f32,              // 相似度分数（越高越相似）
    pub text: Option<String>,    // 块内容
    pub metadata: Option<Value>, // 完整元数据
}
```

`Citation::render()` - 渲染引用标签，格式 `"{doc_id}:{chunk_id}"`

## 核心函数

### chunk_text

将文本切分为重叠的块（同步）：

```rust
pub fn chunk_text(
    doc_id: &str,
    text: &str,
    max_chars: usize,      // 最大字符数
    overlap_chars: usize,  // 重叠字符数
) -> Vec<TextChunk>
```

**特性：**
- 基于字符切分（正确处理 Unicode）
- 块 ID 格式：`{doc_id}:{index}` (例如 "doc1:0", "doc1:1")
- 元数据包含：`doc_id`, `chunk_index`, `start_char`, `end_char`
- 安全处理：`max_chars` 最小为 1，`overlap_chars` 最大为 `max_chars - 1`

### index_chunks

向量化并索引预分块的文本（异步）：

```rust
pub async fn index_chunks<P, S>(
    embedder: &P,
    store: &S,
    chunks: &[TextChunk],
) -> Result<(), ProviderError>
where
    P: EmbeddingProvider,
    S: VectorStore,
```

使用 `embed_batch` 进行高效批量向量化。

### index_text

便捷函数：一步完成分块 + 索引（异步）：

```rust
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
```

内部调用 `chunk_text` 然后 `index_chunks`。

### retrieve

向量化查询并检索相似块（异步）：

```rust
pub async fn retrieve<P, S>(
    embedder: &P,
    store: &S,
    query_text: &str,
    top_k: usize,
) -> Result<Vec<Citation>, ProviderError>
where
    P: EmbeddingProvider,
    S: VectorStore,
```

## 完整示例

### 示例 1：基本 RAG 管道

```rust
use agentkit::rag::{index_text, retrieve};
use agentkit::agent::SimpleAgent;
use agentkit::prelude::Agent;
use agentkit::provider::OpenAiProvider;
use agentkit_embed::openai::OpenAiEmbeddingProvider;
use agentkit_retrieval::in_memory::InMemoryVectorStore;

// 步骤 1：设置组件
let vector_store = InMemoryVectorStore::new();
let embedder = OpenAiEmbeddingProvider::from_env()?;
let llm_provider = OpenAiProvider::from_env()?;

// 步骤 2：索引文档
let chunks = index_text(
    &embedder,
    &vector_store,
    "rust_ownership",
    "Rust 的所有权系统是内存安全的保证。每个值都有一个变量作为其所有者...",
    200,  // 每块 200 字符
    30,   // 重叠 30 字符
).await?;

println!("创建了 {} 个块", chunks.len());

// 步骤 3：检索相关上下文
let citations = retrieve(
    &embedder,
    &vector_store,
    "Rust 的所有权是如何工作的？",
    3,  // 返回前 3 个最相关结果
).await?;

// 步骤 4：构建上下文字符串
let context = citations.iter()
    .map(|c| format!("[{}]: {}", c.render(), c.text.as_ref().unwrap_or("无内容")))
    .collect::<Vec<_>>()
    .join("\n\n");

// 步骤 5：创建带上下文的提示
let enhanced_prompt = format!(
    "请根据以下上下文信息回答问题。\n\
     === 上下文 ===\n{}\n\n\
     === 问题 ===\n{}\n\n\
     请提供详细、准确的回答，并引用相关来源。",
    context, "Rust 的所有权系统是如何工作的？"
);

// 步骤 6：使用 LLM 生成回答
let agent = SimpleAgent::builder()
    .provider(llm_provider)
    .model("gpt-4o-mini")
    .system_prompt("你是一个基于检索结果的问答助手。请根据提供的上下文信息回答问题，并引用来源。")
    .build();

let output = agent.run(enhanced_prompt.into()).await?;
println!("回答：{}", output);
```

### 示例 2：索引多个文档

```rust
use agentkit::rag::{chunk_text, index_chunks, index_text, retrieve};

let vector_store = InMemoryVectorStore::new();
let embedder = OpenAiEmbeddingProvider::from_env()?;

// 方法 1：使用 index_text（自动分块并索引）
let rust_chunks = index_text(
    &embedder, &vector_store,
    "rust_doc",
    &rust_language_document,
    200, 30
).await?;

let python_chunks = index_text(
    &embedder, &vector_store,
    "python_doc",
    &python_language_document,
    150, 20
).await?;

// 方法 2：手动分块然后索引（更灵活）
let chunks = chunk_text("go_doc", &go_document, 180, 25);
index_chunks(&embedder, &vector_store, &chunks).await?;

// 检索时会自动搜索所有文档
let citations = retrieve(
    &embedder, &vector_store,
    "哪种语言最适合系统编程？",
    5
).await?;

for citation in &citations {
    println!("[{}] 分数: {:.3}", citation.render(), citation.score);
}
```

### 示例 3：使用缓存 Embedding

```rust
use agentkit::rag::{index_text, retrieve};
use agentkit_embed::cache::CachedEmbeddingProvider;
use agentkit_embed::openai::OpenAiEmbeddingProvider;
use agentkit_retrieval::in_memory::InMemoryVectorStore;

// 包装 OpenAI Embedding 提供者为缓存版本
let inner = OpenAiEmbeddingProvider::from_env()?;
let cached = CachedEmbeddingProvider::new(inner);
let store = InMemoryVectorStore::new();

// 索引文档（嵌入会被缓存）
index_text(&cached, &store, "doc1", "Document content...", 500, 50).await?;

// 检索（查询嵌入也会被缓存）
let citations = retrieve(&cached, &store, "query", 5).await?;

// 重复相同的查询会直接使用缓存，无需 API 调用
let cached_citations = retrieve(&cached, &store, "query", 5).await?;
```

## 分块策略

### 推荐大小

| 文本大小 | 是否需要分块 | 推荐配置 |
|----------|-------------|---------|
| < 1000 字符 | 否 | 无需分块 |
| 1000-5000 字符 | 是 | 500 字符/块，50 字符重叠 |
| > 5000 字符 | 是 | 1000 字符/块，100 字符重叠 |

### 重叠设置

重叠应占块大小的 10-20%，以保持上下文连续性：

```rust
// 小块：10% 重叠
chunk_text("doc1", &text, 200, 20);   // 200 字符，20 重叠

// 中等块：15% 重叠
chunk_text("doc2", &text, 500, 75);   // 500 字符，75 重叠

// 大块：20% 重叠
chunk_text("doc3", &text, 1000, 200); // 1000 字符，200 重叠
```

## 检索配置

### TopK 选择

| 查询类型 | 推荐 top_k |
|----------|-----------|
| 精确查询 | 3-5 |
| 模糊查询 | 5-10 |
| 探索性查询 | 10-20 |

## 向量存储选择

| 场景 | 推荐组合 |
|------|---------|
| 开发/测试 | `OpenAiEmbeddingProvider` + `InMemoryVectorStore` |
| 本地部署 | `OllamaEmbeddingProvider` + `ChromaPersistentStore` |
| 生产环境 | `OpenAiEmbeddingProvider` + `QdrantVectorStore` |
| 带缓存 | `CachedEmbeddingProvider` + 任意 VectorStore |

## VectorStore 实现

### InMemoryVectorStore

```rust
use agentkit_retrieval::in_memory::InMemoryVectorStore;

let store = InMemoryVectorStore::new();
```

- 内存存储，使用 `Arc<RwLock<HashMap>>`
- Tokio 异步安全
- 余弦相似度计算
- 适合开发和测试

### ChromaVectorStore

```rust
use agentkit_retrieval::chroma::ChromaVectorStore;

let store = ChromaVectorStore::from_env()?;
```

- ChromaDB HTTP API
- 相似度分数：`1/(1+distance)`
- 环境变量：`CHROMA_URL`, `CHROMA_COLLECTION`, `CHROMA_TENANT`, `CHROMA_DATABASE`

### ChromaPersistentStore

```rust
use agentkit_retrieval::chroma_persistent::ChromaPersistentStore;

let store = ChromaPersistentStore::from_env()?;
```

- 本地 JSON 文件持久化
- 余弦相似度
- 环境变量：`CHROMA_PERSIST_DIR`

### QdrantVectorStore

```rust
use agentkit_retrieval::qdrant::QdrantVectorStore;

let store = QdrantVectorStore::from_env()?;
```

- Qdrant HTTP API
- 原生相似度分数
- 环境变量：`QDRANT_URL`, `QDRANT_API_KEY`, `QDRANT_COLLECTION`

## Embedding 提供者

### OpenAI

```rust
use agentkit_embed::openai::OpenAiEmbeddingProvider;

let embedder = OpenAiEmbeddingProvider::from_env()?;
```

环境变量：
- `OPENAI_API_KEY` (必需)
- `OPENAI_BASE_URL` (默认 `https://api.openai.com/v1`)
- `EMBEDDING_MODEL` (必需)

已知维度映射：
- `text-embedding-ada-002` / `text-embedding-3-small` = 1536
- `text-embedding-3-large` = 3072

### Ollama

```rust
use agentkit_embed::ollama::OllamaEmbeddingProvider;

let embedder = OllamaEmbeddingProvider::from_env()?;
```

环境变量：
- `OLLAMA_BASE_URL` (默认 `http://localhost:11434`)
- `OLLAMA_EMBED_MODEL` (默认 `nomic-embed-text`)

### Cached

```rust
use agentkit_embed::cache::CachedEmbeddingProvider;

let cached = CachedEmbeddingProvider::new(inner_embedder);
```

- 包装任意 `EmbeddingProvider`
- 内存 `HashMap` 缓存
- 线程安全
- `embed_batch` 优化：一次锁定读取所有缓存命中

## 性能优化

1. **使用 `CachedEmbeddingProvider`** - 减少重复嵌入的 API 调用
2. **使用 `embed_batch`** - 批量向量化比单次调用更高效
3. **定期清理向量存储** - 使用 `clear()` 清理不再需要的数据
4. **合适的重叠比例** - 10-20% 重叠保持上下文连续性
5. **选择合适的 top_k** - 避免检索过多不必要的块

## 与 Agent 集成

RAG 系统通过 **检索-然后-提示** 模式与任意 Agent 类型集成：

```rust
use agentkit::rag::{index_text, retrieve};
use agentkit::agent::ToolAgent;
use agentkit::prelude::*;

// 1. 索引阶段（通常一次性完成）
let chunks = index_text(&embedder, &store, "docs", &documentation, 500, 50).await?;

// 2. 查询阶段（每次用户请求）
let citations = retrieve(&embedder, &store, &user_query, 5).await?;

// 3. 构建上下文
let context = citations.iter()
    .map(|c| format!("[{}]: {}", c.render(), c.text.as_deref().unwrap_or("")))
    .collect::<Vec<_>>()
    .join("\n");

// 4. 创建增强提示
let prompt = format!(
    "基于以下上下文回答问题：\n{}\n\n问题：{}",
    context, user_query
);

// 5. 传递给任意 Agent 类型
let agent = ToolAgent::builder()
    .provider(llm_provider)
    .model("gpt-4o-mini")
    .system_prompt("你是基于检索结果的问答助手。")
    .build();

let response = agent.run(prompt.into()).await?;
```

## 环境变量参考

| 变量 | 提供者/存储 | 默认值 |
|------|-----------|--------|
| `OPENAI_API_KEY` | OpenAiEmbeddingProvider | (必需) |
| `OPENAI_BASE_URL` | OpenAiEmbeddingProvider | `https://api.openai.com/v1` |
| `EMBEDDING_MODEL` | OpenAiEmbeddingProvider | (必需) |
| `OLLAMA_BASE_URL` | OllamaEmbeddingProvider | `http://localhost:11434` |
| `OLLAMA_EMBED_MODEL` | OllamaEmbeddingProvider | `nomic-embed-text` |
| `CHROMA_URL` | ChromaVectorStore | `http://localhost:8000` |
| `CHROMA_COLLECTION` | ChromaVectorStore | `default` |
| `CHROMA_TENANT` | ChromaVectorStore | `default_tenant` |
| `CHROMA_DATABASE` | ChromaVectorStore | `default_database` |
| `CHROMA_PERSIST_DIR` | ChromaPersistentStore | `./chroma_db` |
| `QDRANT_URL` | QdrantVectorStore | `http://localhost:6333` |
| `QDRANT_API_KEY` | QdrantVectorStore | (无) |
| `QDRANT_COLLECTION` | QdrantVectorStore | `default` |

## 相关文件

- `agentkit/src/rag/mod.rs` - 核心 RAG 模块
- `agentkit/examples/07_rag.rs` - 完整 RAG 示例
- `agentkit/tests/rag_pipeline.rs` - RAG 管道集成测试
- `docs/embedding_retrieval.md` - Embedding 和检索基础
