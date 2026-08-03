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
//! use rucora::agent::SimpleAgent;
//! use rucora::provider::OpenAiProvider;
//! use rucora::prelude::Agent;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = OpenAiProvider::from_env()?;
//!
//! let agent = SimpleAgent::builder(provider)
//!     .model("gpt-4o-mini")
//!     .system_prompt("你是一个翻译助手")
//!     .temperature(0.3)
//!     .build();
//!
//! let output = agent.run("把'Hello'翻译成中文".into()).await?;
//! println!("{}", output.text().unwrap_or("无回复"));
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use rucora_core::agent::{Agent, AgentContext, AgentDecision, AgentInput, AgentOutput};
use rucora_core::provider::LlmProvider;
use rucora_core::provider::types::LlmParams;
use std::sync::Arc;

use crate::agent::execution::{DefaultExecution, build_default_execution};

/// SimpleAgent - 简单问答 Agent
///
/// 特点：
/// - 一次 LLM 调用直接返回结果
/// - 无工具调用
/// - 无循环
/// - 适合简单任务
pub struct SimpleAgent<P> {
    /// LLM Provider
    provider: Arc<P>,
    /// Agent 级模型覆盖；为空时使用 Provider 默认模型。
    model: Option<String>,
    /// 系统提示词
    _system_prompt: Option<String>,
    /// LLM 请求参数
    llm_params: LlmParams,
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
        AgentDecision::chat({
            let mut request = context.default_chat_request_with(&self.llm_params);
            request.model = self.model.clone();
            request.tools = None; // 不使用工具
            request
        })
    }

    fn name(&self) -> &str {
        "simple_agent"
    }

    fn description(&self) -> Option<&str> {
        Some("简单问答 Agent，一次调用直接返回结果")
    }

    /// 运行 Agent（覆盖默认实现，使用 DefaultExecution）
    async fn run(&self, input: AgentInput) -> Result<AgentOutput, rucora_core::agent::AgentError> {
        self.execution.run(self, input).await
    }

    /// 流式运行
    fn run_stream(
        &self,
        input: AgentInput,
    ) -> futures_util::stream::BoxStream<
        'static,
        Result<rucora_core::channel::types::ChannelEvent, rucora_core::agent::AgentError>,
    > {
        self.execution.run_stream_simple(input)
    }
}

impl<P> SimpleAgent<P>
where
    P: LlmProvider + Send + Sync + 'static,
{
    /// 流式运行并返回拼接后的最终文本。
    pub async fn run_stream_text(
        &self,
        input: impl Into<AgentInput>,
    ) -> Result<String, rucora_core::agent::AgentError> {
        self.execution.run_stream_text(input.into()).await
    }
}

impl<P> SimpleAgent<P> {
    /// 创建新的构建器。
    ///
    /// # 参数
    ///
    /// - `provider`: LLM Provider（必需）
    #[must_use = "构建器必须调用 build() 来创建 Agent"]
    pub fn builder(provider: P) -> SimpleAgentBuilder<P> {
        SimpleAgentBuilder::new(provider)
    }

    /// 获取 Provider 引用
    pub fn provider(&self) -> &P {
        &self.provider
    }

    /// 获取模型名称
    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }
}

/// SimpleAgent 构建器
pub struct SimpleAgentBuilder<P> {
    provider: P,
    system_prompt: Option<String>,
    model: Option<String>,
    llm_params: LlmParams,
    middleware_chain: crate::middleware::MiddlewareChain,
}

impl<P> SimpleAgentBuilder<P> {
    /// 创建新的构建器。
    ///
    /// # 参数
    ///
    /// - `provider`: LLM Provider（必需）
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            system_prompt: None,
            model: None,
            llm_params: LlmParams::default(),
            middleware_chain: crate::middleware::MiddlewareChain::new(),
        }
    }
}

impl<P> SimpleAgentBuilder<P>
where
    P: LlmProvider + Send + Sync + 'static,
{
    /// 设置系统提示词
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// 设置 Agent 级模型覆盖。不设置时使用 Provider 默认模型。
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// 设置温度参数（控制随机性，0.0-1.0）
    ///
    /// - 较低值（0.2-0.5）：更确定、保守
    /// - 较高值（0.7-1.0）：更随机、创造性
    pub fn temperature(mut self, value: f32) -> Self {
        self.llm_params.temperature = Some(value);
        self
    }

    /// 设置 top_p
    pub fn top_p(mut self, value: f32) -> Self {
        self.llm_params.top_p = Some(value);
        self
    }

    /// 设置 top_k
    pub fn top_k(mut self, value: u32) -> Self {
        self.llm_params.top_k = Some(value);
        self
    }

    /// 设置 max_tokens
    pub fn max_tokens(mut self, value: u32) -> Self {
        self.llm_params.max_tokens = Some(value);
        self
    }

    /// 设置 frequency_penalty
    pub fn frequency_penalty(mut self, value: f32) -> Self {
        self.llm_params.frequency_penalty = Some(value);
        self
    }

    /// 设置 presence_penalty
    pub fn presence_penalty(mut self, value: f32) -> Self {
        self.llm_params.presence_penalty = Some(value);
        self
    }

    /// 设置 stop 序列
    pub fn stop(mut self, value: Vec<String>) -> Self {
        self.llm_params.stop = Some(value);
        self
    }

    /// 设置额外参数（provider 特定）
    pub fn extra_params(mut self, value: serde_json::Value) -> Self {
        self.llm_params.extra = Some(value);
        self
    }

    /// 设置 LLM 请求参数
    pub fn llm_params(mut self, params: LlmParams) -> Self {
        self.llm_params = params;
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

    /// 构建 Agent。
    pub fn build(self) -> SimpleAgent<P> {
        let provider = self.provider;
        // 创建执行能力（SimpleAgent 不使用工具）
        let provider_arc = Arc::new(provider);
        let execution = build_default_execution(crate::agent::ExecutionBuildConfig {
            provider: provider_arc.clone(),
            model: self.model.clone(),
            tools: crate::agent::ToolRegistry::new(),
            system_prompt: self.system_prompt.clone(),
            max_steps: 1, // SimpleAgent 只需要 1 步
            max_tool_concurrency: 1,
            conversation_manager: None,
            middleware_chain: self.middleware_chain.clone(),
            enhanced_config: crate::agent::tool_call_config::ToolCallEnhancedConfig::default(),
            llm_params: self.llm_params.clone(),
        });

        SimpleAgent {
            provider: provider_arc,
            model: self.model,
            _system_prompt: self.system_prompt,
            llm_params: self.llm_params,
            execution,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rucora_core::test_utils::MockProvider;

    #[test]
    fn test_simple_agent_builder() {
        let _agent = SimpleAgentBuilder::<MockProvider>::new(MockProvider)
            .model("gpt-4o-mini")
            .system_prompt("test")
            .temperature(0.5)
            .build();
    }
}
