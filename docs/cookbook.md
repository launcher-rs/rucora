# AgentKit 示例集合 (Cookbook)

本集合提供实用的代码示例，帮助您快速解决常见问题。

## 📋 目录

1. [基础示例](#基础示例)
2. [对话管理](#对话管理)
3. [工具使用](#工具使用)
4. [成本管理](#成本管理)
5. [高级用法](#高级用法)

---

## 基础示例

### 1. 最简单的 Agent

```rust
use agentkit::provider::OpenAiProvider;
use agentkit_runtime::{DefaultRuntime, ToolRegistry};
use agentkit_core::agent::types::AgentInput;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = OpenAiProvider::from_env()?;
    let runtime = DefaultRuntime::new(Arc::new(provider), ToolRegistry::new())
        .with_system_prompt("你是有用的助手");

    let input = AgentInput::from("你好，请介绍下自己");
    let output = runtime.run(input).await?;

    println!("{}", output.message.content);
    Ok(())
}
```

### 2. 流式输出

```rust
use agentkit_runtime::{DefaultRuntime, ChannelEvent};
use futures_util::StreamExt;

let mut stream = runtime.run_stream(input);
while let Some(event) = stream.next().await {
    match event? {
        ChannelEvent::TokenDelta(delta) => {
            print!("{}", delta.delta);
        }
        _ => {}
    }
}
```

### 3. 多轮对话

```rust
use agentkit::conversation::ConversationManager;

let mut conv = ConversationManager::new()
    .with_system_prompt("你是 Rust 专家");

// 第一轮
conv.add_user_message("什么是所有权？");
conv.add_assistant_message("所有权是 Rust 的核心概念...");

// 第二轮
conv.add_user_message("能举个例子吗？");

// 获取完整历史用于 API 调用
let messages = conv.get_messages();
```

---

## 对话管理

### 4. 限制对话长度

```rust
let manager = ConversationManager::new()
    .with_max_messages(20)  // 保留最近 20 条
    .with_system_prompt("助手");
```

### 5. 对话持久化

```rust
// 保存
let json = manager.to_json()?;
std::fs::write("conversation.json", json)?;

// 加载
let json = std::fs::read_to_string("conversation.json")?;
let manager = ConversationManager::from_json(&json)?;
```

### 6. 对话压缩

```rust
// 当对话过长时，压缩早期历史
manager.compress("用户询问了 Rust 的所有权概念，已解释基本规则。");
```

---

## 工具使用

### 7. 使用内置工具

```rust
use agentkit::tools::{
    FileReadTool, FileWriteTool,
    HttpRequestTool, ShellTool,
};

let tools = ToolRegistry::new()
    .register(FileReadTool::new())
    .register(FileWriteTool::new())
    .register(HttpRequestTool::new())
    .register(ShellTool::new());
```

### 8. 创建自定义工具

```rust
use agentkit_core::tool::{Tool, ToolCategory};
use agentkit_core::error::ToolError;
use async_trait::async_trait;
use serde_json::{Value, json};

struct CalculatorTool;

#[async_trait]
impl Tool for CalculatorTool {
    fn name(&self) -> &str { "calculator" }
    
    fn description(&self) -> Option<&str> {
        Some("执行数学计算")
    }

    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::Basic]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {"type": "string", "enum": ["add", "sub", "mul", "div"]},
                "a": {"type": "number"},
                "b": {"type": "number"}
            },
            "required": ["operation", "a", "b"]
        })
    }

    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let op = input["operation"].as_str().unwrap();
        let a = input["a"].as_f64().unwrap();
        let b = input["b"].as_f64().unwrap();
        
        let result = match op {
            "add" => a + b,
            "sub" => a - b,
            "mul" => a * b,
            "div" => a / b,
            _ => return Err(ToolError::Message("未知操作".into())),
        };
        
        Ok(json!({"result": result}))
    }
}
```

### 9. 工具组合使用

```rust
// 创建工具链
let tools = ToolRegistry::new()
    .register(FileReadTool::new())
    .register(CalculatorTool);

// Agent 会自动选择合适的工具
let input = AgentInput::from("读取 data.txt 并计算其中数字的总和");
```

---

## 成本管理

### 10. Token 计数

```rust
use agentkit::cost::TokenCounter;

let counter = TokenCounter::new("gpt-4");

// 计算文本
let tokens = counter.count_text("Hello, World!");

// 计算消息
let tokens = counter.count_messages(&messages);

// 计算工具定义
let tokens = counter.count_tools(&tools);
```

### 11. 成本追踪

```rust
use agentkit::cost::CostTracker;

let tracker = CostTracker::new();

// 记录使用
tracker.record_usage("gpt-4", 100, 50, 0.0045).await;

// 获取总成本
let cost = tracker.get_current_cost().await;
println!("总成本：${}", cost);

// 获取使用量
let usage = tracker.get_total_usage().await;
println!("Token: {}", usage.total_tokens);
```

### 12. 预算控制

```rust
let tracker = CostTracker::new()
    .with_budget_limit(10.0);  // $10 预算

// 每次调用前检查
if !tracker.check_budget(10.0).await {
    return Err("超出预算".into());
}

// 记录使用
tracker.record_usage(model, prompt_tokens, completion_tokens, cost).await;
```

### 13. 使用统计

```rust
let stats = tracker.get_statistics().await;

println!("总成本：${}", stats.total_cost);
println!("总请求：{}", stats.total_requests);
println!("输入 Token: {}", stats.total_prompt_tokens);
println!("输出 Token: {}", stats.total_completion_tokens);

// 按模型统计
for (model, model_stats) in &stats.models {
    println!("{}: {} 次请求", model, model_stats.requests);
}
```

---

## 高级用法

### 14. 中间件：日志记录

```rust
use agentkit::middleware::{MiddlewareChain, LoggingMiddleware};

let chain = MiddlewareChain::new()
    .with(LoggingMiddleware::new());

chain.process_request(&mut input).await?;
// ... 处理 ...
chain.process_response(&mut output).await?;
```

### 15. 中间件：限流

```rust
use agentkit::middleware::RateLimitMiddleware;

let chain = MiddlewareChain::new()
    .with(RateLimitMiddleware::new(100)  // 100 次/分钟
        .with_window_secs(60));
```

### 16. 中间件：指标收集

```rust
use agentkit::middleware::MetricsMiddleware;

let metrics = MetricsMiddleware::new();
let chain = MiddlewareChain::new()
    .with(metrics.clone());

// 获取指标
println!("请求数：{}", metrics.get_request_count());
println!("响应数：{}", metrics.get_response_count());
```

### 17. Prompt 模板

```rust
use agentkit::prompt::PromptTemplate;
use serde_json::json;

// 简单变量替换
let template = PromptTemplate::from_string(
    "你是{{role}}，请帮助{{user}}。"
);

let prompt = template.render(&json!({
    "role": "Python 专家",
    "user": "张三"
}))?;

// 条件渲染
let template = PromptTemplate::from_string(
    "你好{{#if name}}，{{name}}{{/if}}！"
);

// 循环渲染
let template = PromptTemplate::from_string(
    "项目：{{#each items}}- {{this}}\n{{/each}}"
);
```

### 18. 错误处理最佳实践

```rust
use agentkit_core::error::{DiagnosticError, ErrorCategory};

match provider.chat(request).await {
    Ok(response) => response,
    Err(err) => {
        // 获取诊断信息
        let diag = err.diagnostic();
        
        match err.category() {
            ErrorCategory::RateLimit => {
                // 等待后重试
                tokio::time::sleep(diag.retry_after.unwrap()).await;
                retry().await?;
            }
            ErrorCategory::Authentication => {
                // 检查 API Key
                return Err("API Key 无效".into());
            }
            ErrorCategory::Timeout => {
                // 重试
                retry().await?;
            }
            _ => return Err(err.into()),
        }
    }
}
```

### 19. 向量检索（RAG）

```rust
use agentkit::retrieval::InMemoryVectorStore;
use agentkit_core::retrieval::{VectorRecord, VectorQuery};

// 创建存储
let store = InMemoryVectorStore::new();

// 插入文档
store.upsert(vec![
    VectorRecord::new("doc1", embedding)
        .with_text("Rust 是系统编程语言")
        .with_metadata(json!({"source": "wiki"})),
]).await?;

// 搜索
let results = store.search(
    VectorQuery::new(query_embedding)
        .with_top_k(5)
        .with_score_threshold(0.7)
).await?;

for result in results {
    println!("{}: {} ({})", result.id, result.text.unwrap(), result.score);
}
```

### 20. 完整应用示例

```rust
use agentkit::prelude::*;
use agentkit::conversation::ConversationManager;
use agentkit::cost::{TokenCounter, CostTracker};
use agentkit::middleware::{MiddlewareChain, LoggingMiddleware};
use agentkit_runtime::DefaultRuntime;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 初始化组件
    let provider = OpenAiProvider::from_env()?;
    let tools = ToolRegistry::new();
    let mut conv = ConversationManager::new()
        .with_max_messages(20);
    
    let counter = TokenCounter::new("gpt-4");
    let tracker = CostTracker::new()
        .with_budget_limit(10.0);
    
    let middleware = MiddlewareChain::new()
        .with(LoggingMiddleware::new());
    
    // 2. 创建运行时
    let runtime = DefaultRuntime::new(Arc::new(provider), tools)
        .with_system_prompt("你是有用的助手");
    
    // 3. 对话循环
    loop {
        // 读取用户输入
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        if input.trim() == "exit" {
            break;
        }
        
        // 添加到对话
        conv.add_user_message(input);
        
        // 中间件处理
        let mut agent_input = AgentInput {
            messages: conv.get_messages().to_vec(),
            metadata: None,
        };
        middleware.process_request(&mut agent_input).await?;
        
        // 运行 Agent
        let output = runtime.run(agent_input).await?;
        
        // 记录成本
        let tokens = counter.count_messages(conv.get_messages());
        tracker.record_usage("gpt-4", tokens, 0, 0.0).await;
        
        // 显示回复
        println!("助手：{}", output.message.content);
        conv.add_assistant_message(output.message.content);
    }
    
    // 4. 显示统计
    let stats = tracker.get_statistics().await;
    println!("\n总成本：${}", stats.total_cost);
    println!("总请求：{}", stats.total_requests);
    
    Ok(())
}
```

---

## 📚 相关资源

- [用户指南](./user_guide.md) - 详细使用文档
- [快速入门](./quick_start.md) - 10 分钟上手
- [API 参考](./api_reference.md) - 完整 API 文档
- [常见问题](./faq.md) - 问题解答
