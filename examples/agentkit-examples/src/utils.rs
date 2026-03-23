//! 示例工具模块
//!
//! 提供示例中常用的共享工具，如 Mock Provider 等

use async_trait::async_trait;
use futures_util::stream;
use std::sync::Arc;

/// Mock Provider - 用于示例和测试，不需要 API Key
pub struct MockProvider {
    /// 默认回复内容
    pub default_response: String,
}

impl MockProvider {
    /// 创建新的 Mock Provider
    pub fn new() -> Self {
        Self {
            default_response: "这是一个模拟回复。设置 OPENAI_API_KEY 环境变量可使用真实 AI 服务。"
                .to_string(),
        }
    }

    /// 创建带自定义回复的 Mock Provider
    #[allow(dead_code)]
    pub fn with_response(response: impl Into<String>) -> Self {
        Self {
            default_response: response.into(),
        }
    }
}

impl Default for MockProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl agentkit::core::provider::LlmProvider for MockProvider {
    async fn chat(
        &self,
        _request: agentkit::core::provider::types::ChatRequest,
    ) -> Result<agentkit::core::provider::types::ChatResponse, agentkit::core::error::ProviderError>
    {
        Ok(agentkit::core::provider::types::ChatResponse {
            message: agentkit::core::provider::types::ChatMessage {
                role: agentkit::core::provider::types::Role::Assistant,
                content: self.default_response.clone(),
                name: None,
            },
            tool_calls: vec![],
            usage: None,
            finish_reason: None,
        })
    }

    fn stream_chat(
        &self,
        _request: agentkit::core::provider::types::ChatRequest,
    ) -> Result<
        futures_util::stream::BoxStream<
            'static,
            Result<
                agentkit::core::provider::types::ChatStreamChunk,
                agentkit::core::error::ProviderError,
            >,
        >,
        agentkit::core::error::ProviderError,
    > {
        let text = self.default_response.clone();
        let chars: Vec<char> = text.chars().collect();
        let stream = stream::unfold(0, move |index| {
            let chars = chars.clone();
            async move {
                if index < chars.len() {
                    Some((
                        Ok(agentkit::core::provider::types::ChatStreamChunk {
                            delta: Some(chars[index].to_string()),
                            tool_calls: vec![],
                            usage: None,
                            finish_reason: None,
                        }),
                        index + 1,
                    ))
                } else {
                    None
                }
            }
        });
        Ok(Box::pin(stream))
    }
}

/// 尝试从环境变量创建 Provider，失败则返回 Mock Provider
#[allow(dead_code)]
pub fn create_provider_or_mock() -> Arc<dyn agentkit::core::provider::LlmProvider> {
    use agentkit::provider::OpenAiProvider;

    match OpenAiProvider::from_env() {
        Ok(p) => {
            tracing::info!("✓ 使用 OpenAI Provider");
            Arc::new(p)
        }
        Err(_) => {
            tracing::info!("⚠ 未设置 OPENAI_API_KEY，使用 Mock Provider");
            Arc::new(MockProvider::new())
        }
    }
}
