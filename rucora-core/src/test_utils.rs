//! 共享测试工具——提供简单的 MockProvider 用于 Agent 单元测试

use crate::error::ProviderError;
use crate::provider::LlmProvider;
use crate::provider::types::{ChatMessage, ChatRequest, ChatResponse, ChatStreamChunk};
use async_trait::async_trait;
use futures_util::stream::BoxStream;

/// 极简 MockProvider，返回固定 "Mock response" 文本，无工具调用，无流式数据。
pub struct MockProvider;

#[async_trait]
impl LlmProvider for MockProvider {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        Ok(ChatResponse {
            message: ChatMessage::assistant("Mock response"),
            tool_calls: vec![],
            usage: None,
            finish_reason: None,
        })
    }

    fn stream_chat(
        &self,
        _request: ChatRequest,
    ) -> Result<BoxStream<'static, Result<ChatStreamChunk, ProviderError>>, ProviderError> {
        Ok(Box::pin(futures_util::stream::empty()))
    }
}
