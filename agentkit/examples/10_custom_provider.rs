//! AgentKit 自定义 Provider 示例
//!
//! 展示如何实现自定义 LLM Provider。
//!
//! ## 运行方法
//! ```bash
//! cargo run --example 10_custom_provider
//! ```

use agentkit_core::error::ProviderError;
use agentkit_core::provider::{LlmProvider, types::*};
use async_trait::async_trait;
use futures_util::StreamExt;
use futures_util::stream::{self, BoxStream};
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

// 自定义 Mock Provider
struct MockProvider;

#[async_trait]
impl LlmProvider for MockProvider {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        // 模拟 LLM 响应
        Ok(ChatResponse {
            message: ChatMessage {
                role: Role::Assistant,
                content: "你好！我是 Mock Provider，这是我的模拟响应。".to_string(),
                name: None,
            },
            tool_calls: vec![],
            usage: Some(Usage {
                prompt_tokens: 10,
                completion_tokens: 20,
                total_tokens: 30,
            }),
            finish_reason: Some(FinishReason::Stop),
        })
    }

    fn stream_chat(
        &self,
        _request: ChatRequest,
    ) -> Result<BoxStream<'static, Result<ChatStreamChunk, ProviderError>>, ProviderError> {
        // 模拟流式响应
        let chunks = vec![
            Ok(ChatStreamChunk {
                delta: Some("你好".to_string()),
                tool_calls: vec![],
                usage: None,
                finish_reason: None,
            }),
            Ok(ChatStreamChunk {
                delta: Some("！我是".to_string()),
                tool_calls: vec![],
                usage: None,
                finish_reason: None,
            }),
            Ok(ChatStreamChunk {
                delta: Some(" Mock Provider".to_string()),
                tool_calls: vec![],
                usage: None,
                finish_reason: None,
            }),
        ];
        Ok(Box::pin(stream::iter(chunks)))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔════════════════════════════════════════╗");
    info!("║   AgentKit 自定义 Provider 示例       ║");
    info!("╚════════════════════════════════════════╝\n");

    info!("1. 创建自定义 Provider...\n");

    let provider = MockProvider;

    info!("✓ Provider 创建成功\n");

    info!("2. 测试聊天功能...\n");

    let request = ChatRequest {
        messages: vec![ChatMessage::user("你好")],
        model: Some("mock-model".to_string()),
        tools: None,
        temperature: Some(0.7),
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

    let response = provider.chat(request).await?;

    info!("响应内容：{}", response.message.content);
    if let Some(usage) = response.usage {
        info!(
            "Token 使用：输入={}, 输出={}, 总计={}",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
        );
    }

    info!("\n3. 测试流式聊天功能...\n");

    let request = ChatRequest {
        messages: vec![ChatMessage::user("你好")],
        model: Some("mock-model".to_string()),
        tools: None,
        temperature: Some(0.7),
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

    let mut stream = provider.stream_chat(request)?;

    info!("流式响应：");
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if let Some(delta) = chunk.delta {
            info!("  {}", delta);
        }
    }

    info!("\n═══════════════════════════════════════");
    info!("实现自定义 Provider 的步骤:");
    info!("═══════════════════════════════════════");
    info!("1. 实现 LlmProvider trait");
    info!("2. 实现 chat() 方法（非流式）");
    info!("3. 实现 stream_chat() 方法（流式）");
    info!("4. 处理错误和超时");
    info!("5. 集成到 Agent 中使用");
    info!("═══════════════════════════════════════\n");

    info!("示例完成！");

    Ok(())
}
