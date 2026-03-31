//! AgentKit 自定义 Provider 示例
//!
//! 展示如何实现自定义 LLM Provider。
//!
//! ## 运行方法
//! ```bash
//! cargo run --example 10_custom_provider
//! ```
//!
//! ## 功能演示
//!
//! 1. **Mock Provider** - 模拟 LLM 响应
//! 2. **流式响应** - 实现流式聊天
//! 3. **错误处理** - 处理各种错误场景
//! 4. **集成使用** - 与 Agent 集成使用

use agentkit::agent::SimpleAgent;
use agentkit::prelude::{Agent};
use agentkit_core::error::ProviderError;
use agentkit_core::provider::{LlmProvider, types::*};
use async_trait::async_trait;
use futures_util::StreamExt;
use futures_util::stream::{self, BoxStream};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

// ═══════════════════════════════════════════════════════════
// 自定义 Mock Provider
// ═══════════════════════════════════════════════════════════

/// Mock Provider - 用于测试和演示
///
/// 模拟 LLM 响应，不实际调用 API
struct MockProvider {
    /// 模拟延迟（毫秒）
    latency_ms: u64,
    /// 是否模拟错误
    simulate_error: bool,
}

impl MockProvider {
    fn new() -> Self {
        Self {
            latency_ms: 100,
            simulate_error: false,
        }
    }

    fn with_latency(mut self, latency_ms: u64) -> Self {
        self.latency_ms = latency_ms;
        self
    }

    fn with_error_simulation(mut self) -> Self {
        self.simulate_error = true;
        self
    }
}

impl Default for MockProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LlmProvider for MockProvider {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        // 模拟网络延迟
        sleep(Duration::from_millis(self.latency_ms)).await;

        // 模拟错误
        if self.simulate_error {
            return Err(ProviderError::Message("模拟错误：API 调用失败".to_string()));
        }

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
            Ok(ChatStreamChunk {
                delta: Some("，这是我的".to_string()),
                tool_calls: vec![],
                usage: None,
                finish_reason: None,
            }),
            Ok(ChatStreamChunk {
                delta: Some(" 流式响应示例".to_string()),
                tool_calls: vec![],
                usage: None,
                finish_reason: Some(FinishReason::Stop),
            }),
        ];

        Ok(Box::pin(stream::iter(chunks)))
    }
}

// ═══════════════════════════════════════════════════════════
// 自定义 Echo Provider
// ═══════════════════════════════════════════════════════════

/// Echo Provider - 回显用户输入
///
/// 用于测试，简单回应用户输入
struct EchoProvider;

#[async_trait]
impl LlmProvider for EchoProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        // 获取最后一条用户消息
        let user_message = request
            .messages
            .iter()
            .rev()
            .find(|m| m.role == Role::User)
            .map(|m| m.content.clone())
            .unwrap_or_default();

        // 回显用户输入
        Ok(ChatResponse {
            message: ChatMessage {
                role: Role::Assistant,
                content: format!("Echo: {}", user_message),
                name: None,
            },
            tool_calls: vec![],
            usage: Some(Usage {
                prompt_tokens: user_message.len() as u32 / 4,
                completion_tokens: user_message.len() as u32 / 4,
                total_tokens: user_message.len() as u32 / 2,
            }),
            finish_reason: Some(FinishReason::Stop),
        })
    }

    fn stream_chat(
        &self,
        request: ChatRequest,
    ) -> Result<BoxStream<'static, Result<ChatStreamChunk, ProviderError>>, ProviderError> {
        let user_message = request
            .messages
            .iter()
            .rev()
            .find(|m| m.role == Role::User)
            .map(|m| m.content.clone())
            .unwrap_or_default();

        // 逐字返回
        let chunks: Vec<_> = user_message
            .chars()
            .map(|c| {
                Ok(ChatStreamChunk {
                    delta: Some(c.to_string()),
                    tool_calls: vec![],
                    usage: None,
                    finish_reason: None,
                })
            })
            .collect();

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

    // ═══════════════════════════════════════════════════════════
    // 示例 1: Mock Provider
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 1: Mock Provider");
    info!("═══════════════════════════════════════\n");

    info!("1.1 创建 Mock Provider...");
    let mock_provider = MockProvider::new().with_latency(50);
    info!("✓ Mock Provider 创建成功\n");

    info!("1.2 测试聊天功能...");
    let request = ChatRequest::from_user_text("你好");
    let response = mock_provider.chat(request).await?;

    info!("响应内容：{}", response.message.content);
    if let Some(usage) = response.usage {
        info!(
            "Token 使用：输入={}, 输出={}, 总计={}",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
        );
    }
    info!("");

    // ═══════════════════════════════════════════════════════════
    // 示例 2: 流式响应
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 2: 流式响应");
    info!("═══════════════════════════════════════\n");

    info!("2.1 测试流式聊天功能...");
    let request = ChatRequest::from_user_text("你好");
    let mut stream = mock_provider.stream_chat(request)?;

    info!("流式响应：");
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if let Some(delta) = chunk.delta {
            info!("  {}", delta);
        }
    }
    info!("");

    // ═══════════════════════════════════════════════════════════
    // 示例 3: 错误处理
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 3: 错误处理");
    info!("═══════════════════════════════════════\n");

    info!("3.1 创建带错误模拟的 Provider...");
    let error_provider = MockProvider::new().with_error_simulation();
    info!("✓ Provider 创建成功\n");

    info!("3.2 测试错误处理...");
    let request = ChatRequest::from_user_text("你好");
    match error_provider.chat(request).await {
        Ok(_) => info!("意外：请求成功了"),
        Err(e) => info!("预期错误：{}", e),
    }
    info!("");

    // ═══════════════════════════════════════════════════════════
    // 示例 4: Echo Provider
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 4: Echo Provider");
    info!("═══════════════════════════════════════\n");

    info!("4.1 创建 Echo Provider...");
    let echo_provider = EchoProvider;
    info!("✓ Echo Provider 创建成功\n");

    info!("4.2 测试 Echo 功能...");
    let request = ChatRequest::from_user_text("这是一个测试消息");
    let response = echo_provider.chat(request).await?;

    info!("输入：这是一个测试消息");
    info!("输出：{}\n", response.message.content);

    // ═══════════════════════════════════════════════════════════
    // 示例 5: 与 Agent 集成
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 5: 与 Agent 集成");
    info!("═══════════════════════════════════════\n");

    info!("5.1 使用 Echo Provider 创建 Agent...");
    let agent = SimpleAgent::builder()
        .provider(echo_provider)
        .model("echo-model")
        .system_prompt("你是一个回显助手，会重复用户的话。")
        .build();
    info!("✓ Agent 创建成功\n");

    info!("5.2 测试 Agent...");
    let test_input = "你好，Agent！";
    info!("输入：\"{}\"", test_input);

    match agent.run(test_input.into()).await {
        Ok(output) => {
            if let Some(text) = output.text() {
                info!("输出：\"{}\"", text);
            }
        }
        Err(e) => {
            info!("Agent 运行出错：{}", e);
        }
    }
    info!("");

    // ═══════════════════════════════════════════════════════════
    // 总结
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例完成！");
    info!("═══════════════════════════════════════\n");

    info!("📝 实现自定义 Provider 的步骤：\n");

    info!("1. 定义 Provider 结构体:");
    info!("   - 存储配置（API Key、Base URL 等）");
    info!("   - 存储客户端（reqwest::Client 等）\n");

    info!("2. 实现 LlmProvider trait:");
    info!("   - chat() - 非流式聊天");
    info!("   - stream_chat() - 流式聊天（可选）\n");

    info!("3. 处理请求和响应:");
    info!("   - 构建请求体");
    info!("   - 发送 HTTP 请求");
    info!("   - 解析响应");
    info!("   - 转换为 ChatResponse\n");

    info!("4. 错误处理:");
    info!("   - 网络错误");
    info!("   - API 错误");
    info!("   - 解析错误");
    info!("   - 转换为 ProviderError\n");

    info!("5. 集成使用:");
    info!("   - 传递给 Agent");
    info!("   - 配置默认模型");
    info!("   - 设置超时和重试\n");

    info!("💡 提示:");
    info!("   - Mock Provider 适合单元测试");
    info!("   - Echo Provider 适合调试");
    info!("   - 实际 Provider 需要处理认证和限流\n");

    Ok(())
}
