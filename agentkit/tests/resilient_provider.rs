use std::sync::{Arc, Mutex};

use agentkit::provider::{ResilientProvider, RetryConfig};
use agentkit_core::error::ProviderError;
use agentkit_core::provider::types::{ChatMessage, ChatRequest, ChatResponse, Role};
use agentkit_core::provider::LlmProvider;
use async_trait::async_trait;
use futures_util::stream::BoxStream;
use futures_util::StreamExt;

struct FlakyProvider {
    attempts: Mutex<u32>,
}

#[async_trait]
impl LlmProvider for FlakyProvider {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let mut a = self.attempts.lock().unwrap();
        *a += 1;
        if *a < 3 {
            // 使用明确的网络错误消息以便重试
            return Err(ProviderError::Message(
                "network connection reset".to_string(),
            ));
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

#[tokio::test]
async fn resilient_provider_should_retry_chat() {
    let inner = Arc::new(FlakyProvider {
        attempts: Mutex::new(0),
    });

    let rp = ResilientProvider::new(inner).with_config(RetryConfig {
        max_retries: 5,
        base_delay_ms: 1,
        max_delay_ms: 2,
        timeout_ms: None,
        retry_non_retriable_once: false,
    });

    let resp = rp
        .chat(ChatRequest {
            messages: vec![ChatMessage {
                role: Role::User,
                content: "hi".to_string(),
                name: None,
            }],
            model: None,
            tools: None,
            temperature: None,
            max_tokens: None,
            response_format: None,
            metadata: None,
        })
        .await
        .expect("chat should succeed after retries");

    assert_eq!(resp.message.content, "ok");
}

struct InfiniteStreamProvider;

#[async_trait]
impl LlmProvider for InfiniteStreamProvider {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        Err(ProviderError::Message("not used".to_string()))
    }

    fn stream_chat(
        &self,
        _request: ChatRequest,
    ) -> Result<
        BoxStream<'static, Result<agentkit_core::provider::types::ChatStreamChunk, ProviderError>>,
        ProviderError,
    > {
        let s = async_stream::try_stream! {
            loop {
                yield agentkit_core::provider::types::ChatStreamChunk {
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

#[tokio::test]
async fn resilient_provider_stream_should_be_cancellable() {
    let inner = Arc::new(InfiniteStreamProvider);
    let rp = ResilientProvider::new(inner);

    let (handle, mut stream) = rp
        .stream_chat_cancellable(ChatRequest {
            messages: vec![ChatMessage {
                role: Role::User,
                content: "hi".to_string(),
                name: None,
            }],
            model: None,
            tools: None,
            temperature: None,
            max_tokens: None,
            response_format: None,
            metadata: None,
        })
        .expect("stream should start");

    let first = stream.next().await.expect("should have first item");
    assert!(first.is_ok());

    handle.cancel();
    let next = stream.next().await;
    assert!(next.is_none());
}
