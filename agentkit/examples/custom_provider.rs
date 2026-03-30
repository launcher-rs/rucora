//! 自定义 Provider 实现示例
//!
//! 本示例展示如何实现自定义的 LLM Provider

use agentkit_core::error::ProviderError;
use agentkit_core::provider::LlmProvider;
use agentkit_core::provider::types::{
    ChatMessage, ChatRequest, ChatResponse, ChatStreamChunk, Role,
};
use async_trait::async_trait;
use futures_util::StreamExt;
use futures_util::stream::{self, BoxStream};
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

/// 模拟 Provider - 用于测试和演示
///
/// 这个 Provider 不实际调用 API，而是返回预设的响应
pub struct MockProvider {
    default_model: String,
    response_text: String,
}

impl MockProvider {
    /// 创建新的 Mock Provider
    pub fn new(response_text: impl Into<String>) -> Self {
        Self {
            default_model: "mock-model".to_string(),
            response_text: response_text.into(),
        }
    }

    /// 设置默认模型
    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }
}

#[async_trait]
impl LlmProvider for MockProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        info!("MockProvider: 收到聊天请求");
        info!("  - 消息数：{}", request.messages.len());
        info!("  - 模型：{:?}", request.model);
        info!("  - 工具数：{:?}", request.tools.as_ref().map(|t| t.len()));

        // 模拟处理延迟
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // 返回预设响应
        Ok(ChatResponse {
            message: ChatMessage {
                role: Role::Assistant,
                content: self.response_text.clone(),
                name: None,
            },
            tool_calls: vec![],
            usage: None,
            finish_reason: None,
        })
    }

    fn stream_chat(
        &self,
        _request: ChatRequest,
    ) -> Result<BoxStream<'static, Result<ChatStreamChunk, ProviderError>>, ProviderError> {
        info!("MockProvider: 收到流式聊天请求");

        // 模拟流式输出
        let text = self.response_text.clone();
        let chars: Vec<char> = text.chars().collect();

        let stream = stream::unfold(0, move |index| {
            let chars = chars.clone();
            async move {
                if index < chars.len() {
                    let chunk = ChatStreamChunk {
                        delta: Some(chars[index].to_string()),
                        tool_calls: vec![],
                        usage: None,
                        finish_reason: None,
                    };
                    Some((Ok(chunk), index + 1))
                } else {
                    None
                }
            }
        });

        Ok(Box::pin(stream))
    }
}

/// 带延迟的 Provider - 模拟网络延迟
pub struct DelayedProvider {
    inner: MockProvider,
    delay_ms: u64,
}

impl DelayedProvider {
    pub fn new(inner: MockProvider, delay_ms: u64) -> Self {
        Self { inner, delay_ms }
    }
}

#[async_trait]
impl LlmProvider for DelayedProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        info!("DelayedProvider: 延迟 {}ms", self.delay_ms);
        tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
        self.inner.chat(request).await
    }

    fn stream_chat(
        &self,
        request: ChatRequest,
    ) -> Result<BoxStream<'static, Result<ChatStreamChunk, ProviderError>>, ProviderError> {
        self.inner.stream_chat(request)
    }
}

/// 运行示例
#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("=== 自定义 Provider 示例 ===");

    // 创建 Mock Provider
    let provider =
        MockProvider::new("你好！我是一个模拟的 AI 助手。").with_default_model("mock-1.0");

    info!("✓ Mock Provider 创建成功");

    // 测试非流式聊天
    let request = ChatRequest {
        messages: vec![ChatMessage {
            role: Role::User,
            content: "你好".to_string(),
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
    };

    let response = provider.chat(request.clone()).await?;
    info!("✓ 非流式聊天成功：{}", response.message.content);

    // 测试流式聊天
    info!("\n--- 流式输出 ---");
    let request_stream = ChatRequest {
        messages: vec![ChatMessage {
            role: Role::User,
            content: "请流式输出".to_string(),
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
    };

    let mut stream = provider.stream_chat(request_stream)?;
    let mut content = String::new();
    while let Some(chunk) = stream.next().await {
        if let Ok(chunk) = chunk {
            if let Some(delta) = &chunk.delta {
                content.push_str(delta);
                print!("{}", delta);
            }
        }
    }
    println!();
    info!("✓ 流式聊天成功：{}", content);

    // 测试 Delayed Provider
    info!("\n--- Delayed Provider ---");
    let delayed = DelayedProvider::new(MockProvider::new("延迟响应"), 500);

    let start = std::time::Instant::now();
    let _ = delayed.chat(request).await?;
    let elapsed = start.elapsed();
    info!("✓ Delayed Provider 响应时间：{:?}", elapsed);

    Ok(())
}
