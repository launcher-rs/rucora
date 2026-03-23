use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use agentkit_core::error::ProviderError;
use agentkit_core::provider::LlmProvider;
use agentkit_core::provider::types::ChatRequest;
use async_trait::async_trait;
use futures_util::{StreamExt, stream::BoxStream};
use tokio::time::{Duration, sleep, timeout};
use tracing::warn;

/// 错误类型分类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
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

impl ErrorCategory {
    /// 判断是否可重试
    pub fn is_retriable(&self) -> bool {
        matches!(
            self,
            ErrorCategory::Network
                | ErrorCategory::Timeout
                | ErrorCategory::RateLimit
                | ErrorCategory::Unavailable
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
            return ErrorCategory::Auth;
        }

        // 无效请求
        if lower.contains("invalid")
            || lower.contains("bad request")
            || lower.contains("400")
            || lower.contains("not found")
            || lower.contains("404")
        {
            return ErrorCategory::InvalidRequest;
        }

        // 限流
        if lower.contains("rate limit")
            || lower.contains("too many requests")
            || lower.contains("429")
        {
            return ErrorCategory::RateLimit;
        }

        // 超时
        if lower.contains("timeout") || lower.contains("timed out") {
            return ErrorCategory::Timeout;
        }

        // 网络错误
        if lower.contains("network")
            || lower.contains("connection")
            || lower.contains("dns")
            || lower.contains("socket")
            || lower.contains("reset")
            || lower.contains("unreachable")
        {
            return ErrorCategory::Network;
        }

        // 服务不可用
        if lower.contains("unavailable")
            || lower.contains("503")
            || lower.contains("502")
            || lower.contains("504")
        {
            return ErrorCategory::Unavailable;
        }

        ErrorCategory::Other
    }

    /// 从 HTTP 状态码分类错误
    pub fn from_status_code(status: u16) -> Self {
        match status {
            400 => ErrorCategory::InvalidRequest,
            401 | 403 => ErrorCategory::Auth,
            404 => ErrorCategory::InvalidRequest,
            429 => ErrorCategory::RateLimit,
            500 | 502 | 503 | 504 => ErrorCategory::Unavailable,
            _ => ErrorCategory::Other,
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
        let delay = self.cfg.base_delay_ms.saturating_mul(pow);

        // 添加 10% 的抖动
        let jitter = (delay / 10).max(1); // 确保 jitter 不为 0
        let jitter_offset = (attempt as u64 * jitter) % jitter;

        delay.min(self.cfg.max_delay_ms) + jitter_offset
    }

    /// 判断错误是否可重试
    fn should_retry(&self, error: &ProviderError, attempt: usize) -> bool {
        let msg = error.to_string();
        let category = ErrorCategory::from_error_message(&msg);

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
                    agentkit_core::provider::types::ChatStreamChunk,
                    agentkit_core::error::ProviderError,
                >,
            >,
        ),
        agentkit_core::error::ProviderError,
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
    ) -> Result<agentkit_core::provider::types::ChatResponse, agentkit_core::error::ProviderError>
    {
        let mut attempt = 0usize;

        loop {
            let fut = self.inner.chat(request.clone());

            let result = if let Some(ms) = self.cfg.timeout_ms {
                match timeout(Duration::from_millis(ms), fut).await {
                    Ok(r) => r,
                    Err(_) => Err(ProviderError::Message(format!(
                        "provider chat timeout after {}ms",
                        ms
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
                            category = ?ErrorCategory::from_error_message(&e.to_string()),
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
                        category = ?ErrorCategory::from_error_message(&e.to_string()),
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
                agentkit_core::provider::types::ChatStreamChunk,
                agentkit_core::error::ProviderError,
            >,
        >,
        agentkit_core::error::ProviderError,
    > {
        self.inner.stream_chat(request)
    }
}
