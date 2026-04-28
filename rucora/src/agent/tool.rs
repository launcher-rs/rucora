//! ToolAgent - 工具调用 Agent
//!
//! # 概述
//!
//! ToolAgent 是支持工具调用的 Agent 类型，会自动决定何时调用工具。
//! 这是当前 `DefaultAgent` 的定位，但重构后更清晰地分离了决策和执行。
//!
//! # 适用场景
//!
//! - 执行具体任务（查询天气、执行命令、读写文件等）
//! - 需要多步推理的任务
//! - 需要访问外部资源的任务
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use rucora::agent::ToolAgent;
//! use rucora::provider::OpenAiProvider;
//! use rucora::tools::{ShellTool, FileReadTool};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = OpenAiProvider::from_env()?;
//!
//! let agent = ToolAgent::builder()
//!     .provider(provider)
//!     .model("gpt-4o-mini")
//!     .system_prompt("你是有用的助手")
//!     .tool(ShellTool)
//!     .tool(FileReadTool)
//!     .max_steps(10)
//!     .max_tool_concurrency(3)
//!     .build();
//!
//! let output = agent.run("帮我列出当前目录的文件").await?;
//! println!("{}", output.text().unwrap_or("无回复"));
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use rucora_core::agent::{Agent, AgentContext, AgentDecision, AgentInput, AgentOutput};
use rucora_core::provider::LlmProvider;
use rucora_core::provider::types::{ChatMessage, ChatRequest, LlmParams, Role};
use rucora_core::tool::Tool;
use rucora_core::tool::types::ToolDefinition;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::agent::ToolRegistry;
use crate::agent::execution::DefaultExecution;
use crate::agent::tool_call_config::ToolCallEnhancedConfig;
use crate::conversation::ConversationManager;

/// ToolAgent - 工具调用 Agent
///
/// 特点：
/// - 支持工具调用循环
/// - 自动决定何时调用工具
/// - 支持并发工具执行
/// - 支持工具策略
pub struct ToolAgent<P> {
    /// LLM Provider（用于外部访问）
    #[allow(dead_code)]
    provider: Arc<P>,
    /// 默认使用的模型
    #[allow(dead_code)]
    model: String,
    /// 系统提示词
    #[allow(dead_code)]
    system_prompt: Option<String>,
    /// 工具注册表
    #[allow(dead_code)]
    tools: ToolRegistry,
    /// 最大步骤数
    #[allow(dead_code)]
    max_steps: usize,
    /// 对话管理器（可选）
    #[allow(dead_code)]
    conversation_manager: Option<Arc<Mutex<ConversationManager>>>,
    /// LLM 请求参数
    llm_params: LlmParams,
    /// 执行能力（内聚）
    execution: DefaultExecution,
}

#[async_trait]
impl<P> Agent for ToolAgent<P>
where
    P: LlmProvider + Send + Sync + 'static,
{
    async fn think(&self, context: &AgentContext) -> AgentDecision {
        // 检查是否有工具调用结果需要处理
        if !context.tool_results.is_empty() {
            // 有工具结果，让 LLM 生成最终回复
            return AgentDecision::Chat {
                request: Box::new(self._build_chat_request(context)),
            };
        }

        // 默认：让 LLM 决定是否调用工具
        AgentDecision::Chat {
            request: Box::new(self._build_chat_request_with_tools(context)),
        }
    }

    fn name(&self) -> &str {
        "tool_agent"
    }

    fn description(&self) -> Option<&str> {
        Some("工具调用 Agent，支持自动决定何时调用工具")
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

impl<P> ToolAgent<P>
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

impl<P> ToolAgent<P>
where
    P: LlmProvider,
{
    /// 创建新的构建器
    pub fn builder() -> ToolAgentBuilder<P> {
        ToolAgentBuilder::new()
    }

    /// 构建聊天请求（不带工具）
    fn _build_chat_request(&self, context: &AgentContext) -> ChatRequest {
        let mut request = context.default_chat_request_with(&self.llm_params);
        self._apply_config(&mut request);
        request
    }

    /// 构建聊天请求（带工具定义）
    fn _build_chat_request_with_tools(&self, context: &AgentContext) -> ChatRequest {
        let mut request = context.default_chat_request_with(&self.llm_params);

        // 如果有工具，添加到请求中供 LLM 选择
        let tool_defs = self._get_tool_definitions();
        if !tool_defs.is_empty() {
            request.tools = Some(tool_defs);
        }

        self._apply_config(&mut request);
        request
    }

    /// 应用配置到聊天请求
    fn _apply_config(&self, request: &mut ChatRequest) {
        // 添加系统提示词（如果是第一条消息）
        if let Some(ref prompt) = self.system_prompt
            && (request.messages.is_empty()
                || request.messages.first().map(|m| &m.role) != Some(&Role::System))
        {
            request
                .messages
                .insert(0, ChatMessage::system(prompt.clone()));
        }

        request.model = Some(self.model.clone());
    }

    /// 获取工具定义列表
    fn _get_tool_definitions(&self) -> Vec<ToolDefinition> {
        self.tools.definitions()
    }

    /// 获取工具列表
    pub fn tools(&self) -> Vec<&str> {
        self.tools
            .tool_names()
            .into_iter()
            .map(|s| s.as_str())
            .collect()
    }

    /// 获取 Provider 引用
    pub fn provider(&self) -> &P {
        &self.provider
    }

    /// 获取模型名称
    pub fn model(&self) -> &str {
        &self.model
    }

    /// 获取工具注册表引用
    pub fn tool_registry(&self) -> &ToolRegistry {
        &self.tools
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

/// ToolAgent 构建器
pub struct ToolAgentBuilder<P> {
    provider: Option<P>,
    system_prompt: Option<String>,
    model: Option<String>,
    tools: ToolRegistry,
    max_steps: usize,
    max_tool_concurrency: usize,
    conversation_manager: Option<Arc<Mutex<ConversationManager>>>,
    middleware_chain: crate::middleware::MiddlewareChain,
    enhanced_config: ToolCallEnhancedConfig,
    llm_params: LlmParams,
}

impl<P> ToolAgentBuilder<P> {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            provider: None,
            system_prompt: None,
            model: None,
            tools: ToolRegistry::new(),
            max_steps: 10,
            max_tool_concurrency: 1,
            conversation_manager: None,
            middleware_chain: crate::middleware::MiddlewareChain::new(),
            enhanced_config: ToolCallEnhancedConfig::default(),
            llm_params: LlmParams::default(),
        }
    }
}

impl<P> ToolAgentBuilder<P>
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

    /// 注册工具
    pub fn tool(mut self, tool: impl Tool + 'static) -> Self {
        self.tools = self.tools.register(tool);
        self
    }

    /// 注册多个工具
    pub fn tools<I, T>(mut self, tools: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Tool + 'static,
    {
        for tool in tools {
            self.tools = self.tools.register(tool);
        }
        self
    }

    /// 设置工具注册表
    pub fn tool_registry(mut self, registry: ToolRegistry) -> Self {
        self.tools = registry;
        self
    }

    /// 设置最大步骤数
    pub fn max_steps(mut self, max: usize) -> Self {
        self.max_steps = max;
        self
    }

    /// 设置最大工具并发数
    pub fn max_tool_concurrency(mut self, max: usize) -> Self {
        self.max_tool_concurrency = max.max(1);
        self
    }

    /// 启用对话历史管理
    pub fn with_conversation(mut self, enabled: bool) -> Self {
        if enabled {
            let mut conv = ConversationManager::new();
            if let Some(ref prompt) = self.system_prompt {
                conv = conv.with_system_prompt(prompt.clone());
            }
            self.conversation_manager = Some(Arc::new(Mutex::new(conv)));
        } else {
            self.conversation_manager = None;
        }
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

    /// 设置工具调用增强配置（重试、超时、熔断器、缓存等）
    ///
    /// 默认所有增强特性均关闭，通过此方法按需启用。
    pub fn with_enhanced_config(mut self, config: ToolCallEnhancedConfig) -> Self {
        self.enhanced_config = config;
        self
    }

    /// 设置 LLM 请求参数
    pub fn llm_params(mut self, params: LlmParams) -> Self {
        self.llm_params = params;
        self
    }

    /// 设置 temperature
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

    /// 构建 Agent
    ///
    /// # Panics
    ///
    /// 如果没有设置 `provider` 或 `model`，此方法会 panic。
    pub fn build(self) -> ToolAgent<P> {
        let provider = self.provider.expect("Provider is required");
        let model = self.model.expect("Model is required");

        // 创建执行能力
        let provider_arc = Arc::new(provider);
        let execution =
            DefaultExecution::new(provider_arc.clone(), model.clone(), self.tools.clone())
                .with_system_prompt_opt(self.system_prompt.clone())
                .with_max_steps(self.max_steps)
                .with_max_tool_concurrency(self.max_tool_concurrency)
                .with_conversation_manager(self.conversation_manager.clone())
                .with_middleware_chain(self.middleware_chain)
                .with_enhanced_config(self.enhanced_config)
                .with_llm_params(self.llm_params.clone());

        ToolAgent {
            provider: provider_arc,
            model,
            system_prompt: self.system_prompt,
            tools: self.tools,
            max_steps: self.max_steps,
            conversation_manager: self.conversation_manager,
            llm_params: self.llm_params,
            execution,
        }
    }
}

impl<P> Default for ToolAgentBuilder<P> {
    fn default() -> Self {
        Self::new()
    }
}

/// 向后兼容：DefaultAgent 作为 ToolAgent 的别名
#[deprecated(
    since = "0.2.0",
    note = "DefaultAgent 已重命名为 ToolAgent，请使用 ToolAgent"
)]
pub type DefaultAgent<P> = ToolAgent<P>;

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::stream;
    use futures_util::stream::BoxStream;
    use rucora_core::error::ProviderError;
    use rucora_core::provider::types::{ChatResponse, ChatStreamChunk};

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
    fn test_tool_agent_builder() {
        let _agent = ToolAgentBuilder::<MockProvider>::new()
            .provider(MockProvider)
            .model("gpt-4o-mini")
            .system_prompt("test")
            .max_steps(10)
            .build();
    }
}
