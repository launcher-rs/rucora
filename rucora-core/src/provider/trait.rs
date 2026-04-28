use async_trait::async_trait;
use futures_util::stream::BoxStream;

use crate::{
    error::ProviderError,
    provider::types::{ChatRequest, ChatResponse, ChatStreamChunk},
};

/// LLM 提供者抽象。
///
/// 该 trait 的目标：用统一的 `ChatRequest/ChatResponse` 描述“对话”能力，
/// 以便上层 runtime 可以替换不同的模型提供方而无需修改编排逻辑。
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// 一次性对话请求（非流式）。
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError>;

    /// 流式对话请求（可选能力）。
    ///
    /// 默认实现会返回”不支持”。具体 provider 如果支持流式输出，重写该方法即可。
    ///
    /// # Errors
    ///
    /// 当流式输出不被支持或创建流失败时返回 [`ProviderError`]。
    fn stream_chat(
        &self,
        _request: ChatRequest,
    ) -> Result<BoxStream<'static, Result<ChatStreamChunk, ProviderError>>, ProviderError> {
        Err(ProviderError::Message(
            "stream_chat not supported".to_string(),
        ))
    }
}
