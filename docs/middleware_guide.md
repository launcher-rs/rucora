# AgentKit 中间件使用指南

## 概述

中间件系统允许你在 Agent 执行流程中插入自定义逻辑，实现横切关注点的模块化。中间件可以在以下时机执行：

1. **请求前** - 用户输入进入 Agent 之前
2. **响应后** - Agent 输出返回给用户之前
3. **错误处理** - Agent 执行出错时
4. **工具调用前** - 工具执行之前
5. **工具调用后** - 工具执行之后

## 核心概念

### Middleware Trait

所有中间件必须实现 `Middleware` trait：

```rust
use agentkit::middleware::Middleware;
use agentkit_core::agent::{AgentError, AgentInput, AgentOutput};
use agentkit_core::tool::types::{ToolCall, ToolResult};
use async_trait::async_trait;

#[async_trait]
pub trait Middleware: Send + Sync {
    /// 中间件名称
    fn name(&self) -> &str;

    /// 请求前钩子
    async fn on_request(&self, input: &mut AgentInput) -> Result<(), AgentError>;

    /// 响应后钩子
    async fn on_response(&self, output: &mut AgentOutput) -> Result<(), AgentError>;

    /// 错误处理钩子
    async fn on_error(&self, error: &mut AgentError) -> Result<(), AgentError>;

    /// 工具调用前钩子
    async fn on_tool_call_before(&self, call: &mut ToolCall) -> Result<(), AgentError>;

    /// 工具调用后钩子
    async fn on_tool_call_after(&self, result: &mut ToolResult) -> Result<(), AgentError>;
}
```

### MiddlewareChain

中间件链按顺序管理多个中间件：

```rust
use agentkit::middleware::MiddlewareChain;

// 创建中间件链
let chain = MiddlewareChain::new()
    .with(LoggingMiddleware::new())
    .with(CacheMiddleware::new())
    .with(RateLimitMiddleware::new(100));

// 或者逐个添加
let mut chain = MiddlewareChain::new();
chain = chain.with(LoggingMiddleware::new());
```

## 内置中间件

### LoggingMiddleware

记录请求和响应信息。

```rust
use agentkit::middleware::LoggingMiddleware;

let agent = ToolAgent::builder()
    .provider(provider)
    .with_middleware(LoggingMiddleware::new())
    .build();
```

### RateLimitMiddleware

限制请求频率。

```rust
use agentkit::middleware::RateLimitMiddleware;

// 每分钟最多 60 个请求
let agent = ToolAgent::builder()
    .provider(provider)
    .with_middleware(RateLimitMiddleware::new(60))
    .build();
```

### CacheMiddleware

缓存响应结果。

```rust
use agentkit::middleware::CacheMiddleware;

let agent = ToolAgent::builder()
    .provider(provider)
    .with_middleware(CacheMiddleware::new())
    .build();
```

### MetricsMiddleware

收集性能指标。

```rust
use agentkit::middleware::MetricsMiddleware;

let metrics = MetricsMiddleware::new();
let agent = ToolAgent::builder()
    .provider(provider)
    .with_middleware(metrics.clone())
    .build();

// 获取指标
println!("请求计数：{}", metrics.get_request_count());
println!("响应计数：{}", metrics.get_response_count());
```

## 自定义中间件

### 示例 1：认证中间件

```rust
use agentkit::middleware::Middleware;
use agentkit_core::agent::{AgentError, AgentInput};
use async_trait::async_trait;

#[derive(Clone)]
struct AuthMiddleware {
    api_key: String,
}

impl AuthMiddleware {
    fn new(api_key: impl Into<String>) -> Self {
        Self { api_key: api_key.into() }
    }
}

#[async_trait]
impl Middleware for AuthMiddleware {
    fn name(&self) -> &str {
        "auth"
    }

    async fn on_request(&self, input: &mut AgentInput) -> Result<(), AgentError> {
        if input.text.contains("UNAUTHORIZED") {
            return Err(AgentError::Message("认证失败".to_string()));
        }
        Ok(())
    }
}

// 使用
let agent = ToolAgent::builder()
    .provider(provider)
    .with_middleware(AuthMiddleware::new("sk-test-key"))
    .build();
```

### 示例 2：敏感词过滤中间件

```rust
use agentkit::middleware::Middleware;
use agentkit_core::agent::{AgentError, AgentInput};
use async_trait::async_trait;

#[derive(Clone)]
struct SensitiveWordFilterMiddleware {
    sensitive_words: Vec<String>,
}

impl SensitiveWordFilterMiddleware {
    fn new(sensitive_words: Vec<&str>) -> Self {
        Self {
            sensitive_words: sensitive_words.into_iter().map(String::from).collect(),
        }
    }
}

#[async_trait]
impl Middleware for SensitiveWordFilterMiddleware {
    fn name(&self) -> &str {
        "sensitive_word_filter"
    }

    async fn on_request(&self, input: &mut AgentInput) -> Result<(), AgentError> {
        let mut filtered = input.text.clone();
        for word in &self.sensitive_words {
            if filtered.contains(word.as_str()) {
                filtered = filtered.replace(word.as_str(), "***");
            }
        }
        input.text = filtered;
        Ok(())
    }
}

// 使用
let agent = ToolAgent::builder()
    .provider(provider)
    .with_middleware(SensitiveWordFilterMiddleware::new(vec!["敏感词", "违规"]))
    .build();
```

### 示例 3：响应格式化中间件

```rust
use agentkit::middleware::Middleware;
use agentkit_core::agent::{AgentError, AgentOutput};
use async_trait::async_trait;
use serde_json::json;

#[derive(Clone)]
struct ResponseFormatMiddleware;

#[async_trait]
impl Middleware for ResponseFormatMiddleware {
    fn name(&self) -> &str {
        "response_format"
    }

    async fn on_response(&self, output: &mut AgentOutput) -> Result<(), AgentError> {
        if let Some(content) = output.value.as_object_mut() {
            content.insert("formatted".to_string(), json!(true));
        }
        Ok(())
    }
}
```

### 示例 4：工具调用日志中间件

```rust
use agentkit::middleware::Middleware;
use agentkit_core::agent::AgentError;
use agentkit_core::tool::types::{ToolCall, ToolResult};
use async_trait::async_trait;
use tracing::info;

#[derive(Clone)]
struct ToolCallLoggingMiddleware;

#[async_trait]
impl Middleware for ToolCallLoggingMiddleware {
    fn name(&self) -> &str {
        "tool_call_logging"
    }

    async fn on_tool_call_before(&self, call: &mut ToolCall) -> Result<(), AgentError> {
        info!(
            "工具调用前：名称={}, 输入={}",
            call.name, call.input
        );
        Ok(())
    }

    async fn on_tool_call_after(&self, result: &mut ToolResult) -> Result<(), AgentError> {
        info!(
            "工具调用后：ID={}, 输出={}",
            result.tool_call_id, result.output
        );
        Ok(())
    }
}
```

## 与 Agent 集成

### 方式 1：使用 with_middleware_chain()

一次性设置完整的中间件链：

```rust
use agentkit::agent::ToolAgent;
use agentkit::middleware::MiddlewareChain;

let agent = ToolAgent::builder()
    .provider(provider)
    .model("gpt-4o-mini")
    .with_middleware_chain(
        MiddlewareChain::new()
            .with(LoggingMiddleware::new())
            .with(RateLimitMiddleware::new(60))
            .with(CacheMiddleware::new())
    )
    .build();
```

### 方式 2：使用 with_middleware()

逐个添加中间件：

```rust
let agent = ToolAgent::builder()
    .provider(provider)
    .model("gpt-4o-mini")
    .with_middleware(LoggingMiddleware::new())
    .with_middleware(CacheMiddleware::new())
    .with_middleware(AuthMiddleware::new("key"))
    .build();
```

### 所有 Agent 类型都支持中间件

```rust
// SimpleAgent
let simple = SimpleAgent::builder()
    .provider(provider)
    .with_middleware(LoggingMiddleware::new())
    .build();

// ChatAgent
let chat = ChatAgent::builder()
    .provider(provider)
    .with_middleware_chain(MiddlewareChain::new()
        .with(LoggingMiddleware::new())
        .with(AuthMiddleware::new("key"))
    )
    .build();

// ToolAgent
let tool = ToolAgent::builder()
    .provider(provider)
    .with_middleware(LoggingMiddleware::new())
    .with_middleware(ToolCallLoggingMiddleware)
    .build();

// ReActAgent
let react = ReActAgent::builder()
    .provider(provider)
    .with_middleware(RateLimitMiddleware::new(30))
    .build();

// ReflectAgent
let reflect = ReflectAgent::builder()
    .provider(provider)
    .with_middleware(ResponseFormatMiddleware)
    .build();
```

## 自定义 Agent 使用中间件

如果你创建了自己的自定义 Agent 类型，可以通过以下方式使用中间件：

### 方式 1：组合 DefaultExecution（推荐）

最简单的方式是组合 `DefaultExecution`，它已经内置了中间件支持：

```rust
use agentkit::agent::execution::DefaultExecution;
use agentkit::middleware::MiddlewareChain;
use agentkit_core::agent::{Agent, AgentContext, AgentDecision, AgentInput, AgentOutput};
use agentkit_core::provider::LlmProvider;
use async_trait::async_trait;
use std::sync::Arc;

/// 自定义 Agent
pub struct MyAgent<P> {
    provider: Arc<P>,
    system_prompt: Option<String>,
    execution: DefaultExecution,  // 组合 DefaultExecution
}

#[async_trait]
impl<P> Agent for MyAgent<P>
where
    P: LlmProvider + Send + Sync + 'static,
{
    async fn think(&self, context: &AgentContext) -> AgentDecision {
        // 你的决策逻辑
        AgentDecision::Chat {
            request: context.default_chat_request(),
        }
    }

    fn name(&self) -> &str {
        "my_agent"
    }

    /// 运行 Agent（使用 DefaultExecution 的执行能力）
    async fn run(
        &self,
        input: AgentInput,
    ) -> Result<AgentOutput, agentkit_core::agent::AgentError> {
        self.execution.run(self, input).await
    }
}

/// 构建器
pub struct MyAgentBuilder<P> {
    provider: Option<P>,
    system_prompt: Option<String>,
    middleware_chain: MiddlewareChain,  // 支持中间件
}

impl<P> MyAgentBuilder<P> {
    pub fn new() -> Self {
        Self {
            provider: None,
            system_prompt: None,
            middleware_chain: MiddlewareChain::new(),
        }
    }
}

impl<P> MyAgentBuilder<P>
where
    P: LlmProvider + Send + Sync + 'static,
{
    pub fn provider(mut self, provider: P) -> Self {
        self.provider = Some(provider);
        self
    }

    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// 添加中间件链
    pub fn with_middleware_chain(mut self, middleware_chain: MiddlewareChain) -> Self {
        self.middleware_chain = middleware_chain;
        self
    }

    /// 添加单个中间件
    pub fn with_middleware<M: agentkit::middleware::Middleware + 'static>(
        mut self,
        middleware: M,
    ) -> Self {
        self.middleware_chain = self.middleware_chain.with(middleware);
        self
    }

    pub fn build(self) -> MyAgent<P> {
        let provider = Arc::new(self.provider.expect("Provider is required"));
        
        // 创建执行能力，包含中间件链
        let execution = DefaultExecution::new(
            provider.clone(),
            "my-model",
            agentkit::agent::ToolRegistry::new(),
        )
        .with_system_prompt_opt(self.system_prompt)
        .with_middleware_chain(self.middleware_chain);  // 设置中间件链

        MyAgent {
            provider,
            system_prompt: None,
            execution,
        }
    }
}

// 使用示例
let agent = MyAgentBuilder::new()
    .provider(provider)
    .system_prompt("你是有用的助手")
    .with_middleware(LoggingMiddleware::new())
    .with_middleware(RateLimitMiddleware::new(60))
    .build();
```

### 方式 2：手动调用中间件

如果你想完全控制执行流程，可以手动调用中间件钩子：

```rust
use agentkit::middleware::MiddlewareChain;
use agentkit_core::agent::{Agent, AgentContext, AgentDecision, AgentInput, AgentOutput, AgentError};
use agentkit_core::provider::LlmProvider;
use async_trait::async_trait;
use std::sync::Arc;

pub struct MyCustomAgent<P> {
    provider: Arc<P>,
    middleware_chain: MiddlewareChain,
}

#[async_trait]
impl<P> Agent for MyCustomAgent<P>
where
    P: LlmProvider + Send + Sync + 'static,
{
    async fn think(&self, context: &AgentContext) -> AgentDecision {
        AgentDecision::Chat {
            request: context.default_chat_request(),
        }
    }

    fn name(&self) -> &str {
        "my_custom_agent"
    }

    async fn run(
        &self,
        mut input: AgentInput,
    ) -> Result<AgentOutput, AgentError> {
        // 1. 执行请求前中间件钩子
        self.middleware_chain.process_request(&mut input).await.map_err(|e| {
            AgentError::Message(format!("中间件处理失败：{}", e))
        })?;

        // 2. 执行你的 Agent 逻辑
        let mut output = self._execute_logic(input).await?;

        // 3. 执行响应后中间件钩子
        self.middleware_chain.process_response(&mut output).await.map_err(|e| {
            AgentError::Message(format!("中间件响应处理失败：{}", e))
        })?;

        Ok(output)
    }
}

impl<P> MyCustomAgent<P> {
    async fn _execute_logic(
        &self,
        input: AgentInput,
    ) -> Result<AgentOutput, AgentError> {
        // 你的执行逻辑
        let response = self.provider.chat(input.into()).await?;
        Ok(AgentOutput::new(serde_json::json!({
            "content": response.message.content
        })))
    }
}

// 构建器
pub struct MyCustomAgentBuilder<P> {
    provider: Option<P>,
    middleware_chain: MiddlewareChain,
}

impl<P> MyCustomAgentBuilder<P> {
    pub fn new() -> Self {
        Self {
            provider: None,
            middleware_chain: MiddlewareChain::new(),
        }
    }

    pub fn provider(mut self, provider: P) -> Self {
        self.provider = Some(provider);
        self
    }

    pub fn with_middleware_chain(mut self, middleware_chain: MiddlewareChain) -> Self {
        self.middleware_chain = middleware_chain;
        self
    }

    pub fn with_middleware<M: agentkit::middleware::Middleware + 'static>(
        mut self,
        middleware: M,
    ) -> Self {
        self.middleware_chain = self.middleware_chain.with(middleware);
        self
    }

    pub fn build(self) -> MyCustomAgent<P> {
        MyCustomAgent {
            provider: Arc::new(self.provider.expect("Provider is required")),
            middleware_chain: self.middleware_chain,
        }
    }
}
```

### 方式 3：在工具执行中使用中间件

如果你的 Agent 支持工具调用，可以在工具执行时也使用中间件：

```rust
use agentkit::middleware::MiddlewareChain;
use agentkit_core::tool::types::{ToolCall, ToolResult};

// 在工具执行函数中
async fn execute_tool_with_middleware(
    middleware_chain: &MiddlewareChain,
    call: &ToolCall,
) -> Result<ToolResult, AgentError> {
    // 1. 工具调用前中间件
    let mut call_mut = call.clone();
    middleware_chain.process_tool_call_before(&mut call_mut).await?;

    // 2. 执行工具
    let mut result = self._execute_tool(&call_mut).await?;

    // 3. 工具调用后中间件
    middleware_chain.process_tool_call_after(&mut result).await?;

    Ok(result)
}
```

### 完整示例：创建带中间件的自定义 Agent

```rust
//! 自定义 Agent 使用中间件示例

use agentkit::agent::execution::DefaultExecution;
use agentkit::middleware::{Middleware, MiddlewareChain, LoggingMiddleware};
use agentkit::provider::OpenAiProvider;
use agentkit_core::agent::{Agent, AgentContext, AgentDecision, AgentInput, AgentOutput, AgentError};
use agentkit_core::provider::LlmProvider;
use agentkit_core::provider::types::{ChatMessage, ChatRequest};
use async_trait::async_trait;
use std::sync::Arc;

/// 自定义中间件：添加时间戳
#[derive(Clone)]
struct TimestampMiddleware;

#[async_trait]
impl Middleware for TimestampMiddleware {
    fn name(&self) -> &str {
        "timestamp"
    }

    async fn on_request(&self, input: &mut AgentInput) -> Result<(), AgentError> {
        let timestamp = chrono::Utc::now().to_rfc3339();
        input.text = format!("[{}] {}", timestamp, input.text);
        Ok(())
    }
}

/// 自定义 Agent
pub struct TimestampAgent<P> {
    execution: DefaultExecution,
    _provider: Arc<P>,
}

#[async_trait]
impl<P> Agent for TimestampAgent<P>
where
    P: LlmProvider + Send + Sync + 'static,
{
    async fn think(&self, context: &AgentContext) -> AgentDecision {
        AgentDecision::Chat {
            request: ChatRequest {
                messages: context.messages.clone(),
                ..Default::default()
            },
        }
    }

    fn name(&self) -> &str {
        "timestamp_agent"
    }

    async fn run(
        &self,
        input: AgentInput,
    ) -> Result<AgentOutput, AgentError> {
        self.execution.run(self, input).await
    }
}

/// 构建器
pub struct TimestampAgentBuilder<P> {
    provider: Option<P>,
    middleware_chain: MiddlewareChain,
}

impl<P> TimestampAgentBuilder<P> {
    pub fn new() -> Self {
        Self {
            provider: None,
            middleware_chain: MiddlewareChain::new(),
        }
    }

    pub fn provider(mut self, provider: P) -> Self {
        self.provider = Some(provider);
        self
    }

    pub fn with_middleware<M: Middleware + 'static>(
        mut self,
        middleware: M,
    ) -> Self {
        self.middleware_chain = self.middleware_chain.with(middleware);
        self
    }

    pub fn build(self) -> TimestampAgent<P> {
        let provider = Arc::new(self.provider.expect("Provider is required"));
        
        let execution = DefaultExecution::new(
            provider.clone(),
            "gpt-4o-mini",
            agentkit::agent::ToolRegistry::new(),
        )
        .with_middleware_chain(self.middleware_chain);

        TimestampAgent {
            execution,
            _provider: provider,
        }
    }
}

// 使用
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let provider = OpenAiProvider::from_env()?;
    
    let agent = TimestampAgent::builder()
        .provider(provider)
        .with_middleware(LoggingMiddleware::new())
        .with_middleware(TimestampMiddleware)
        .build();
    
    let output = agent.run("你好".into()).await?;
    println!("{}", output.text().unwrap());
    
    Ok(())
}
```

## 执行流程

### 完整执行流程

```
用户输入
    ↓
┌─────────────────────────────────┐
│ Middleware Chain (请求前)        │
│ → LoggingMiddleware              │
│ → RateLimitMiddleware            │
│ → AuthMiddleware                 │
│ → SensitiveWordFilter            │
└─────────────────────────────────┘
    ↓
┌─────────────────────────────────┐
│ Agent 处理                       │
│ → think()                        │
│ → LLM 调用                        │
│ → 工具执行                        │
│   ┌─────────────────────────┐   │
│   │ 工具调用中间件           │   │
│   │ → on_tool_call_before() │   │
│   │ → 工具执行               │   │
│   │ → on_tool_call_after()  │   │
│   └─────────────────────────┘   │
└─────────────────────────────────┘
    ↓
┌─────────────────────────────────┐
│ Middleware Chain (响应后，逆序)  │
│ ← ResponseFormat                 │
│ ← AuthMiddleware                 │
│ ← RateLimitMiddleware            │
│ ← LoggingMiddleware              │
└─────────────────────────────────┘
    ↓
返回给用户
```

### 中间件执行顺序

- **请求前**：按添加顺序执行（FIFO）
- **响应后**：按添加逆序执行（LIFO）
- **工具调用前**：按添加顺序执行
- **工具调用后**：按添加逆序执行

## 最佳实践

### 1. 中间件命名

使用有意义的名称：

```rust
fn name(&self) -> &str {
    "auth"  // ✅ 好
    // "middleware_1"  // ❌ 不好
}
```

### 2. 错误处理

优雅处理错误，不影响其他中间件：

```rust
async fn on_request(&self, input: &mut AgentInput) -> Result<(), AgentError> {
    match self.validate(input) {
        Ok(_) => Ok(()),
        Err(e) => {
            tracing::warn!("中间件验证失败：{}", e);
            Err(AgentError::Message(format!("验证失败：{}", e)))
        }
    }
}
```

### 3. 性能考虑

避免在中间件中进行耗时操作：

```rust
// ✅ 好 - 快速验证
async fn on_request(&self, input: &mut AgentInput) -> Result<(), AgentError> {
    if input.text.is_empty() {
        return Err(AgentError::Message("输入不能为空".to_string()));
    }
    Ok(())
}

// ❌ 不好 - 耗时操作
async fn on_request(&self, input: &mut AgentInput) -> Result<(), AgentError> {
    // 避免在这里进行网络请求或数据库查询
    let result = slow_database_query(input.text).await?;
    Ok(())
}
```

### 4. 中间件组合

将相关功能组织成独立中间件：

```rust
// ✅ 好 - 单一职责
let chain = MiddlewareChain::new()
    .with(AuthMiddleware::new("key"))
    .with(LoggingMiddleware::new())
    .with(CacheMiddleware::new());

// ❌ 不好 - 功能混杂
struct MegaMiddleware {
    auth: bool,
    logging: bool,
    caching: bool,
}
```

### 5. 可配置性

提供配置选项：

```rust
#[derive(Clone)]
struct RateLimitMiddleware {
    max_requests: usize,
    window_secs: u64,
}

impl RateLimitMiddleware {
    fn new(max_requests: usize) -> Self {
        Self {
            max_requests,
            window_secs: 60,
        }
    }

    fn with_window_secs(mut self, secs: u64) -> Self {
        self.window_secs = secs;
        self
    }
}
```

## 常见用例

### 1. 请求日志

```rust
#[derive(Clone)]
struct RequestLoggingMiddleware;

#[async_trait]
impl Middleware for RequestLoggingMiddleware {
    async fn on_request(&self, input: &mut AgentInput) -> Result<(), AgentError> {
        tracing::info!("收到请求：{}", input.text);
        Ok(())
    }

    async fn on_response(&self, output: &mut AgentOutput) -> Result<(), AgentError> {
        tracing::info!("返回响应：{}", output.text().unwrap_or(""));
        Ok(())
    }
}
```

### 2. 输入验证

```rust
#[derive(Clone)]
struct InputValidationMiddleware {
    max_length: usize,
}

#[async_trait]
impl Middleware for InputValidationMiddleware {
    async fn on_request(&self, input: &mut AgentInput) -> Result<(), AgentError> {
        if input.text.len() > self.max_length {
            return Err(AgentError::Message(
                format!("输入过长：最大 {} 字符", self.max_length)
            ));
        }
        Ok(())
    }
}
```

### 3. 响应缓存

```rust
use std::collections::HashMap;
use tokio::sync::RwLock;

#[derive(Clone)]
struct CacheMiddleware {
    cache: Arc<RwLock<HashMap<String, String>>>,
}

#[async_trait]
impl Middleware for CacheMiddleware {
    async fn on_request(&self, input: &mut AgentInput) -> Result<(), AgentError> {
        // 检查缓存（简化示例）
        Ok(())
    }

    async fn on_response(&self, output: &mut AgentOutput) -> Result<(), AgentError> {
        // 缓存响应（简化示例）
        Ok(())
    }
}
```

## 故障排除

### 中间件未执行

检查是否正确添加到 Agent：

```rust
// ❌ 错误 - 创建了中间件链但未使用
let chain = MiddlewareChain::new()
    .with(LoggingMiddleware::new());
let agent = ToolAgent::builder()
    .provider(provider)
    .build();  // 忘记添加 with_middleware_chain()

// ✅ 正确
let agent = ToolAgent::builder()
    .provider(provider)
    .with_middleware_chain(chain)
    .build();
```

### 中间件顺序问题

中间件执行顺序很重要：

```rust
// 认证应该在日志之前，避免记录未认证的请求
let chain = MiddlewareChain::new()
    .with(AuthMiddleware::new("key"))  // 先认证
    .with(LoggingMiddleware::new());   // 后日志
```

### 修改未生效

确保修改的是可变引用：

```rust
// ✅ 正确 - 修改可变引用
async fn on_request(&self, input: &mut AgentInput) -> Result<(), AgentError> {
    input.text = input.text.to_uppercase();
    Ok(())
}

// ❌ 错误 - 创建了新值但未使用
async fn on_request(&self, input: &mut AgentInput) -> Result<(), AgentError> {
    let new_text = input.text.to_uppercase();
    // new_text 未被使用
    Ok(())
}
```

## 运行示例

```bash
# 运行中间件示例
cargo run --example 08_middleware

# 示例演示：
# 1. 创建中间件链
# 2. 自定义中间件（认证、过滤、格式化）
# 3. SimpleAgent + 中间件
# 4. ChatAgent + 中间件
# 5. ToolAgent + 中间件
# 6. ReActAgent + 中间件
# 7. ReflectAgent + 中间件
```

## 总结

中间件系统提供了强大的扩展机制，让你可以：

- ✅ 模块化横切关注点
- ✅ 复用通用逻辑
- ✅ 灵活组合功能
- ✅ 不影响核心业务逻辑
- ✅ 支持所有 Agent 类型
- ✅ 支持工具调用拦截

通过合理使用中间件，可以构建更清晰、更易维护的 Agent 应用。
