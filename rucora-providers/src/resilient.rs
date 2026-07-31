use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use futures_util::{StreamExt, stream::BoxStream};
use rucora_core::error::ProviderError;
use rucora_core::provider::LlmProvider;
use rucora_core::provider::types::ChatRequest;
use tokio::time::{Duration, sleep, timeout};
use tracing::warn;

/// Provider 错误类型分类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderErrorCategory {
    /// 网络错误（可重试）
    Network,
    /// 超时错误（可重试）
    Timeout,
    /// 限流错误（可重试，需要退避）
    RateLimit,
    /// 认证错误（不可重试）
    Auth,
    /// 无效请求（不可重试）
    InvalidRequest,
    /// 服务不可用（可重试）
    Unavailable,
    /// 其他错误（默认不重试）
    Other,
}

impl ProviderErrorCategory {
    /// 判断是否可重试
    pub fn is_retriable(&self) -> bool {
        matches!(
            self,
            ProviderErrorCategory::Network
                | ProviderErrorCategory::Timeout
                | ProviderErrorCategory::RateLimit
                | ProviderErrorCategory::Unavailable
        )
    }

    /// 从错误消息中分类错误
    pub fn from_error_message(msg: &str) -> Self {
        let lower = msg.to_lowercase();

        // 认证错误
        if lower.contains("auth")
            || lower.contains("unauthorized")
            || lower.contains("401")
            || lower.contains("api key")
            || lower.contains("permission")
        {
            return ProviderErrorCategory::Auth;
        }

        // 无效请求
        if lower.contains("invalid")
            || lower.contains("bad request")
            || lower.contains("400")
            || lower.contains("not found")
            || lower.contains("404")
        {
            return ProviderErrorCategory::InvalidRequest;
        }

        // 限流
        if lower.contains("rate limit")
            || lower.contains("too many requests")
            || lower.contains("429")
        {
            return ProviderErrorCategory::RateLimit;
        }

        // 超时
        if lower.contains("timeout") || lower.contains("timed out") {
            return ProviderErrorCategory::Timeout;
        }

        // 网络错误
        if lower.contains("network")
            || lower.contains("connection")
            || lower.contains("dns")
            || lower.contains("socket")
            || lower.contains("reset")
            || lower.contains("unreachable")
        {
            return ProviderErrorCategory::Network;
        }

        // 服务不可用
        if lower.contains("unavailable")
            || lower.contains("503")
            || lower.contains("502")
            || lower.contains("504")
        {
            return ProviderErrorCategory::Unavailable;
        }

        ProviderErrorCategory::Other
    }

    /// 从 HTTP 状态码分类错误
    pub fn from_status_code(status: u16) -> Self {
        match status {
            400 => ProviderErrorCategory::InvalidRequest,
            401 | 403 => ProviderErrorCategory::Auth,
            404 => ProviderErrorCategory::InvalidRequest,
            429 => ProviderErrorCategory::RateLimit,
            500 | 502 | 503 | 504 => ProviderErrorCategory::Unavailable,
            _ => ProviderErrorCategory::Other,
        }
    }
}

/// 重试配置
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: usize,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub timeout_ms: Option<u64>,
    /// 是否对不可重试的错误也尝试一次（默认 false）
    pub retry_non_retriable_once: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 2,
            base_delay_ms: 200,
            max_delay_ms: 2_000,
            timeout_ms: None,
            retry_non_retriable_once: false,
        }
    }
}

impl RetryConfig {
    /// 创建默认配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置最大重试次数
    pub fn with_max_retries(mut self, max: usize) -> Self {
        self.max_retries = max;
        self
    }

    /// 设置基础延迟（毫秒）
    pub fn with_base_delay_ms(mut self, delay: u64) -> Self {
        self.base_delay_ms = delay;
        self
    }

    /// 设置最大延迟（毫秒）
    pub fn with_max_delay_ms(mut self, delay: u64) -> Self {
        self.max_delay_ms = delay;
        self
    }

    /// 设置超时（毫秒）
    pub fn with_timeout_ms(mut self, timeout: u64) -> Self {
        self.timeout_ms = Some(timeout);
        self
    }
}

/// 取消句柄（CancellationToken）
///
/// 用于从外部取消正在进行的流式 Agent 调用。
///
/// # 使用场景
///
/// - 用户点击"停止"按钮中断长时间运行的 Agent 任务
/// - 超时后强制取消正在进行的流式响应
/// - 在多客户端环境中隔离取消操作
///
/// # 示例
///
/// ```rust,ignore
/// use rucora_providers::resilient::CancelHandle;
/// use std::sync::Arc;
/// use std::time::Duration;
///
/// # async fn example() {
/// let (handle, stream) = { /* 创建可取消的流 */ };
/// let cancel_handle = Arc::new(handle);
///
/// // 在另一个线程中取消
/// let handle_clone = cancel_handle.clone();
/// tokio::spawn(async move {
///     tokio::time::sleep(Duration::from_secs(30)).await;
///     handle_clone.cancel(); // 30秒后取消
/// });
///
/// // 消费流（会自动检查取消状态）
/// // ...
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct CancelHandle {
    cancelled: Arc<AtomicBool>,
}

impl CancelHandle {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

#[derive(Clone)]
pub struct ResilientProvider {
    inner: Arc<dyn LlmProvider>,
    cfg: RetryConfig,
}

impl ResilientProvider {
    pub fn new(inner: Arc<dyn LlmProvider>) -> Self {
        Self {
            inner,
            cfg: RetryConfig::default(),
        }
    }

    pub fn with_config(mut self, cfg: RetryConfig) -> Self {
        self.cfg = cfg;
        self
    }

    /// 计算退避延迟（指数退避 + 抖动）
    fn backoff_delay_ms(&self, attempt: usize) -> u64 {
        let pow = 1u64.checked_shl(attempt.min(16) as u32).unwrap_or(u64::MAX);
        let delay = self
            .cfg
            .base_delay_ms
            .saturating_mul(pow)
            .min(self.cfg.max_delay_ms);

        // 添加 10% 的抖动，使用 attempt 作为简单种子生成伪随机值
        let jitter = (delay / 10).max(1); // 确保 jitter 不为 0
        // 使用简单的伪随机：基于 attempt 的哈希值（wrapping 避免大 attempt 溢出）
        let jitter_offset = (attempt as u64)
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1)
            % jitter;

        delay + jitter_offset
    }

    /// 判断错误是否可重试
    fn should_retry(&self, error: &ProviderError, attempt: usize) -> bool {
        if error.is_retriable() {
            return true;
        }

        let msg = error.to_string();
        let category = ProviderErrorCategory::from_error_message(&msg);

        if category.is_retriable() {
            return true;
        }

        // 对于不可重试的错误，如果配置允许且是第一次重试，也可以尝试
        if attempt == 0 && self.cfg.retry_non_retriable_once {
            warn!(error = %error, "resilient: 不可重试的错误，但配置允许尝试一次");
            return true;
        }

        false
    }

    pub fn stream_chat_cancellable(
        &self,
        request: ChatRequest,
    ) -> Result<
        (
            CancelHandle,
            BoxStream<
                'static,
                Result<
                    rucora_core::provider::types::ChatStreamChunk,
                    rucora_core::error::ProviderError,
                >,
            >,
        ),
        rucora_core::error::ProviderError,
    > {
        let cancelled = Arc::new(AtomicBool::new(false));
        let handle = CancelHandle {
            cancelled: cancelled.clone(),
        };

        let inner_stream = self.inner.stream_chat(request)?;
        let stream = async_stream::try_stream! {
            futures_util::pin_mut!(inner_stream);
            while let Some(item) = inner_stream.next().await {
                if cancelled.load(Ordering::SeqCst) {
                    break;
                }
                yield item?;
            }
        };

        Ok((handle, Box::pin(stream)))
    }
}

#[async_trait]
impl LlmProvider for ResilientProvider {
    async fn chat(
        &self,
        request: ChatRequest,
    ) -> Result<rucora_core::provider::types::ChatResponse, rucora_core::error::ProviderError> {
        let mut attempt = 0usize;

        loop {
            let fut = self.inner.chat(request.clone());

            let result = if let Some(ms) = self.cfg.timeout_ms {
                match timeout(Duration::from_millis(ms), fut).await {
                    Ok(r) => r,
                    Err(_) => Err(ProviderError::Message(format!(
                        "provider chat timeout after {ms}ms"
                    ))),
                }
            } else {
                fut.await
            };

            match result {
                Ok(v) => return Ok(v),
                Err(e) => {
                    // 判断是否应该重试
                    if !self.should_retry(&e, attempt) {
                        warn!(
                            attempt,
                            error = %e,
                            category = ?ProviderErrorCategory::from_error_message(&e.to_string()),
                            "resilient: 错误不可重试，直接返回"
                        );
                        return Err(e);
                    }

                    // 检查是否超过最大重试次数
                    if attempt >= self.cfg.max_retries {
                        warn!(
                            attempt,
                            max_retries = self.cfg.max_retries,
                            error = %e,
                            "resilient: 超过最大重试次数"
                        );
                        return Err(e);
                    }

                    // 计算延迟并等待
                    let delay = self.backoff_delay_ms(attempt);
                    warn!(
                        attempt,
                        delay_ms = delay,
                        error = %e,
                        category = ?ProviderErrorCategory::from_error_message(&e.to_string()),
                        "resilient: 重试中"
                    );
                    sleep(Duration::from_millis(delay)).await;
                    attempt += 1;
                }
            }
        }
    }

    fn stream_chat(
        &self,
        request: ChatRequest,
    ) -> Result<
        BoxStream<
            'static,
            Result<
                rucora_core::provider::types::ChatStreamChunk,
                rucora_core::error::ProviderError,
            >,
        >,
        rucora_core::error::ProviderError,
    > {
        self.inner.stream_chat(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_category_retriable() {
        assert!(ProviderErrorCategory::Network.is_retriable());
        assert!(ProviderErrorCategory::Timeout.is_retriable());
        assert!(ProviderErrorCategory::RateLimit.is_retriable());
        assert!(ProviderErrorCategory::Unavailable.is_retriable());
        assert!(!ProviderErrorCategory::Auth.is_retriable());
        assert!(!ProviderErrorCategory::InvalidRequest.is_retriable());
        assert!(!ProviderErrorCategory::Other.is_retriable());
    }

    #[test]
    fn test_classify_from_error_message_auth() {
        assert_eq!(
            ProviderErrorCategory::from_error_message("401 Unauthorized: api key invalid"),
            ProviderErrorCategory::Auth
        );
        assert_eq!(
            ProviderErrorCategory::from_error_message("Invalid API key provided"),
            ProviderErrorCategory::Auth
        );
    }

    #[test]
    fn test_classify_from_error_message_invalid() {
        assert_eq!(
            ProviderErrorCategory::from_error_message("400 Bad Request"),
            ProviderErrorCategory::InvalidRequest
        );
        assert_eq!(
            ProviderErrorCategory::from_error_message("404 Not Found"),
            ProviderErrorCategory::InvalidRequest
        );
    }

    #[test]
    fn test_classify_from_error_message_rate_limit() {
        assert_eq!(
            ProviderErrorCategory::from_error_message("rate limit exceeded"),
            ProviderErrorCategory::RateLimit
        );
        assert_eq!(
            ProviderErrorCategory::from_error_message("429 too many requests"),
            ProviderErrorCategory::RateLimit
        );
    }

    #[test]
    fn test_classify_from_error_message_timeout() {
        assert_eq!(
            ProviderErrorCategory::from_error_message("request timed out"),
            ProviderErrorCategory::Timeout
        );
    }

    #[test]
    fn test_classify_from_error_message_network() {
        assert_eq!(
            ProviderErrorCategory::from_error_message("connection reset by peer"),
            ProviderErrorCategory::Network
        );
        assert_eq!(
            ProviderErrorCategory::from_error_message("dns lookup failed"),
            ProviderErrorCategory::Network
        );
    }

    #[test]
    fn test_classify_from_error_message_unavailable() {
        assert_eq!(
            ProviderErrorCategory::from_error_message("503 service unavailable"),
            ProviderErrorCategory::Unavailable
        );
    }

    #[test]
    fn test_classify_from_error_message_unknown() {
        assert_eq!(
            ProviderErrorCategory::from_error_message("some random message"),
            ProviderErrorCategory::Other
        );
    }

    #[test]
    fn test_classify_from_status_code() {
        assert_eq!(ProviderErrorCategory::from_status_code(400), ProviderErrorCategory::InvalidRequest);
        assert_eq!(ProviderErrorCategory::from_status_code(401), ProviderErrorCategory::Auth);
        assert_eq!(ProviderErrorCategory::from_status_code(403), ProviderErrorCategory::Auth);
        assert_eq!(ProviderErrorCategory::from_status_code(404), ProviderErrorCategory::InvalidRequest);
        assert_eq!(ProviderErrorCategory::from_status_code(429), ProviderErrorCategory::RateLimit);
        assert_eq!(ProviderErrorCategory::from_status_code(500), ProviderErrorCategory::Unavailable);
        assert_eq!(ProviderErrorCategory::from_status_code(200), ProviderErrorCategory::Other);
    }

    #[test]
    fn test_backoff_delay_respects_max() {
        let cfg = RetryConfig::new()
            .with_max_retries(10)
            .with_base_delay_ms(100)
            .with_max_delay_ms(500);
        let inner = crate::OpenAiProvider::with_model("https://example.com", "key", "model");
        let provider = ResilientProvider::new(Arc::new(inner));
        let provider = provider.with_config(cfg);
        // 基础延迟受 max_delay_ms 上限约束，抖动最多再加 10%
        let max_allowed = 500 + 500 / 10;
        for attempt in 0..20 {
            let delay = provider.backoff_delay_ms(attempt);
            assert!(delay <= max_allowed, "attempt {attempt} delay {delay} 超过上限");
        }
    }

    #[test]
    fn test_backoff_delay_grows() {
        let cfg = RetryConfig::new()
            .with_max_retries(10)
            .with_base_delay_ms(100)
            .with_max_delay_ms(u64::MAX);
        let inner = crate::OpenAiProvider::with_model("https://example.com", "key", "model");
        let provider = ResilientProvider::new(Arc::new(inner));
        let provider = provider.with_config(cfg);
        let d0 = provider.backoff_delay_ms(0);
        let d1 = provider.backoff_delay_ms(1);
        let d2 = provider.backoff_delay_ms(2);
        assert!(d1 > d0);
        assert!(d2 > d1);
    }

    #[test]
    fn test_should_retry_retriable_error() {
        let cfg = RetryConfig::new();
        let inner = crate::OpenAiProvider::with_model("https://example.com", "key", "model");
        let provider = ResilientProvider::new(Arc::new(inner));
        let provider = provider.with_config(cfg);
        let err = ProviderError::network("连接失败");
        assert!(provider.should_retry(&err, 0));
    }

    #[test]
    fn test_should_retry_non_retriable_once() {
        let inner = crate::OpenAiProvider::with_model("https://example.com", "key", "model");
        let provider = ResilientProvider::new(Arc::new(inner));
        let mut cfg = RetryConfig::new();
        cfg.retry_non_retriable_once = true;
        let provider = provider.with_config(cfg);
        let err = ProviderError::Message("some error".to_string());
        assert!(provider.should_retry(&err, 0));
        assert!(!provider.should_retry(&err, 1));
    }

    #[test]
    fn test_should_retry_false_for_non_retriable() {
        let inner = crate::OpenAiProvider::with_model("https://example.com", "key", "model");
        let provider = ResilientProvider::new(Arc::new(inner));
        let provider = provider.with_config(RetryConfig::new());
        let err = ProviderError::Message("some error".to_string());
        assert!(!provider.should_retry(&err, 0));
    }

    #[test]
    fn test_retry_config_default() {
        let cfg = RetryConfig::default();
        assert_eq!(cfg.max_retries, 2);
        assert!(!cfg.retry_non_retriable_once);
    }
}
