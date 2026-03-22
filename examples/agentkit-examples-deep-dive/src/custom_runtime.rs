//! 自定义 Runtime 实现示例
//!
//! 本示例展示如何实现自定义的 Runtime

use agentkit::core::agent::types::{AgentInput, AgentOutput};
use agentkit::core::error::AgentError;
use agentkit::core::provider::types::{ChatMessage, ChatRequest};
use agentkit::core::provider::LlmProvider;
use agentkit::core::runtime::Runtime;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use tracing::info;

/// 简单 Runtime - 只执行单次对话，不执行工具
///
/// 用于演示最简单的 Runtime 实现
pub struct SimpleRuntime<P> {
    provider: Arc<P>,
    system_prompt: Option<String>,
}

impl<P: LlmProvider> SimpleRuntime<P> {
    /// 创建新的 Simple Runtime
    pub fn new(provider: Arc<P>) -> Self {
        Self {
            provider,
            system_prompt: None,
        }
    }

    /// 设置系统提示词
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }
}

#[async_trait]
impl<P: LlmProvider + Send + Sync> Runtime for SimpleRuntime<P> {
    async fn run(&self, input: AgentInput) -> Result<AgentOutput, AgentError> {
        info!("SimpleRuntime: 开始执行");

        // 将文本输入转换为消息
        let mut messages = vec![ChatMessage::user(input.text)];
        if let Some(prompt) = &self.system_prompt {
            messages.insert(0, ChatMessage::system(prompt.clone()));
        }

        // 调用 Provider
        let request = ChatRequest {
            messages,
            model: None,
            tools: None, // 简单 Runtime 不支持工具
            temperature: None,
            max_tokens: None,
            response_format: None,
            metadata: None,
        };

        info!("SimpleRuntime: 调用 Provider");
        let response = self
            .provider
            .chat(request)
            .await
            .map_err(|e| AgentError::Message(format!("Provider 错误：{}", e)))?;

        info!("SimpleRuntime: 执行完成");

        Ok(AgentOutput::with_history(
            json!({"content": response.message.content}),
            vec![response.message],
            Vec::new(),
        ))
    }
}

/// 带日志的 Runtime 装饰器
///
/// 包装另一个 Runtime，添加详细的日志记录
pub struct LoggingRuntime<R> {
    inner: R,
}

impl<R: Runtime> LoggingRuntime<R> {
    /// 创建新的 Logging Runtime
    pub fn new(inner: R) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<R: Runtime + Send + Sync> Runtime for LoggingRuntime<R> {
    async fn run(&self, input: AgentInput) -> Result<AgentOutput, AgentError> {
        info!("LoggingRuntime: 开始执行");
        info!("  - 输入文本：{}", input.text);

        let start = std::time::Instant::now();

        let result = self.inner.run(input).await;

        let elapsed = start.elapsed();
        match &result {
            Ok(output) => {
                info!(
                    "LoggingRuntime: 执行成功，耗时：{:?}，回复：{}",
                    elapsed, output.value
                );
            }
            Err(e) => {
                info!("LoggingRuntime: 执行失败：{}", e);
            }
        }

        result
    }
}

/// 带重试的 Runtime 装饰器
///
/// 包装另一个 Runtime，添加自动重试功能
pub struct RetryRuntime<R> {
    inner: R,
    max_retries: usize,
}

impl<R: Runtime> RetryRuntime<R> {
    /// 创建新的 Retry Runtime
    pub fn new(inner: R, max_retries: usize) -> Self {
        Self { inner, max_retries }
    }
}

#[async_trait]
impl<R: Runtime + Send + Sync> Runtime for RetryRuntime<R> {
    async fn run(&self, input: AgentInput) -> Result<AgentOutput, AgentError> {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            info!(
                "RetryRuntime: 尝试 {} / {}",
                attempt + 1,
                self.max_retries + 1
            );

            match self.inner.run(input.clone()).await {
                Ok(output) => {
                    info!("RetryRuntime: 成功");
                    return Ok(output);
                }
                Err(e) => {
                    last_error = Some(e);
                    info!("RetryRuntime: 失败，准备重试");
                    tokio::time::sleep(tokio::time::Duration::from_millis(100 * (attempt as u64)))
                        .await;
                }
            }
        }

        Err(last_error.unwrap())
    }
}

/// 运行示例
pub async fn run() -> anyhow::Result<()> {
    info!("=== 自定义 Runtime 示例 ===");

    // 使用 Mock Provider 创建 Simple Runtime
    use crate::custom_provider::MockProvider;

    let provider = Arc::new(MockProvider::new("你好！我是简单助手。"));

    let runtime = SimpleRuntime::new(provider.clone()).with_system_prompt("你是一个有用的助手");

    info!("✓ Simple Runtime 创建成功");

    // 测试 Simple Runtime
    let input = AgentInput::new("你好");

    let output = runtime.run(input).await?;
    info!("✓ Simple Runtime 回复：{}", output.value);

    // 测试 Logging Runtime 装饰器
    info!("\n--- Logging Runtime ---");
    let logging_runtime = LoggingRuntime::new(SimpleRuntime::new(provider.clone()));

    let input = AgentInput::new("测试日志");

    let _ = logging_runtime.run(input).await;

    // 测试 Retry Runtime 装饰器
    info!("\n--- Retry Runtime ---");
    let retry_runtime = RetryRuntime::new(SimpleRuntime::new(provider), 3);

    let input = AgentInput::new("测试重试");

    let _ = retry_runtime.run(input).await;

    Ok(())
}
