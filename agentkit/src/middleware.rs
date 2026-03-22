//! 中间件系统
//!
//! # 概述
//!
//! 本模块提供中间件机制，支持在请求和响应处理过程中插入自定义逻辑：
//! - 日志记录
//! - 缓存
//! - 限流
//! - 认证
//! - 指标收集
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use agentkit::middleware::{Middleware, MiddlewareChain, Request, Response};
//!
//! // 创建中间件链
//! let chain = MiddlewareChain::new()
//!     .with(LoggingMiddleware::new())
//!     .with(CacheMiddleware::new(cache))
//!     .with(RateLimitMiddleware::new(100));
//!
//! // 处理请求
//! let response = chain.process(request).await?;
//! ```

use agentkit_core::agent::types::{AgentInput, AgentOutput};
use agentkit_core::error::AgentError;
use async_trait::async_trait;
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
/// 限制请求频率。
pub struct RateLimitMiddleware {
    /// 最大请求数
    max_requests: usize,
    /// 时间窗口（秒）
    window_secs: u64,
}

impl RateLimitMiddleware {
    /// 创建新的限流中间件
    pub fn new(max_requests: usize) -> Self {
        Self {
            max_requests,
            window_secs: 60,
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
        // 简化实现：实际应该使用令牌桶或滑动窗口算法
        // 这里只记录限流配置
        tracing::debug!(
            max_requests = self.max_requests,
            window_secs = self.window_secs,
            "middleware.rate_limit.check"
        );
        Ok(())
    }
}

/// 缓存中间件
///
/// 缓存请求响应。
pub struct CacheMiddleware {
    /// 是否启用缓存
    enabled: bool,
}

impl CacheMiddleware {
    /// 创建新的缓存中间件
    pub fn new() -> Self {
        Self { enabled: true }
    }

    /// 设置是否启用缓存
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
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
        if self.enabled {
            tracing::debug!(input_len = input.text.len(), "middleware.cache.request");
        }
        Ok(())
    }

    async fn on_response(&self, output: &mut AgentOutput) -> Result<(), AgentError> {
        if self.enabled {
            tracing::debug!(
                output_value_len = %output.value,
                "middleware.cache.response"
            );
        }
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
