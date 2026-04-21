//! ChatAgent - 纯对话 Agent
//!
//! # 概述
//!
//! ChatAgent 专注于多轮对话，支持对话历史管理，但不使用工具。
//!
//! # 适用场景
//!
//! - 客服对话
//! - 心理咨询
//! - 闲聊
//! - 多轮问答
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use agentkit::agent::ChatAgent;
//! use agentkit::provider::OpenAiProvider;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = OpenAiProvider::from_env()?;
//!
//! let agent = ChatAgent::builder()
//!     .provider(provider)
//!     .model("gpt-4o-mini")
//!     .system_prompt("你是友好的心理咨询助手")
//!     .with_conversation(true)  // 启用对话历史
//!     .max_history_messages(20) // 保留最近 20 条消息
//!     .build();
//!
//! // 第一轮
//! agent.run("我今天心情不好").await?;
//!
//! // 第二轮（自动记住上一轮）
//! agent.run("因为工作压力大").await?;
//!
//! // 第三轮
//! agent.run("有什么建议吗？").await?;
//! # Ok(())
//! # }
//! ```

use agentkit_core::agent::{Agent, AgentContext, AgentDecision, AgentInput, AgentOutput};
use agentkit_core::provider::LlmProvider;
use agentkit_core::provider::types::{ChatMessage, LlmParams};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::agent::execution::DefaultExecution;
use crate::conversation::ConversationManager;

/// ChatAgent - 纯对话 Agent
///
/// 特点：
/// - 支持多轮对话历史
/// - 不使用工具
/// - 适合对话场景
pub struct ChatAgent<P> {
    /// LLM Provider
    provider: Arc<P>,
    /// 默认使用的模型
    model: String,
    /// 系统提示词
    system_prompt: Option<String>,
    /// LLM 请求参数
    llm_params: LlmParams,
    /// 对话管理器
    conversation_manager: Option<Arc<Mutex<ConversationManager>>>,
    /// 最大历史消息数
    max_history_messages: usize,
    /// 执行能力（内聚）
    execution: DefaultExecution,
}

#[async_trait]
impl<P> Agent for ChatAgent<P>
where
    P: LlmProvider + Send + Sync + 'static,
{
    async fn think(&self, context: &AgentContext) -> AgentDecision {
        // 对话策略：直接让 LLM 回答，不调用工具
        AgentDecision::Chat {
            request: Box::new({
                let mut request = context.default_chat_request_with(&self.llm_params);
                request.model = Some(self.model.clone());
                request.tools = None; // 不使用工具
                request
            }),
        }
    }

    fn name(&self) -> &str {
        "chat_agent"
    }

    fn description(&self) -> Option<&str> {
        Some("纯对话 Agent，支持多轮对话历史")
    }

    /// 运行 Agent（覆盖默认实现，使用 DefaultExecution）
    async fn run(
        &self,
        input: AgentInput,
    ) -> Result<AgentOutput, agentkit_core::agent::AgentError> {
        self.execution.run(self, input).await
    }
}

impl<P> ChatAgent<P> {
    /// 创建新的构建器
    pub fn builder() -> ChatAgentBuilder<P> {
        ChatAgentBuilder::new()
    }

    /// 获取 Provider 引用
    pub fn provider(&self) -> &P {
        &self.provider
    }

    /// 获取模型名称
    pub fn model(&self) -> &str {
        &self.model
    }

    /// 获取对话历史（如果启用了）
    pub async fn get_conversation_history(&self) -> Option<Vec<ChatMessage>> {
        match &self.conversation_manager {
            Some(conv_arc) => {
                let conv = conv_arc.lock().await;
                Some(conv.get_messages().to_vec())
            }
            None => None,
        }
    }

    /// 清空对话历史（如果启用了）
    pub async fn clear_conversation(&self) {
        if let Some(ref conv_arc) = self.conversation_manager {
            let mut conv = conv_arc.lock().await;
            conv.clear();
            // 重新添加系统提示
            if let Some(ref prompt) = self.system_prompt {
                conv.ensure_system_prompt(prompt);
            }
        }
    }
}

/// ChatAgent 构建器
pub struct ChatAgentBuilder<P> {
    provider: Option<P>,
    system_prompt: Option<String>,
    model: Option<String>,
    llm_params: LlmParams,
    with_conversation: bool,
    max_history_messages: usize,
    middleware_chain: crate::middleware::MiddlewareChain,
}

impl<P> ChatAgentBuilder<P> {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            provider: None,
            system_prompt: None,
            model: None,
            llm_params: LlmParams::default(),
            with_conversation: false,
            max_history_messages: 0, // 0 表示无限制
            middleware_chain: crate::middleware::MiddlewareChain::new(),
        }
    }
}

impl<P> ChatAgentBuilder<P>
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

    /// 启用对话历史管理
    ///
    /// # 参数
    ///
    /// - `enabled`: 是否启用对话历史
    pub fn with_conversation(mut self, enabled: bool) -> Self {
        self.with_conversation = enabled;
        self
    }

    /// 设置对话历史最大消息数（仅在启用对话时有效）
    ///
    /// # 参数
    ///
    /// - `max_messages`: 最大消息数（0 表示无限制）
    pub fn max_history_messages(mut self, max_messages: usize) -> Self {
        self.max_history_messages = max_messages;
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
    pub fn build(self) -> ChatAgent<P> {
        let provider = self.provider.expect("Provider is required");
        let model = self.model.expect("Model is required");

        // 创建对话管理器
        let conversation_manager = if self.with_conversation {
            let mut conv = ConversationManager::new();
            if let Some(ref prompt) = self.system_prompt {
                conv = conv.with_system_prompt(prompt.clone());
            }
            if self.max_history_messages > 0 {
                conv = conv.with_max_messages(self.max_history_messages);
            }
            Some(Arc::new(Mutex::new(conv)))
        } else {
            None
        };

        // 创建执行能力（ChatAgent 不使用工具）
        let provider_arc = Arc::new(provider);
        let execution = DefaultExecution::new(
            provider_arc.clone(),
            model.clone(),
            crate::agent::ToolRegistry::new(),
        )
        .with_system_prompt_opt(self.system_prompt.clone())
        .with_conversation_manager(conversation_manager.clone())
        .with_middleware_chain(self.middleware_chain)
        .with_llm_params(self.llm_params.clone());

        ChatAgent {
            provider: provider_arc,
            model,
            system_prompt: self.system_prompt,
            llm_params: self.llm_params,
            conversation_manager,
            max_history_messages: self.max_history_messages,
            execution,
        }
    }
}

impl<P> Default for ChatAgentBuilder<P> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentkit_core::error::ProviderError;
    use agentkit_core::provider::Role;
    use agentkit_core::provider::types::{ChatRequest, ChatResponse, ChatStreamChunk};
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
    fn test_chat_agent_builder() {
        let _agent = ChatAgentBuilder::<MockProvider>::new()
            .provider(MockProvider)
            .model("gpt-4o-mini")
            .system_prompt("test")
            .with_conversation(true)
            .max_history_messages(20)
            .build();
    }
}
