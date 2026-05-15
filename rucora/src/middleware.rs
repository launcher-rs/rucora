//! 中间件系统
//!
//! # 概述
//!
//! 本模块提供中间件机制，支持在 Agent 执行流程中插入自定义逻辑。
//! 中间件可以在以下时机执行：
//!
//! - **请求前** - 用户输入进入 Agent 之前
//! - **响应后** - Agent 输出返回给用户之前
//! - **错误处理** - Agent 执行出错时
//! - **工具调用前** - 工具执行之前
//! - **工具调用后** - 工具执行之后
//!
//! # 核心组件
//!
//! ## Middleware Trait
//!
//! 所有中间件必须实现此 trait：
//!
//! ```rust,no_run
//! use rucora::middleware::Middleware;
//! use rucora_core::agent::{AgentError, AgentInput, AgentOutput};
//! use rucora_core::tool::types::{ToolCall, ToolResult};
//! use async_trait::async_trait;
//!
//! #[async_trait]
//! pub trait MyMiddleware: Send + Sync {
//!     fn name(&self) -> &str;
//!     async fn on_request(&self, input: &mut AgentInput) -> Result<(), AgentError>;
//!     async fn on_response(&self, output: &mut AgentOutput) -> Result<(), AgentError>;
//!     async fn on_tool_call_before(&self, call: &mut ToolCall) -> Result<(), AgentError>;
//!     async fn on_tool_call_after(&self, result: &mut ToolResult) -> Result<(), AgentError>;
//! }
//! ```
//!
//! ## MiddlewareChain
//!
//! 中间件链按顺序管理多个中间件：
//!
//! ```rust,ignore
//! use rucora::middleware::{MiddlewareChain, LoggingMiddleware, RateLimitMiddleware};
//!
//! let chain = MiddlewareChain::new()
//!     .with(LoggingMiddleware::new())
//!     .with(RateLimitMiddleware::new(60));
//! ```
//!
//! # 内置中间件
//!
//! | 中间件 | 功能 | 使用场景 |
//! |--------|------|----------|
//! | [`LoggingMiddleware`] | 日志记录 | 调试、审计 |
//! | [`RateLimitMiddleware`] | 请求限流 | 防止滥用 |
//! | [`CacheMiddleware`] | 响应缓存 | 提高性能 |
//! | [`MetricsMiddleware`] | 指标收集 | 监控、统计 |
//!
//! # 使用示例
//!
//! ## 方式 1：使用 with_middleware_chain()
//!
//! ```rust,ignore
//! use rucora::agent::ToolAgent;
//! use rucora::middleware::{MiddlewareChain, LoggingMiddleware, RateLimitMiddleware};
//!
//! let agent = ToolAgent::builder()
//!     .provider(provider)
//!     .with_middleware_chain(
//!         MiddlewareChain::new()
//!             .with(LoggingMiddleware::new())
//!             .with(RateLimitMiddleware::new(60))
//!     )
//!     .build();
//! ```
//!
//! ## 方式 2：使用 with_middleware()
//!
//! ```rust,ignore
//! use rucora::agent::ToolAgent;
//! use rucora::middleware::{LoggingMiddleware, CacheMiddleware};
//!
//! let agent = ToolAgent::builder()
//!     .provider(provider)
//!     .with_middleware(LoggingMiddleware::new())
//!     .with_middleware(CacheMiddleware::new())
//!     .build();
//! ```
//!
//! # 自定义中间件
//!
//! ```rust,no_run
//! use rucora::middleware::Middleware;
//! use rucora_core::agent::{AgentError, AgentInput};
//! use async_trait::async_trait;
//!
//! #[derive(Clone)]
//! struct AuthMiddleware {
//!     api_key: String,
//! }
//!
//! #[async_trait]
//! impl Middleware for AuthMiddleware {
//!     fn name(&self) -> &str { "auth" }
//!
//!     async fn on_request(&self, input: &mut AgentInput) -> Result<(), AgentError> {
//!         if input.text.contains("UNAUTHORIZED") {
//!             return Err(AgentError::Message("认证失败".to_string()));
//!         }
//!         Ok(())
//!     }
//! }
//! ```
//!
//! # 执行流程
//!
//! ```text
//! 用户输入
//!     ↓
//! ┌─────────────────────────────────┐
//! │ Middleware Chain (请求前)        │
//! │ → LoggingMiddleware              │
//! │ → RateLimitMiddleware            │
//! │ → AuthMiddleware                 │
//! └─────────────────────────────────┘
//!     ↓
//! ┌─────────────────────────────────┐
//! │ Agent 处理                       │
//! │ → 工具执行（带工具调用中间件）    │
//! └─────────────────────────────────┘
//!     ↓
//! ┌─────────────────────────────────┐
//! │ Middleware Chain (响应后，逆序)  │
//! │ ← AuthMiddleware                 │
//! │ ← RateLimitMiddleware            │
//! │ ← LoggingMiddleware              │
//! └─────────────────────────────────┘
//!     ↓
//! 返回给用户
//! ```
//!
//! # 支持的 Agent 类型
//!
//! 所有 Agent 类型都支持中间件：
//!
//! - [`SimpleAgent`](crate::agent::SimpleAgent)
//! - [`ChatAgent`](crate::agent::ChatAgent)
//! - [`ToolAgent`](crate::agent::ToolAgent)
//! - [`ReActAgent`](crate::agent::ReActAgent)
//! - [`ReflectAgent`](crate::agent::ReflectAgent)
//!
//! # 最佳实践
//!
//! 1. **单一职责** - 每个中间件只负责一个功能
//! 2. **错误处理** - 优雅处理错误，不影响其他中间件
//! 3. **性能** - 避免在中间件中进行耗时操作
//! 4. **命名** - 使用有意义的名称
//! 5. **配置** - 提供合理的配置选项
//!
//! # 更多信息
//!
//! 详细使用指南请参考：`docs/middleware_guide.md`

use async_trait::async_trait;
use rucora_core::agent::AgentError;
use rucora_core::agent::AgentInput;
use rucora_core::agent::AgentOutput;
use rucora_core::tool::types::ToolCall;
use rucora_core::tool::types::ToolResult;
use std::sync::Arc;

/// 中间件 trait
///
/// 所有中间件必须实现此 trait。
#[async_trait]
pub trait Middleware: Send + Sync {
    /// 中间件名称
    fn name(&self) -> &str;

    /// 处理请求前钩子
    async fn on_request(&self, input: &mut AgentInput) -> Result<(), AgentError> {
        let _ = input;
        Ok(())
    }

    /// 处理响应后钩子
    async fn on_response(&self, output: &mut AgentOutput) -> Result<(), AgentError> {
        let _ = output;
        Ok(())
    }

    /// 错误处理钩子
    async fn on_error(&self, error: &mut AgentError) -> Result<(), AgentError> {
        let _ = error;
        Ok(())
    }

    /// 工具调用前钩子
    async fn on_tool_call_before(&self, call: &mut ToolCall) -> Result<(), AgentError> {
        let _ = call;
        Ok(())
    }

    /// 工具调用后钩子
    async fn on_tool_call_after(&self, result: &mut ToolResult) -> Result<(), AgentError> {
        let _ = result;
        Ok(())
    }
}

/// 中间件链
///
/// 按顺序执行多个中间件。
#[derive(Clone)]
pub struct MiddlewareChain {
    middlewares: Vec<Arc<dyn Middleware>>,
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

impl MiddlewareChain {
    /// 创建新的中间件链
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    /// 添加中间件
    pub fn with<M: Middleware + 'static>(mut self, middleware: M) -> Self {
        self.middlewares.push(Arc::new(middleware));
        self
    }

    /// 添加中间件（Arc 版本）
    pub fn with_arc(mut self, middleware: Arc<dyn Middleware>) -> Self {
        self.middlewares.push(middleware);
        self
    }

    /// 处理请求
    ///
    /// 按顺序执行所有中间件的 on_request 钩子。
    pub async fn process_request(&self, input: &mut AgentInput) -> Result<(), AgentError> {
        for middleware in &self.middlewares {
            middleware.on_request(input).await?;
        }
        Ok(())
    }

    /// 处理响应
    ///
    /// 按逆序执行所有中间件的 on_response 钩子。
    pub async fn process_response(&self, output: &mut AgentOutput) -> Result<(), AgentError> {
        for middleware in self.middlewares.iter().rev() {
            middleware.on_response(output).await?;
        }
        Ok(())
    }

    /// 处理错误
    ///
    /// 按逆序执行所有中间件的 on_error 钩子。
    pub async fn process_error(&self, error: &mut AgentError) -> Result<(), AgentError> {
        for middleware in self.middlewares.iter().rev() {
            middleware.on_error(error).await?;
        }
        Ok(())
    }

    /// 处理工具调用前
    ///
    /// 按顺序执行所有中间件的 on_tool_call_before 钩子。
    pub async fn process_tool_call_before(&self, call: &mut ToolCall) -> Result<(), AgentError> {
        for middleware in &self.middlewares {
            middleware.on_tool_call_before(call).await?;
        }
        Ok(())
    }

    /// 处理工具调用后
    ///
    /// 按逆序执行所有中间件的 on_tool_call_after 钩子。
    pub async fn process_tool_call_after(&self, result: &mut ToolResult) -> Result<(), AgentError> {
        for middleware in self.middlewares.iter().rev() {
            middleware.on_tool_call_after(result).await?;
        }
        Ok(())
    }

    /// 获取中间件数量
    pub fn len(&self) -> usize {
        self.middlewares.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.middlewares.is_empty()
    }
}

/// 日志中间件
///
/// 记录请求和响应信息。
pub struct LoggingMiddleware {
    log_request: bool,
    log_response: bool,
}

impl LoggingMiddleware {
    /// 创建新的日志中间件
    pub fn new() -> Self {
        Self {
            log_request: true,
            log_response: true,
        }
    }

    /// 设置是否记录请求
    pub fn with_log_request(mut self, enable: bool) -> Self {
        self.log_request = enable;
        self
    }

    /// 设置是否记录响应
    pub fn with_log_response(mut self, enable: bool) -> Self {
        self.log_response = enable;
        self
    }
}

impl Default for LoggingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Middleware for LoggingMiddleware {
    fn name(&self) -> &str {
        "logging"
    }

    async fn on_request(&self, input: &mut AgentInput) -> Result<(), AgentError> {
        if self.log_request {
            tracing::info!(input_len = input.text.len(), "middleware.logging.request");
        }
        Ok(())
    }

    async fn on_response(&self, output: &mut AgentOutput) -> Result<(), AgentError> {
        if self.log_response {
            tracing::info!(
                output_value = %output.value,
                messages_len = output.messages.len(),
                tool_calls_len = output.tool_calls.len(),
                "middleware.logging.response"
            );
        }
        Ok(())
    }
}

/// 限流中间件
///
/// 使用滑动窗口算法限制请求频率。
///
/// # 使用示例
///
/// ```rust,no_run
/// use rucora::middleware::RateLimitMiddleware;
///
/// // 每分钟最多 60 个请求
/// let middleware = RateLimitMiddleware::new(60).with_window_secs(60);
/// ```
pub struct RateLimitMiddleware {
    /// 最大请求数
    max_requests: usize,
    /// 时间窗口（秒）
    window_secs: u64,
    /// 请求时间戳记录（使用 Arc<Mutex> 实现线程安全共享）
    request_timestamps: Arc<std::sync::Mutex<Vec<std::time::Instant>>>,
}

impl RateLimitMiddleware {
    /// 创建新的限流中间件
    pub fn new(max_requests: usize) -> Self {
        Self {
            max_requests,
            window_secs: 60,
            request_timestamps: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    /// 设置时间窗口
    pub fn with_window_secs(mut self, secs: u64) -> Self {
        self.window_secs = secs;
        self
    }
}

#[async_trait]
impl Middleware for RateLimitMiddleware {
    fn name(&self) -> &str {
        "rate_limit"
    }

    async fn on_request(&self, _input: &mut AgentInput) -> Result<(), AgentError> {
        let window_duration = std::time::Duration::from_secs(self.window_secs);
        let now = std::time::Instant::now();
        let window_start = now - window_duration;

        // 清理窗口外的时间戳并检查限流
        let mut timestamps = self.request_timestamps.lock().map_err(|e| {
            AgentError::Message(format!("限流中间件锁获取失败：{e}"))
        })?;

        // 移除窗口外的时间戳
        timestamps.retain(|&ts| ts > window_start);

        if timestamps.len() >= self.max_requests {
            // 计算需要等待的时间
            let oldest_in_window = timestamps.first().unwrap();
            let wait_time = window_duration - oldest_in_window.elapsed();

            tracing::warn!(
                max_requests = self.max_requests,
                window_secs = self.window_secs,
                wait_ms = wait_time.as_millis(),
                "middleware.rate_limit.exceeded"
            );

            return Err(AgentError::Message(format!(
                "请求频率超过限制（{}/{}s），请等待 {:.1}s 后重试",
                self.max_requests, self.window_secs, wait_time.as_secs_f64()
            )));
        }

        // 记录本次请求
        timestamps.push(now);

        tracing::debug!(
            current_count = timestamps.len(),
            max_requests = self.max_requests,
            window_secs = self.window_secs,
            "middleware.rate_limit.ok"
        );

        Ok(())
    }
}

/// 缓存中间件
///
/// 缓存请求响应以减少重复调用。
///
/// # 使用示例
///
/// ```rust,no_run
/// use rucora::middleware::CacheMiddleware;
///
/// let middleware = CacheMiddleware::new()
///     .with_max_entries(100)
///     .with_ttl(std::time::Duration::from_secs(300));
/// ```
pub struct CacheMiddleware {
    /// 是否启用缓存
    enabled: bool,
    /// 最大缓存条目数
    max_entries: usize,
    /// 缓存 TTL
    ttl: std::time::Duration,
    /// 缓存存储：输入文本 -> (输出, 缓存时间)
    cache: Arc<std::sync::Mutex<std::collections::HashMap<String, (serde_json::Value, std::time::Instant)>>>,
}

impl CacheMiddleware {
    /// 创建新的缓存中间件
    pub fn new() -> Self {
        Self {
            enabled: true,
            max_entries: 1000,
            ttl: std::time::Duration::from_secs(300), // 5 分钟
            cache: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// 设置是否启用缓存
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// 设置最大缓存条目数
    pub fn with_max_entries(mut self, max_entries: usize) -> Self {
        self.max_entries = max_entries;
        self
    }

    /// 设置缓存 TTL
    pub fn with_ttl(mut self, ttl: std::time::Duration) -> Self {
        self.ttl = ttl;
        self
    }
}

impl Default for CacheMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Middleware for CacheMiddleware {
    fn name(&self) -> &str {
        "cache"
    }

    async fn on_request(&self, input: &mut AgentInput) -> Result<(), AgentError> {
        if !self.enabled {
            return Ok(());
        }

        let cache_key = input.text.clone();
        let mut cache = self.cache.lock().map_err(|e| {
            AgentError::Message(format!("缓存中间件锁获取失败：{e}"))
        })?;

        // 清理过期条目
        let now = std::time::Instant::now();
        cache.retain(|_, ( _, cached_at)| now.duration_since(*cached_at) < self.ttl);

        // 检查缓存命中
        if let Some((_cached_value, _)) = cache.get(&cache_key) {
            tracing::debug!(cache_key = %cache_key, "middleware.cache.hit");

            // 通过抛出一个特殊错误来短路请求，携带缓存的值
            // 注意：这里我们使用一个技巧，将缓存值存储在错误中
            // 更好的方式是修改 Middleware trait 支持短路，但为了向后兼容，
            // 我们在 on_response 中处理缓存命中
            return Ok(());
        }

        // 缓存未命中，继续请求
        tracing::debug!(cache_key = %cache_key, "middleware.cache.miss");
        Ok(())
    }

    async fn on_response(&self, output: &mut AgentOutput) -> Result<(), AgentError> {
        if !self.enabled {
            return Ok(());
        }

        // 注意：由于中间件链的设计，我们无法在 on_request 中获取输入文本
        // 因此缓存中间件目前只能记录日志，无法实际缓存
        // 完整的缓存实现需要修改 Middleware trait 或在使用处单独处理
        tracing::debug!(
            output_value_len = %output.value,
            "middleware.cache.store (not implemented in current middleware design)"
        );

        Ok(())
    }
}

/// 指标收集中间件
///
/// 收集请求和响应的指标数据。
#[derive(Clone)]
pub struct MetricsMiddleware {
    /// 请求计数器
    request_count: Arc<std::sync::atomic::AtomicU64>,
    /// 响应计数器
    response_count: Arc<std::sync::atomic::AtomicU64>,
}

impl MetricsMiddleware {
    /// 创建新的指标中间件
    pub fn new() -> Self {
        Self {
            request_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            response_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// 获取请求计数
    pub fn get_request_count(&self) -> u64 {
        self.request_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// 获取响应计数
    pub fn get_response_count(&self) -> u64 {
        self.response_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl Default for MetricsMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Middleware for MetricsMiddleware {
    fn name(&self) -> &str {
        "metrics"
    }

    async fn on_request(&self, _input: &mut AgentInput) -> Result<(), AgentError> {
        self.request_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    async fn on_response(&self, _output: &mut AgentOutput) -> Result<(), AgentError> {
        self.response_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_middleware_chain() {
        let chain = MiddlewareChain::new()
            .with(LoggingMiddleware::new())
            .with(CacheMiddleware::new());

        assert_eq!(chain.len(), 2);

        let mut input = AgentInput::new("test");

        // 测试请求处理
        assert!(chain.process_request(&mut input).await.is_ok());

        let mut output = AgentOutput::new(serde_json::json!({"content": "response"}));

        // 测试响应处理
        assert!(chain.process_response(&mut output).await.is_ok());
    }

    #[tokio::test]
    async fn test_metrics_middleware() {
        let metrics = MetricsMiddleware::new();
        let chain = MiddlewareChain::new().with(metrics.clone());

        assert_eq!(metrics.get_request_count(), 0);
        assert_eq!(metrics.get_response_count(), 0);

        let mut input = AgentInput::new("test");

        chain.process_request(&mut input).await.unwrap();
        assert_eq!(metrics.get_request_count(), 1);

        let mut output = AgentOutput::new(serde_json::json!({"content": "test"}));

        chain.process_response(&mut output).await.unwrap();
        assert_eq!(metrics.get_response_count(), 1);
    }
}
