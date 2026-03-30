//! Middleware 使用示例
//!
//! 展示如何在 Provider、Agent、Runtime 中使用 Middleware
//!
//! # 运行方式
//!
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! export OPENAI_BASE_URL=http://your-server:11434/v1
//!
//! cargo run --example 08_middleware -p agentkit
//! ```

use agentkit::agent::DefaultAgent;
use agentkit::middleware::{LoggingMiddleware, Middleware, MiddlewareChain, RateLimitMiddleware};
use agentkit::prelude::*;
use agentkit::provider::OpenAiProvider;
use agentkit::runtime::{DefaultRuntime, ToolRegistry};
use agentkit_core::Runtime;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Instant;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

// 自定义中间件：性能监控
pub struct PerformanceMonitorMiddleware {
    start_time: RwLock<Option<Instant>>,
}

impl PerformanceMonitorMiddleware {
    pub fn new() -> Self {
        Self {
            start_time: RwLock::new(None),
        }
    }
}

impl Default for PerformanceMonitorMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Middleware for PerformanceMonitorMiddleware {
    fn name(&self) -> &str {
        "performance_monitor"
    }

    async fn on_request(
        &self,
        input: &mut AgentInput,
    ) -> Result<(), agentkit_core::error::AgentError> {
        *self.start_time.write().unwrap() = Some(Instant::now());
        info!("⏱️  [性能监控] 开始处理请求：{} 字符", input.text.len());
        Ok(())
    }

    async fn on_response(
        &self,
        output: &mut AgentOutput,
    ) -> Result<(), agentkit_core::error::AgentError> {
        if let Some(start) = *self.start_time.read().unwrap() {
            let elapsed = start.elapsed();
            info!(
                "⏱️  [性能监控] 请求完成：耗时 {:?}, 回复 {} 字符",
                elapsed,
                output.text().map(|s| s.len()).unwrap_or(0)
            );
        }
        Ok(())
    }

    async fn on_error(
        &self,
        _error: &mut agentkit_core::error::AgentError,
    ) -> Result<(), agentkit_core::error::AgentError> {
        if let Some(start) = *self.start_time.read().unwrap() {
            let elapsed = start.elapsed();
            info!("⏱️  [性能监控] 请求失败：耗时 {:?}", elapsed);
        }
        Ok(())
    }
}

// 自定义中间件：请求修改
pub struct PrefixMiddleware {
    prefix: String,
}

impl PrefixMiddleware {
    pub fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl Middleware for PrefixMiddleware {
    fn name(&self) -> &str {
        "prefix"
    }

    async fn on_request(
        &self,
        input: &mut AgentInput,
    ) -> Result<(), agentkit_core::error::AgentError> {
        // 在用户输入前添加前缀
        input.text = format!("{} {}", self.prefix, input.text);
        info!("📝  [前缀中间件] 已添加前缀：{}", self.prefix);
        Ok(())
    }
}

// 自定义中间件：响应过滤
pub struct ResponseFilterMiddleware {
    forbidden_words: Vec<String>,
}

impl ResponseFilterMiddleware {
    pub fn new(forbidden_words: Vec<&str>) -> Self {
        Self {
            forbidden_words: forbidden_words.into_iter().map(|s| s.to_string()).collect(),
        }
    }
}

#[async_trait::async_trait]
impl Middleware for ResponseFilterMiddleware {
    fn name(&self) -> &str {
        "response_filter"
    }

    async fn on_response(
        &self,
        output: &mut AgentOutput,
    ) -> Result<(), agentkit_core::error::AgentError> {
        // 过滤敏感词
        if let Some(text) = output.text() {
            let mut filtered_text = text.to_string();
            for word in &self.forbidden_words {
                filtered_text = filtered_text.replace(word, "***");
            }
            if filtered_text != text {
                info!("🚫  [过滤中间件] 已过滤敏感词");
                // 注意：AgentOutput 的 content 是只读的，这里只是示例
            }
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔═══════════════════════════════════════════════════════════╗");
    info!("║         AgentKit Middleware 使用示例                      ║");
    info!("╚═══════════════════════════════════════════════════════════╝\n");

    // 示例 2: 在 Agent 中使用 Middleware
    info!("\n=== 示例 2: 在 Agent 中使用 Middleware ===\n");
    test_agent_with_middleware().await?;

    // 示例 3: 在 Runtime 中使用 Middleware
    info!("\n=== 示例 3: 在 Runtime 中使用 Middleware ===\n");
    test_runtime_with_middleware().await?;

    // 示例 4: 自定义中间件链
    info!("\n=== 示例 4: 自定义中间件链 ===\n");
    test_custom_middleware_chain().await?;

    info!("\n=== 所有示例完成 ===");

    Ok(())
}

/// 示例 2: 在 Agent 中使用 Middleware
async fn test_agent_with_middleware() -> anyhow::Result<()> {
    info!("创建 Agent 和中间件链...");

    // 创建中间件链
    let middleware_chain = MiddlewareChain::new()
        .with(LoggingMiddleware::new())
        .with(PerformanceMonitorMiddleware::new())
        .with(PrefixMiddleware::new("[助手模式]"))
        .with(ResponseFilterMiddleware::new(vec!["错误", "失败"]));

    info!("✓ 中间件链创建成功 ({} 个中间件)", middleware_chain.len());

    // 创建 Agent
    let provider = OpenAiProvider::from_env()?;
    let model = "qwen3.5:9b";

    let agent = DefaultAgent::builder()
        .provider(provider)
        .model(model)
        .with_conversation(false)
        .build();

    info!("✓ Agent 创建成功 (模型：{})", model);

    // 测试 Agent（带中间件处理）
    info!("\n测试 Agent 请求...");
    let mut input = AgentInput::new("用一句话介绍 Rust");

    // 处理请求中间件
    middleware_chain.process_request(&mut input).await?;

    // 运行 Agent
    match agent.run(input).await {
        Ok(mut output) => {
            // 处理响应中间件
            middleware_chain.process_response(&mut output).await?;

            if let Some(text) = output.text() {
                info!(
                    "✓ Agent 回复：{}",
                    text.chars().take(50).collect::<String>()
                );
            }
        }
        Err(e) => {
            info!("✗ Agent 错误：{}", e);
        }
    }

    Ok(())
}

/// 示例 3: 在 Runtime 中使用 Middleware
async fn test_runtime_with_middleware() -> anyhow::Result<()> {
    info!("创建 Runtime 和中间件链...");

    // 创建中间件链
    let middleware_chain = MiddlewareChain::new()
        .with(LoggingMiddleware::new())
        .with(PerformanceMonitorMiddleware::new())
        .with(RateLimitMiddleware::new(5)); // 每分钟最多 5 次请求

    info!("✓ 中间件链创建成功 ({} 个中间件)", middleware_chain.len());

    // 创建 Runtime
    let provider = OpenAiProvider::from_env()?;
    let model = "qwen3.5:9b";

    let runtime = DefaultRuntime::new(Arc::new(provider), ToolRegistry::new(), model)
        .with_system_prompt("你是一个简洁的助手，回答要简短。");

    info!("✓ Runtime 创建成功 (模型：{})", model);

    // 测试 Runtime（带中间件处理）
    info!("\n测试 Runtime 请求...");
    let mut input = AgentInput::new("1+1 等于几？");

    // 处理请求中间件
    middleware_chain.process_request(&mut input).await?;

    // 运行 Runtime
    match runtime.run(input).await {
        Ok(mut output) => {
            // 处理响应中间件
            middleware_chain.process_response(&mut output).await?;

            if let Some(text) = output.text() {
                info!(
                    "✓ Runtime 回复：{}",
                    text.chars().take(50).collect::<String>()
                );
            }
        }
        Err(e) => {
            info!("✗ Runtime 错误：{}", e);
        }
    }

    Ok(())
}

/// 示例 4: 自定义中间件链
async fn test_custom_middleware_chain() -> anyhow::Result<()> {
    info!("创建自定义中间件链...");

    // 创建多个中间件
    let logging = LoggingMiddleware::new()
        .with_log_request(true)
        .with_log_response(true);

    let rate_limit = RateLimitMiddleware::new(10).with_window_secs(60);

    let perf_monitor = PerformanceMonitorMiddleware::new();
    let prefix = PrefixMiddleware::new("[测试模式]");
    let filter = ResponseFilterMiddleware::new(vec!["测试"]);

    // 构建中间件链
    let middleware_chain = MiddlewareChain::new()
        .with(logging)
        .with(perf_monitor)
        .with(rate_limit)
        .with(prefix)
        .with(filter);

    info!(
        "✓ 自定义中间件链创建成功 ({} 个中间件)",
        middleware_chain.len()
    );

    // 测试中间件链
    info!("\n测试中间件链处理...");

    let mut input = AgentInput::new("这是一个测试请求");
    info!("原始输入：{}", input.text);

    // 处理请求
    middleware_chain.process_request(&mut input).await?;
    info!("处理后输入：{}", input.text);

    // 创建模拟响应
    let mut output =
        AgentOutput::new(serde_json::json!({"content": "这是一个测试回复，不包含敏感词"}));

    // 处理响应
    middleware_chain.process_response(&mut output).await?;

    if let Some(text) = output.text() {
        info!(
            "✓ 响应处理完成：{}",
            text.chars().take(50).collect::<String>()
        );
    }

    Ok(())
}
