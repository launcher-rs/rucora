use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use agentkit_core::provider::LlmProvider;
use agentkit_core::provider::types::ChatRequest;
use async_trait::async_trait;
use futures_util::{StreamExt, stream::BoxStream};
use tokio::time::{Duration, sleep, timeout};
use tracing::warn;

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: usize,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub timeout_ms: Option<u64>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 2,
            base_delay_ms: 200,
            max_delay_ms: 2_000,
            timeout_ms: None,
        }
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

    fn backoff_delay_ms(&self, attempt: usize) -> u64 {
        let pow = 1u64.checked_shl(attempt.min(16) as u32).unwrap_or(u64::MAX);
        let delay = self.cfg.base_delay_ms.saturating_mul(pow);
        delay.min(self.cfg.max_delay_ms)
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
                timeout(Duration::from_millis(ms), fut).await.map_err(|_| {
                    agentkit_core::error::ProviderError::Message(format!(
                        "provider chat timeout after {}ms",
                        ms
                    ))
                })?
            } else {
                fut.await
            };

            match result {
                Ok(v) => return Ok(v),
                Err(e) => {
                    if attempt >= self.cfg.max_retries {
                        return Err(e);
                    }
                    let delay = self.backoff_delay_ms(attempt);
                    warn!(attempt, delay_ms = delay, error = %e, "provider.chat.retry");
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
