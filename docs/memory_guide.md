# rucora 记忆系统与 Agent 结合使用指南

## 概述

rucora 的记忆系统允许 Agent 具备长期记忆能力，可以存储和检索用户信息、偏好、历史对话等内容。本指南展示如何将记忆系统与 Agent 集成使用。

## 核心组件

### 1. Memory Trait

```rust
#[async_trait]
pub trait Memory: Send + Sync {
    /// 写入一条记忆
    async fn add(&self, item: MemoryItem) -> Result<(), MemoryError>;
    
    /// 检索记忆
    async fn query(&self, query: MemoryQuery) -> Result<Vec<MemoryItem>, MemoryError>;
}
```

### 2. 记忆实现

| 类型 | 说明 | 使用场景 |
|------|------|----------|
| `InMemoryMemory` | 进程内记忆存储 | 测试、临时会话 |
| `FileMemory` | 文件持久化记忆 | 长期存储、跨会话 |

### 3. 记忆工具

| 工具 | 功能 | 说明 |
|------|------|------|
| `MemoryStoreTool` | 存储记忆 | 让 Agent 主动存储重要信息 |
| `MemoryRecallTool` | 检索记忆 | 让 Agent 检索历史记忆 |

## 使用方式

### 方式 1：直接使用 Memory API

适合简单场景，手动管理记忆的存储和检索：

```rust
use rucora::memory::InMemoryMemory;
use rucora_core::memory::{Memory, MemoryItem, MemoryQuery};

let memory = InMemoryMemory::new();

// 添加记忆
memory.add(MemoryItem {
    id: "core:user_name".to_string(),
    content: "张三".to_string(),
    metadata: None,
}).await?;

// 查询记忆
let results = memory.query(MemoryQuery {
    text: "用户姓名".to_string(),
    limit: 10,
}).await?;
```

### 方式 2：Agent + 记忆工具（推荐）

让 Agent 自主决定何时存储和检索记忆：

```rust
use rucora::agent::ToolAgent;
use rucora::memory::InMemoryMemory;
use rucora::tools::{MemoryStoreTool, MemoryRecallTool};
use rucora_core::agent::Agent;
use std::sync::Arc;

// 创建共享记忆系统
let shared_memory = Arc::new(InMemoryMemory::new());

// 创建记忆工具
let memory_store = MemoryStoreTool::from_memory(shared_memory.clone());
let memory_recall = MemoryRecallTool::from_memory(shared_memory.clone());

// 创建带记忆功能的 Agent
let agent = ToolAgent::builder()
    .provider(provider)
    .model("gpt-4o-mini")
    .system_prompt(
        "你是一个有帮助的助手，拥有长期记忆能力。\n\
         你可以使用 memory_store 工具存储重要信息。\n\
         你可以使用 memory_recall 工具检索之前存储的信息。"
    )
    .tool(memory_store)
    .tool(memory_recall)
    .max_steps(5)
    .build();

// Agent 会自动决定何时使用记忆工具
let output = agent.run("我叫李四，是一名软件工程师。").await?;
```

### 方式 3：文件持久化记忆

适合需要跨会话保存记忆的场景：

```rust
use rucora::memory::FileMemory;
use rucora_core::memory::{Memory, MemoryItem};

let memory = FileMemory::new("memory.json");

// 添加记忆（自动保存到文件）
memory.add(MemoryItem {
    id: "core:preference".to_string(),
    content: "喜欢 Rust 编程".to_string(),
    metadata: None,
}).await?;

// 下次启动时会自动加载之前的记忆
```

## 记忆分类

通过 ID 前缀对记忆进行分类管理：

| 前缀 | 类型 | 说明 | 示例 |
|------|------|------|------|
| `core:` | 永久记忆 | 用户偏好、基本信息 | `core:user_name` |
| `daily:` | 会话记忆 | 当天对话主题 | `daily:last_topic` |
| `conversation:` | 对话上下文 | 最近的对话内容 | `conversation:last_msg` |

```rust
// 永久记忆
MemoryItem {
    id: "core:user_name".to_string(),
    content: "Alice".to_string(),
    metadata: None,
}

// 会话记忆
MemoryItem {
    id: "daily:2024-01-01_topic".to_string(),
    content: "讨论了机器学习项目".to_string(),
    metadata: None,
}
```

## 完整示例

运行示例代码：

```bash
export OPENAI_API_KEY=sk-your-key
cargo run --example 06_memory
```

示例演示了：

1. **基础记忆操作** - 直接使用 Memory API
2. **Agent + 记忆工具** - Agent 自主存储和检索记忆
3. **文件记忆持久化** - 跨会话保存记忆
4. **对话历史 + 记忆系统** - 多轮对话中的记忆使用

## 最佳实践

### 1. 使用有意义的 ID

```rust
// ✅ 好的 ID
"core:user_name"
"core:programming_preference"
"daily:2024-01-01_topic"

// ❌ 避免的 ID
"memory_1"
"temp_data"
```

### 2. 合理设置记忆容量

```rust
// 限制记忆数量，避免无限增长
let memory = InMemoryMemory::with_capacity(100); // 最多 100 条
```

### 3. 定期清理过期记忆

```rust
// 清空所有记忆
memory.clear().await;

// 或者按类别删除特定记忆
```

### 4. 结合系统提示词

在 Agent 的系统提示词中明确记忆使用说明：

```rust
.system_prompt(
    "你是一个有帮助的助手，拥有长期记忆能力。\n\
     当用户提到以下信息时，请存储到记忆中：\n\
     - 个人信息（姓名、职业、地点等）\n\
     - 偏好（语言、工具、兴趣等）\n\
     - 重要事实或项目信息\n\
     当需要回忆之前的信息时，使用记忆检索工具。"
)
```

### 5. 共享记忆系统

多个 Agent 实例可以共享同一个记忆系统：

```rust
let shared_memory = Arc::new(InMemoryMemory::new());

let agent1 = ToolAgent::builder()
    .tool(MemoryStoreTool::from_memory(shared_memory.clone()))
    .build();

let agent2 = ToolAgent::builder()
    .tool(MemoryRecallTool::from_memory(shared_memory.clone()))
    .build();

// agent1 存储的记忆，agent2 可以检索
```

## 高级用法：RAG 增强记忆

结合向量检索实现语义搜索：

```rust
use rucora::retrieval::InMemoryVectorStore;
use rucora_core::retrieval::{VectorStore, VectorQuery};

// 1. 将记忆向量化存储
let vector_store = InMemoryVectorStore::new();

// 2. 使用语义搜索而非关键词匹配
let results = vector_store.search(VectorQuery {
    vector: embedding_vector,  // 查询的向量表示
    top_k: 5,
    score_threshold: Some(0.7),
}).await?;
```

## 常见问题

### Q: 记忆工具会被 Agent 滥用吗？

A: 可以通过以下方式控制：
- 设置 `max_steps` 限制工具调用次数
- 在系统提示词中说明使用场景
- 使用 `DefaultToolPolicy` 设置工具调用策略

### Q: 记忆会一直保存吗？

A: 
- `InMemoryMemory`: 程序退出后消失
- `FileMemory`: 持久化保存，除非手动删除文件

### Q: 如何保护用户隐私？

A: 
- 使用加密的文件存储
- 定期清理敏感记忆
- 提供用户删除记忆的接口

## 总结

rucora 的记忆系统提供了灵活的记忆能力：

| 特性 | 说明 |
|------|------|
| **简单易用** | 最小化 API 设计 |
| **灵活集成** | 可作为工具或直接使用 |
| **持久化支持** | 文件记忆长期保存 |
| **分类管理** | 通过 ID 前缀分类 |
| **扩展性强** | 可实现自定义 Memory |

通过记忆系统，Agent 可以：
- ✅ 记住用户偏好和基本信息
- ✅ 在多次对话中保持一致性
- ✅ 提供个性化的服务
- ✅ 建立长期的用户关系
