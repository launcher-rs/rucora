//! SimpleAgent - 简单问答 Agent
//!
//! # 概述
//!
//! SimpleAgent 是最简单的 Agent 类型，一次 LLM 调用直接返回结果，无工具调用，无循环。
//!
//! # 适用场景
//!
//! - 简单问答
//! - 翻译
//! - 总结
//! - 一次性任务
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use agentkit::agent::SimpleAgent;
//! use agentkit::provider::OpenAiProvider;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = OpenAiProvider::from_env()?;
//!
//! let agent = SimpleAgent::builder()
//!     .provider(provider)
//!     .model("gpt-4o-mini")
//!     .system_prompt("你是一个翻译助手")
//!     .temperature(0.3)
//!     .build();
//!
//! let output = agent.run("把'Hello'翻译成中文").await?;
//! println!("{}", output.text().unwrap_or("无回复"));
//! # Ok(())
//! # }
//! ```

use agentkit_core::agent::{Agent, AgentContext, AgentDecision, AgentInput, AgentOutput};
use agentkit_core::provider::LlmProvider;
use agentkit_core::provider::types::ChatRequest;
use async_trait::async_trait;
use std::sync::Arc;

use crate::agent::execution::DefaultExecution;

/// SimpleAgent - 简单问答 Agent
///
/// 特点：
/// - 一次 LLM 调用直接返回结果
/// - 无工具调用
/// - 无循环
/// - 适合简单任务
pub struct SimpleAgent<P> {
    /// LLM Provider
    #[allow(dead_code)]
    provider: Arc<P>,
    /// 默认使用的模型
    #[allow(dead_code)]
    model: String,
    /// 系统提示词
    #[allow(dead_code)]
    system_prompt: Option<String>,
    /// 温度参数（控制随机性）
    #[allow(dead_code)]
    temperature: f32,
    /// 执行能力（内聚）
    execution: DefaultExecution,
}

#[async_trait]
impl<P> Agent for SimpleAgent<P>
where
    P: LlmProvider + Send + Sync + 'static,
{
    async fn think(&self, context: &AgentContext) -> AgentDecision {
        // 简单策略：直接让 LLM 回答，不调用工具
        AgentDecision::Chat {
            request: Box::new(ChatRequest {
                messages: context.messages.clone(),
                model: Some(self.model.clone()),
                temperature: Some(self.temperature),
                tools: None, // 不使用工具
                ..Default::default()
            }),
        }
    }

    fn name(&self) -> &str {
        "simple_agent"
    }

    fn description(&self) -> Option<&str> {
        Some("简单问答 Agent，一次调用直接返回结果")
    }

    /// 运行 Agent（覆盖默认实现，使用 DefaultExecution）
    async fn run(
        &self,
        input: AgentInput,
    ) -> Result<AgentOutput, agentkit_core::agent::AgentError> {
        self.execution.run(self, input).await
    }
}

impl<P> SimpleAgent<P> {
    /// 创建新的构建器
    pub fn builder() -> SimpleAgentBuilder<P> {
        SimpleAgentBuilder::new()
    }

    /// 获取 Provider 引用
    pub fn provider(&self) -> &P {
        &self.provider
    }

    /// 获取模型名称
    pub fn model(&self) -> &str {
        &self.model
    }
}

/// SimpleAgent 构建器
pub struct SimpleAgentBuilder<P> {
    provider: Option<P>,
    system_prompt: Option<String>,
    model: Option<String>,
    temperature: f32,
    middleware_chain: crate::middleware::MiddlewareChain,
}

impl<P> SimpleAgentBuilder<P> {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            provider: None,
            system_prompt: None,
            model: None,
            temperature: 0.7,
            middleware_chain: crate::middleware::MiddlewareChain::new(),
        }
    }
}

impl<P> SimpleAgentBuilder<P>
where
    P: LlmProvider + Send + Sync + 'static,
{
    /// 设置 Provider（必需）
    pub fn provider(mut self, provider: P) -> Self {
        self.provider = Some(provider);
        self
    }

    /// 设置系统提示词
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// 设置默认模型（必需）
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// 设置温度参数（控制随机性，0.0-1.0）
    ///
    /// - 较低值（0.2-0.5）：更确定、保守
    /// - 较高值（0.7-1.0）：更随机、创造性
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature.clamp(0.0, 1.0);
        self
    }

    /// 设置中间件链
    pub fn with_middleware_chain(
        mut self,
        middleware_chain: crate::middleware::MiddlewareChain,
    ) -> Self {
        self.middleware_chain = middleware_chain;
        self
    }

    /// 添加中间件
    pub fn with_middleware<M: crate::middleware::Middleware + 'static>(
        mut self,
        middleware: M,
    ) -> Self {
        self.middleware_chain = self.middleware_chain.with(middleware);
        self
    }

    /// 构建 Agent
    ///
    /// # Panics
    ///
    /// 如果没有设置 `provider` 或 `model`，此方法会 panic。
    pub fn build(self) -> SimpleAgent<P> {
        let provider = self.provider.expect("Provider is required");
        let model = self.model.expect("Model is required");

        // 创建执行能力（SimpleAgent 不使用工具）
        let provider_arc = Arc::new(provider);
        let execution = DefaultExecution::new(
            provider_arc.clone(),
            model.clone(),
            crate::agent::ToolRegistry::new(),
        )
        .with_system_prompt_opt(self.system_prompt.clone())
        .with_max_steps(1) // SimpleAgent 只需要 1 步
        .with_middleware_chain(self.middleware_chain);

        SimpleAgent {
            provider: provider_arc,
            model,
            system_prompt: self.system_prompt,
            temperature: self.temperature,
            execution,
        }
    }
}

impl<P> Default for SimpleAgentBuilder<P> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentkit_core::error::ProviderError;
    use agentkit_core::provider::types::{ChatResponse, ChatStreamChunk};
    use agentkit_core::provider::{ChatMessage, Role};
    use futures_util::stream;
    use futures_util::stream::BoxStream;

    struct MockProvider;

    #[async_trait]
    impl LlmProvider for MockProvider {
        async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
            Ok(ChatResponse {
                message: ChatMessage {
                    role: Role::Assistant,
                    content: "Mock response".to_string(),
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
        ) -> Result<BoxStream<'static, Result<ChatStreamChunk, ProviderError>>, ProviderError>
        {
            Ok(Box::pin(stream::empty()))
        }
    }

    #[test]
    fn test_simple_agent_builder() {
        let _agent = SimpleAgentBuilder::<MockProvider>::new()
            .provider(MockProvider)
            .model("gpt-4o-mini")
            .system_prompt("test")
            .temperature(0.5)
            .build();
    }
}
