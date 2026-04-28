# rucora 用户指南

欢迎使用 rucora！本指南将帮助您从零开始构建强大的 LLM 应用。

## 📖 目录

1. [简介](#简介)
2. [核心概念](#核心概念)
3. [快速开始](#快速开始)
4. [核心功能](#核心功能)
5. [进阶使用](#进阶使用)
6. [最佳实践](#最佳实践)

---

## 简介

### 什么是 rucora？

rucora 是一个 Rust 编写的 LLM（大语言模型）应用开发框架。它提供了一套完整的工具，让您能够：

- 🤖 **构建智能 Agent** - 让 AI 自动执行任务
- 🔧 **集成各种工具** - 文件操作、网络请求、数据库等
- 💾 **管理对话历史** - 自动维护多轮对话
- 📊 **追踪成本和用量** - 精确控制 API 支出
- 🔌 **灵活扩展** - 轻松添加自定义功能

### 为什么选择 rucora？

| 特性 | rucora | 其他框架 |
|------|----------|----------|
| 性能 | ⚡ Rust 原生，极速 | 🐌 Python 解释执行 |
| 类型安全 | ✅ 编译时检查 | ⚠️ 运行时错误 |
| 内存占用 | 💚 极低 | 🔴 较高 |
| 部署难度 | 🟢 单二进制 | 🟠 需要 Python 环境 |
| 成本监控 | ✅ 内置支持 | ⚠️ 需要额外实现 |

---

## 核心概念

### 1. Provider（模型提供者）

Provider 是连接 LLM 服务的桥梁。rucora 支持多种 Provider：

```rust
use rucora::provider::OpenAiProvider;

// 从环境变量加载配置
let provider = OpenAiProvider::from_env()?;
```

**支持的 Provider**：
- OpenAI (GPT-4, GPT-3.5)
- Ollama (本地模型)
- 自定义 Provider

### 2. Tool（工具）

工具是 Agent 可以调用的功能单元：

```rust
use rucora::tools::{FileReadTool, HttpRequestTool};

// 创建工具
let file_tool = FileReadTool::new();
let http_tool = HttpRequestTool::new();
```

**内置工具**：
- 📁 文件操作（读/写/编辑）
- 🌐 网络请求（HTTP/网页）
- 💾 记忆存储
- 🔧 系统命令

### 3. Runtime（运行时）

Runtime 负责协调 Provider 和 Tool 的工作：

```rust
use rucora_runtime::DefaultRuntime;

let runtime = DefaultRuntime::new(provider, tools)
    .with_system_prompt("你是有用的助手");
```

### 4. Conversation（对话管理）

自动管理对话历史：

```rust
use rucora::conversation::ConversationManager;

let mut conv = ConversationManager::new()
    .with_max_messages(20);

conv.add_user_message("你好");
conv.add_assistant_message("你好！有什么可以帮助你的？");
```

---

## 快速开始

### 1. 安装

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
rucora = "0.1"
rucora-runtime = "0.1"
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

### 2. 配置环境变量

```bash
# OpenAI API Key（必需）
export OPENAI_API_KEY=sk-your-api-key

# 或者使用 Ollama（可选）
export OLLAMA_BASE_URL=http://localhost:11434
```

### 3. 第一个 Agent 应用

```rust
use rucora::provider::OpenAiProvider;
use rucora_runtime::{DefaultRuntime, ToolRegistry};
use rucora_core::agent::types::AgentInput;
use rucora_core::provider::types::{ChatMessage, Role};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 创建 Provider
    let provider = OpenAiProvider::from_env()?;
    
    // 2. 创建工具注册表
    let tools = ToolRegistry::new();
    
    // 3. 创建运行时
    let runtime = DefaultRuntime::new(Arc::new(provider), tools)
        .with_system_prompt("你是有用的助手");
    
    // 4. 创建输入
    let input = AgentInput {
        messages: vec![ChatMessage {
            role: Role::User,
            content: "用一句话介绍 Rust".to_string(),
            name: None,
        }],
        metadata: None,
    };
    
    // 5. 运行 Agent
    let output = runtime.run(input).await?;
    
    println!("助手回复：{}", output.message.content);
    
    Ok(())
}
```

### 4. 运行

```bash
cargo run
```

输出：
```
助手回复：Rust 是一门系统编程语言，专注于安全性和性能。
```

---

## 核心功能

### 1. 对话管理

自动维护对话历史，支持窗口限制和压缩：

```rust
use rucora::conversation::ConversationManager;

let mut manager = ConversationManager::new()
    .with_system_prompt("你是 Rust 专家")
    .with_max_messages(20)  // 保留最近 20 条消息
    .with_auto_compress(true);  // 自动压缩历史

// 添加消息
manager.add_user_message("如何学习 Rust？");
manager.add_assistant_message("首先学习基础语法...");

// 获取历史（用于 API 调用）
let messages = manager.get_messages();

// 导出/导入（用于持久化）
let json = manager.to_json()?;
let manager = ConversationManager::from_json(&json)?;
```

### 2. Prompt 模板

使用模板系统安全地构建 Prompt：

```rust
use rucora::prompt::{PromptTemplate, PromptBuilder};
use serde_json::json;

// 方法 1：模板字符串
let template = PromptTemplate::from_string(
    "你是{{role}}，请帮助{{user_name}}解决{{problem}}。"
);

let prompt = template.render(&json!({
    "role": "Python 专家",
    "user_name": "张三",
    "problem": "代码调试"
}))?;

// 方法 2：构建器
let prompt = PromptBuilder::new()
    .system("你是有用的助手")
    .user("你好")
    .assistant("你好！有什么可以帮助你的？")
    .user("请介绍 Rust")
    .build();
```

### 3. Token 计数和成本管理

精确追踪 API 使用量和成本：

```rust
use rucora::cost::{TokenCounter, CostTracker};

// Token 计数
let counter = TokenCounter::new("gpt-4");
let tokens = counter.count_text("Hello, World!");
let message_tokens = counter.count_messages(&messages);

// 成本追踪
let tracker = CostTracker::new()
    .with_budget_limit(10.0);  // 预算$10

// 记录使用
tracker.record_usage("gpt-4", 100, 50, 0.0045).await;

// 检查预算
if tracker.check_budget(10.0).await {
    println!("预算充足");
} else {
    println!("超出预算！");
}

// 获取统计
let stats = tracker.get_statistics().await;
println!("总成本：${}", stats.total_cost);
```

### 4. 中间件系统

在请求/响应处理过程中插入自定义逻辑：

```rust
use rucora::middleware::{
    MiddlewareChain,
    LoggingMiddleware,
    RateLimitMiddleware,
    MetricsMiddleware,
};

// 创建中间件链
let chain = MiddlewareChain::new()
    .with(LoggingMiddleware::new())  // 日志记录
    .with(RateLimitMiddleware::new(100))  // 限流：100 次/分钟
    .with(MetricsMiddleware::new());  // 指标收集

// 使用中间件
chain.process_request(&mut input).await?;
// ... 处理请求 ...
chain.process_response(&mut output).await?;

// 获取指标
let metrics = MetricsMiddleware::new();
println!("请求数：{}", metrics.get_request_count());
```

### 5. 错误处理

细粒度的错误分类和处理：

```rust
use rucora_core::error::{
    ProviderError, DiagnosticError, ErrorCategory
};

match provider.chat(request).await {
    Ok(response) => {
        // 成功
    }
    Err(err) => {
        // 获取诊断信息
        let diag = err.diagnostic();
        
        // 判断是否可重试
        if err.is_retriable() {
            // 执行重试
        }
        
        // 根据错误类型处理
        match err.category() {
            ErrorCategory::RateLimit => {
                // 等待后重试
            }
            ErrorCategory::Authentication => {
                // 检查 API Key
            }
            _ => {}
        }
    }
}
```

---

## 进阶使用

### 1. 自定义工具

实现自己的工具：

```rust
use rucora_core::tool::{Tool, ToolCategory};
use rucora_core::error::ToolError;
use async_trait::async_trait;
use serde_json::{Value, json};

struct WeatherTool;

#[async_trait]
impl Tool for WeatherTool {
    fn name(&self) -> &str {
        "weather"
    }

    fn description(&self) -> Option<&str> {
        Some("查询指定城市的天气")
    }

    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::External]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "city": {
                    "type": "string",
                    "description": "城市名称"
                }
            },
            "required": ["city"]
        })
    }

    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let city = input
            .get("city")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少 city 参数".to_string()))?;
        
        // 实现天气查询逻辑
        Ok(json!({
            "city": city,
            "temperature": 25,
            "condition": "晴朗"
        }))
    }
}

// 使用自定义工具
let tools = ToolRegistry::new()
    .register(WeatherTool);
```

### 2. 自定义 Provider

接入其他 LLM 服务：

```rust
use rucora_core::provider::{LlmProvider, types::*};
use rucora_core::error::ProviderError;
use async_trait::async_trait;
use futures_util::stream::BoxStream;

struct CustomProvider {
    api_key: String,
    base_url: String,
}

#[async_trait]
impl LlmProvider for CustomProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        // 实现聊天逻辑
        unimplemented!()
    }

    fn stream_chat(
        &self,
        request: ChatRequest,
    ) -> Result<BoxStream<'static, Result<ChatStreamChunk, ProviderError>>, ProviderError> {
        // 实现流式聊天
        unimplemented!()
    }
}
```

### 3. 向量检索（RAG）

实现检索增强生成：

```rust
use rucora::retrieval::InMemoryVectorStore;
use rucora_core::retrieval::{VectorRecord, VectorQuery};

// 创建向量存储
let store = InMemoryVectorStore::new();

// 插入文档
store.upsert(vec![
    VectorRecord::new("doc1", vec![0.1, 0.2, 0.3])
        .with_text("Rust 是系统编程语言"),
    VectorRecord::new("doc2", vec![0.4, 0.5, 0.6])
        .with_text("Python 是脚本语言"),
]).await?;

// 搜索相似文档
let results = store.search(
    VectorQuery::new(vec![0.1, 0.2, 0.3])
        .with_top_k(5)
).await?;

for result in results {
    println!("文档：{} 相似度：{}", result.id, result.score);
}
```

---

## 最佳实践

### 1. 对话管理

```rust
// ✅ 好的做法
let manager = ConversationManager::new()
    .with_max_messages(20)  // 限制消息数量
    .with_system_prompt("你是有用的助手");

// ❌ 避免
// 手动管理消息数组，容易出错
```

### 2. 成本控制

```rust
// ✅ 好的做法
let tracker = CostTracker::new()
    .with_budget_limit(10.0);

// 每次调用后记录
tracker.record_usage(model, prompt_tokens, completion_tokens, cost).await;

// 定期检查预算
if !tracker.check_budget(budget).await {
    // 停止服务或降级
}

// ❌ 避免
// 不追踪使用量，可能导致巨额账单
```

### 3. 错误处理

```rust
// ✅ 好的做法
match result {
    Ok(output) => output,
    Err(err) if err.is_retriable() => {
        // 可重试错误，执行重试
        retry_operation().await?
    }
    Err(err) => {
        // 不可重试错误，记录并返回
        log_error(&err);
        return Err(err);
    }
}

// ❌ 避免
// 不区分错误类型，盲目重试
```

### 4. 中间件使用

```rust
// ✅ 好的做法
let chain = MiddlewareChain::new()
    .with(LoggingMiddleware::new())  // 先日志
    .with(RateLimitMiddleware::new(100))  // 再限流
    .with(CacheMiddleware::new());  // 最后缓存

// ❌ 避免
// 顺序错误：缓存应该在限流之前
```

---

## 下一步

- 📚 查看 [API 参考文档](./api_reference.md) 了解完整 API
- 🍳 查看 [示例集合](./cookbook.md) 学习更多用例
- ❓ 查看 [常见问题](./faq.md) 解决问题

---

**祝您使用愉快！如有问题欢迎反馈。**
