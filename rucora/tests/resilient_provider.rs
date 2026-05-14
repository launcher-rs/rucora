use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use futures_util::StreamExt;
use futures_util::stream::BoxStream;
use rucora::provider::{ResilientProvider, RetryConfig};
use rucora_core::error::ProviderError;
use rucora_core::provider::LlmProvider;
use rucora_core::provider::types::{ChatMessage, ChatRequest, ChatResponse, Role};

// ====== 基础工具 ======

/// 构建一个 ChatRequest 的便捷函数，减少测试中的重复代码
fn make_request(content: &str) -> ChatRequest {
    ChatRequest {
        messages: vec![ChatMessage {
            role: Role::User,
            content: content.to_string(),
            name: None,
        }],
        model: None,
        tools: None,
        temperature: None,
        max_tokens: None,
        response_format: None,
        metadata: None,
        top_p: None,
        top_k: None,
        frequency_penalty: None,
        presence_penalty: None,
        stop: None,
        extra: None,
    }
}

// ====== 测试用 Provider ======

/// 失败 N 次后成功的 Provider
struct FlakyProvider {
    attempts: Mutex<u32>,
    fail_count: u32,
    error_msg: String,
}

impl FlakyProvider {
    fn new(fail_count: u32) -> Self {
        Self {
            attempts: Mutex::new(0),
            fail_count,
            error_msg: "network connection reset".to_string(),
        }
    }
}

#[async_trait]
impl LlmProvider for FlakyProvider {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let mut a = self.attempts.lock().unwrap();
        *a += 1;
        if *a <= self.fail_count {
            return Err(ProviderError::Message(self.error_msg.clone()));
        }
        Ok(ChatResponse {
            message: ChatMessage {
                role: Role::Assistant,
                content: "ok".to_string(),
                name: None,
            },
            tool_calls: vec![],
            usage: None,
            finish_reason: None,
        })
    }
}

/// 始终返回指定错误的 Provider
struct AlwaysFailProvider {
    error_msg: String,
    call_count: AtomicUsize,
}

impl AlwaysFailProvider {
    fn new(error_msg: String) -> Self {
        Self {
            error_msg,
            call_count: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl LlmProvider for AlwaysFailProvider {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        Err(ProviderError::Message(self.error_msg.clone()))
    }
}

/// 无限流 Provider（用于取消测试）
struct InfiniteStreamProvider;

#[async_trait]
impl LlmProvider for InfiniteStreamProvider {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        Err(ProviderError::Message("not used".to_string()))
    }

    fn stream_chat(
        &self,
        _request: ChatRequest,
    ) -> Result<BoxStream<'static, Result<rucora_core::provider::types::ChatStreamChunk, ProviderError>>, ProviderError>
    {
        let s = async_stream::try_stream! {
            loop {
                yield rucora_core::provider::types::ChatStreamChunk {
                    delta: Some("x".to_string()),
                    tool_calls: vec![],
                    usage: None,
                    finish_reason: None,
                };
            }
        };
        Ok(Box::pin(s))
    }
}

// ====== 重试逻辑测试 ======

#[tokio::test]
async fn resilient_provider_should_retry_chat() {
    let inner = Arc::new(FlakyProvider::new(2));

    let rp = ResilientProvider::new(inner.clone()).with_config(RetryConfig {
        max_retries: 5,
        base_delay_ms: 1,
        max_delay_ms: 2,
        timeout_ms: None,
        retry_non_retriable_once: false,
    });

    let resp = rp.chat(make_request("hi")).await.expect("chat should succeed after retries");
    assert_eq!(resp.message.content, "ok");
    // 验证确实进行了重试（失败 2 次 + 第 3 次成功 = 共 3 次调用）
    let attempts = inner.attempts.lock().unwrap();
    assert_eq!(*attempts, 3);
}

#[tokio::test]
async fn resilient_provider_exhausts_retries() {
    // "network connection reset" 是可重试的网络错误
    let inner = Arc::new(AlwaysFailProvider::new("network connection reset".to_string()));

    let rp = ResilientProvider::new(inner.clone()).with_config(RetryConfig {
        max_retries: 2,
        base_delay_ms: 1,
        max_delay_ms: 2,
        timeout_ms: None,
        retry_non_retriable_once: false,
    });

    let result = rp.chat(make_request("test")).await;
    assert!(result.is_err());
    // 首次调用 + 2 次重试 = 共 3 次调用
    assert_eq!(inner.call_count.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn resilient_provider_zero_retries() {
    let inner = Arc::new(FlakyProvider::new(2));

    let rp = ResilientProvider::new(inner.clone()).with_config(RetryConfig {
        max_retries: 0,
        base_delay_ms: 1,
        max_delay_ms: 2,
        timeout_ms: None,
        retry_non_retriable_once: false,
    });

    let result = rp.chat(make_request("test")).await;
    // 零重试意味着首次失败后立即返回
    assert!(result.is_err());
    assert_eq!(inner.attempts.lock().unwrap().to_owned(), 1);
}

// ====== 可重试错误分类测试 ======

#[tokio::test]
async fn resilient_provider_retries_network_error() {
    let inner = Arc::new(AlwaysFailProvider::new("network connection reset".to_string()));

    let rp = ResilientProvider::new(inner.clone()).with_config(RetryConfig {
        max_retries: 3,
        base_delay_ms: 1,
        max_delay_ms: 2,
        timeout_ms: None,
        retry_non_retriable_once: false,
    });

    let result = rp.chat(make_request("test")).await;
    assert!(result.is_err(), "network errors should be retried");
    // 应该进行了 4 次调用（1 + 3 次重试）
    assert_eq!(inner.call_count.load(Ordering::SeqCst), 4);
}

#[tokio::test]
async fn resilient_provider_retries_timeout_error() {
    let inner = Arc::new(AlwaysFailProvider::new("request timed out".to_string()));

    let rp = ResilientProvider::new(inner.clone()).with_config(RetryConfig {
        max_retries: 2,
        base_delay_ms: 1,
        max_delay_ms: 2,
        timeout_ms: None,
        retry_non_retriable_once: false,
    });

    let result = rp.chat(make_request("test")).await;
    assert!(result.is_err(), "timeout errors should be retried");
    assert_eq!(inner.call_count.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn resilient_provider_retries_rate_limit() {
    let inner = Arc::new(AlwaysFailProvider::new("rate limit exceeded, too many requests".to_string()));

    let rp = ResilientProvider::new(inner.clone()).with_config(RetryConfig {
        max_retries: 2,
        base_delay_ms: 1,
        max_delay_ms: 2,
        timeout_ms: None,
        retry_non_retriable_once: false,
    });

    let result = rp.chat(make_request("test")).await;
    assert!(result.is_err(), "rate limit errors should be retried");
    assert_eq!(inner.call_count.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn resilient_provider_retries_503_error() {
    let inner = Arc::new(AlwaysFailProvider::new("503 Service Unavailable".to_string()));

    let rp = ResilientProvider::new(inner.clone()).with_config(RetryConfig {
        max_retries: 2,
        base_delay_ms: 1,
        max_delay_ms: 2,
        timeout_ms: None,
        retry_non_retriable_once: false,
    });

    let result = rp.chat(make_request("test")).await;
    assert!(result.is_err(), "503 errors should be retried");
    assert_eq!(inner.call_count.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn resilient_provider_does_not_retry_auth_error() {
    let inner = Arc::new(AlwaysFailProvider::new("401 Unauthorized: invalid API key".to_string()));

    let rp = ResilientProvider::new(inner.clone()).with_config(RetryConfig {
        max_retries: 3,
        base_delay_ms: 1,
        max_delay_ms: 2,
        timeout_ms: None,
        retry_non_retriable_once: false,
    });

    let result = rp.chat(make_request("test")).await;
    assert!(result.is_err(), "auth errors should fail immediately without retry");
    assert_eq!(inner.call_count.load(Ordering::SeqCst), 1, "auth errors should NOT be retried");
}

#[tokio::test]
async fn resilient_provider_does_not_retry_not_found() {
    let inner = Arc::new(AlwaysFailProvider::new("404 Not Found".to_string()));

    let rp = ResilientProvider::new(inner.clone()).with_config(RetryConfig {
        max_retries: 3,
        base_delay_ms: 1,
        max_delay_ms: 2,
        timeout_ms: None,
        retry_non_retriable_once: false,
    });

    let result = rp.chat(make_request("test")).await;
    assert!(result.is_err(), "404 errors should not be retried");
    assert_eq!(inner.call_count.load(Ordering::SeqCst), 1, "404 should NOT be retried");
}

#[tokio::test]
async fn resilient_provider_retry_non_retriable_once_enabled() {
    // 当 retry_non_retriable_once = true 时，即使是不可重试的错误也会尝试一次
    let inner = Arc::new(AlwaysFailProvider::new("some random error".to_string()));

    let rp = ResilientProvider::new(inner.clone()).with_config(RetryConfig {
        max_retries: 2,
        base_delay_ms: 1,
        max_delay_ms: 2,
        timeout_ms: None,
        retry_non_retriable_once: true,
    });

    let result = rp.chat(make_request("test")).await;
    assert!(result.is_err());
    // 非重试错误 + retry_non_retriable_once = true => 仍然尝试了一次
    assert!(inner.call_count.load(Ordering::SeqCst) >= 1);
}

// ====== 超时配置测试 ======

#[tokio::test]
async fn resilient_provider_with_timeout_config() {
    let inner = Arc::new(FlakyProvider::new(0));

    let rp = ResilientProvider::new(inner).with_config(RetryConfig {
        max_retries: 2,
        base_delay_ms: 1,
        max_delay_ms: 2,
        timeout_ms: Some(5000),
        retry_non_retriable_once: false,
    });

    let resp = rp.chat(make_request("hi")).await.expect("chat should succeed");
    assert_eq!(resp.message.content, "ok");
}

// ====== 指数退避测试 ======

#[tokio::test]
async fn resilient_provider_backoff_increases_delay() {
    let inner = Arc::new(AlwaysFailProvider::new("network error".to_string()));

    let rp = ResilientProvider::new(inner.clone()).with_config(RetryConfig {
        max_retries: 3,
        base_delay_ms: 1,
        max_delay_ms: 2,
        timeout_ms: None,
        retry_non_retriable_once: false,
    });

    let start = std::time::Instant::now();
    let _result = rp.chat(make_request("test")).await;
    let elapsed = start.elapsed();

    // With retries, the total time should be at least the number of retries * min delay
    // (there is jitter so we don't check exact values)
    assert!(elapsed >= Duration::from_millis(3), "backoff should introduce delays: elapsed={:?}", elapsed);
}

// ====== 流式取消测试 ======

#[tokio::test]
async fn resilient_provider_stream_should_be_cancellable() {
    let inner = Arc::new(InfiniteStreamProvider);
    let rp = ResilientProvider::new(inner);

    let (handle, mut stream) = rp
        .stream_chat_cancellable(make_request("hi"))
        .expect("stream should start");

    let first = stream.next().await.expect("should have first item");
    assert!(first.is_ok());

    handle.cancel();
    let next = stream.next().await;
    assert!(next.is_none(), "stream should end after cancel");
}

// ====== 不可重试的 ProviderError 类型测试 ======

#[tokio::test]
async fn resilient_provider_retries_provider_rate_limit_variant() {
    use rucora_core::error::ProviderError;

    struct RateLimitProvider {
        attempts: Mutex<u32>,
    }

    #[async_trait]
    impl LlmProvider for RateLimitProvider {
        async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
            let mut a = self.attempts.lock().unwrap();
            *a += 1;
            Err(ProviderError::RateLimit {
                message: "Too many requests".to_string(),
                retry_after: Some(Duration::from_secs(60)),
            })
        }
    }

    let inner = Arc::new(RateLimitProvider {
        attempts: Mutex::new(0),
    });

    let rp = ResilientProvider::new(inner.clone()).with_config(RetryConfig {
        max_retries: 2,
        base_delay_ms: 1,
        max_delay_ms: 2,
        timeout_ms: None,
        retry_non_retriable_once: false,
    });

    let result = rp.chat(make_request("test")).await;
    // RateLimit 应该被重试
    assert!(result.is_err());
    // 所有重试都应该被执行（RateLimit is retriable）
    assert_eq!(inner.attempts.lock().unwrap().to_owned(), 3);
}

#[tokio::test]
async fn resilient_provider_does_not_retry_provider_authentication_variant() {
    use rucora_core::error::ProviderError;

    struct AuthFailProvider {
        call_count: AtomicUsize,
    }

    #[async_trait]
    impl LlmProvider for AuthFailProvider {
        async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Err(ProviderError::Authentication {
                message: "Invalid API key".to_string(),
            })
        }
    }

    let inner = Arc::new(AuthFailProvider {
        call_count: AtomicUsize::new(0),
    });

    let rp = ResilientProvider::new(inner.clone()).with_config(RetryConfig {
        max_retries: 3,
        base_delay_ms: 1,
        max_delay_ms: 2,
        timeout_ms: None,
        retry_non_retriable_once: false,
    });

    let result = rp.chat(make_request("test")).await;
    // 认证错误应该不被重试
    assert!(result.is_err());
    assert_eq!(inner.call_count.load(Ordering::SeqCst), 1);
}

// ====== ResilientProvider 构造器测试 ======

#[test]
fn test_resilient_provider_creation() {
    let inner = Arc::new(AlwaysFailProvider::new("test".to_string()));
    let _rp = ResilientProvider::new(inner);
    // 使用默认配置创建成功
    let config = RetryConfig::default();
    assert_eq!(config.max_retries, 2);
}

#[test]
fn test_retry_config_builder() {
    let config = RetryConfig::new()
        .with_max_retries(5)
        .with_base_delay_ms(10)
        .with_max_delay_ms(500)
        .with_timeout_ms(3000);

    assert_eq!(config.max_retries, 5);
    assert_eq!(config.base_delay_ms, 10);
    assert_eq!(config.max_delay_ms, 500);
    assert_eq!(config.timeout_ms, Some(3000));
}