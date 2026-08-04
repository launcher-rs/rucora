//! ReActAgent - 推理 + 行动 Agent
//!
//! # 概述
//!
//! ReActAgent 实现显式的 ReAct（Reason + Act）循环：
//! 1. **Think**（思考）：分析问题，规划步骤
//! 2. **Act**（行动）：执行工具调用
//! 3. **Observe**（观察）：分析工具结果
//! 4. 循环直到完成任务
//!
//! # 适用场景
//!
//! - 需要多步推理的复杂任务
//! - 需要分析和规划的任务
//! - 代码分析、项目调研等
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use rucora::agent::ReActAgent;
//! use rucora::provider::OpenAiProvider;
//! use rucora::tools::{ShellTool, FileReadTool};
//! use rucora::prelude::Agent;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = OpenAiProvider::from_env()?;
//!
//! let agent = ReActAgent::builder(provider)
//!     .model("gpt-4o-mini")
//!     .system_prompt("你是一个善于推理的助手")
//!     .tool(ShellTool::new())
//!     .tool(FileReadTool::new())
//!     .max_steps(15)
//!     .build();
//!
//! // 复杂任务：先分析，再分步执行
//! let output = agent.run("帮我分析这个项目的代码结构，找出所有 Rust 文件并统计行数".into()).await?;
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use rucora_core::agent::{Agent, AgentContext, AgentDecision, AgentInput, AgentOutput};
use rucora_core::provider::LlmProvider;
use rucora_core::provider::types::{ChatMessage, ChatRequest, LlmParams};
use rucora_core::tool::Tool;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::agent::ToolRegistry;
use crate::agent::{NoModel, WithModel};
use crate::agent::execution::{DefaultExecution, build_default_execution};
use crate::conversation::ConversationManager;

/// ReActAgent - 推理 + 行动 Agent
///
/// 特点：
/// - 显式的思考 - 行动 - 观察循环
/// - 每一步都先思考再行动
/// - 适合多步推理任务
pub struct ReActAgent<P> {
    /// LLM Provider（持有泛型参数 P）
    _provider: Arc<P>,
    /// Agent 级模型覆盖；为空时使用 Provider 默认模型。
    model: Option<String>,
    /// 工具注册表
    tools: ToolRegistry,
    /// 最大步骤数
    max_steps: usize,
    /// LLM 请求参数
    llm_params: LlmParams,
    /// 执行能力（内聚）
    execution: DefaultExecution,
}

#[async_trait]
impl<P> Agent for ReActAgent<P>
where
    P: LlmProvider + Send + Sync + 'static,
{
    async fn think(&self, context: &AgentContext) -> AgentDecision {
        // ReAct 核心：显式思考步骤
        if context.step == 0 {
            // 第一步：先思考，不工具调用
            AgentDecision::chat(self._build_react_prompt(context, "think"))
        } else if !context.tool_results.is_empty() {
            // 有工具结果：观察后继续思考
            AgentDecision::chat(self._build_react_prompt(context, "observe"))
        } else {
            // 正常：决定行动
            AgentDecision::chat(self._build_react_prompt(context, "act"))
        }
    }

    fn name(&self) -> &str {
        "react_agent"
    }

    fn description(&self) -> Option<&str> {
        Some("ReAct Agent，显式的推理 + 行动循环")
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

impl<P> ReActAgent<P>
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

impl<P> ReActAgent<P>
where
    P: LlmProvider,
{
    /// 创建新的构建器。
    ///
    /// # 参数
    ///
    /// - `provider`: LLM Provider（必需）
    #[must_use = "构建器必须调用 build() 来创建 Agent"]
    pub fn builder(provider: P) -> ReActAgentBuilder<P> {
        ReActAgentBuilder::new(provider)
    }

    /// 构建 ReAct 提示词
    fn _build_react_prompt(&self, context: &AgentContext, phase: &str) -> ChatRequest {
        let prompt = match phase {
            "think" => format!(
                "请分析用户问题，规划解题步骤。\n\
                 \n\
                 思考步骤：\n\
                 1. 理解用户需求\n\
                 2. 确定需要什么信息\n\
                 3. 规划使用哪些工具\n\
                 \n\
                 可用工具：{:?}\n\
                 \n\
                 请详细分析并规划步骤。",
                self.tools.tool_names()
            ),
            "act" => format!(
                "基于以上思考，请选择合适的工具行动。\n\
                 \n\
                 可用工具：{:?}\n\
                 \n\
                 如果需要调用工具，请使用工具调用格式。",
                self.tools.tool_names()
            ),
            "observe" => format!(
                "观察工具执行结果，分析是否完成任务。\n\
                 \n\
                 如果完成，给出最终答案；否则继续思考下一步。\n\
                 \n\
                 当前步骤：{}/{}",
                context.step, self.max_steps
            ),
            _ => {
                tracing::warn!("ReActAgent: 未知阶段 '{}'，回退到 'think'", phase);
                format!(
                    "请分析用户问题，规划解题步骤。\n\
                     \n\
                     可用工具：{:?}\n\
                     \n\
                     请详细分析并规划步骤。",
                    self.tools.tool_names()
                )
            }
        };

        // 构建消息历史（context.messages 已包含用户输入，无需重新注入）
        let mut messages = context.messages.clone();

        // 添加 ReAct 提示词（使用 assistant 角色前缀以保持对话连贯）
        messages.push(ChatMessage::user(prompt));

        let mut request = ChatRequest {
            messages,
            model: self.model.clone(),
            tools: Some(self.tools.definitions()),
            ..Default::default()
        };
        self.llm_params.apply_to(&mut request);
        request
    }

    /// 获取工具列表
    pub fn tools(&self) -> Vec<&str> {
        self.tools
            .tool_names()
            .into_iter()
            .map(|s| s.as_str())
            .collect()
    }
}

/// ReActAgent 构建器（Typestate 模式）
///
/// 泛型参数 `S` 为构建器状态，用于在编译期强制调用 `.model(...)`：
/// - `builder(provider)` 返回「未设置 model」状态的构建器 `ReActAgentBuilder<P, NoModel>`；
/// - 只有调用 `.model(...)` 后才会转为「已设置 model」状态 `ReActAgentBuilder<P, WithModel>`；
/// - `build()` 仅存在于 `WithModel` 状态，因此忘记设置 model 将无法通过编译。
pub struct ReActAgentBuilder<P, S = NoModel> {
    provider: P,
    system_prompt: Option<String>,
    model: Option<String>,
    tools: ToolRegistry,
    max_steps: usize,
    with_conversation: bool,
    middleware_chain: crate::middleware::MiddlewareChain,
    llm_params: LlmParams,
    _marker: std::marker::PhantomData<S>,
}

impl<P> ReActAgentBuilder<P, NoModel> {
    /// 创建新的构建器。
    ///
    /// # 参数
    ///
    /// - `provider`: LLM Provider（必需）
    ///
    /// 初始为「未设置 model」状态，需调用 `.model(...)` 后才可 `.build()`。
    pub fn new(provider: P) -> ReActAgentBuilder<P, NoModel> {
        ReActAgentBuilder {
            provider,
            system_prompt: None,
            model: None,
            tools: ToolRegistry::new(),
            max_steps: 15, // ReAct 通常需要更多步骤
            with_conversation: false,
            middleware_chain: crate::middleware::MiddlewareChain::new(),
            llm_params: LlmParams::default(),
            _marker: std::marker::PhantomData,
        }
    }

    /// 设置默认模型（必需）。必须调用后才能 [`build`](ReActAgentBuilder::build)，
    /// 状态会从「未设置 model」转为「已设置 model」。
    pub fn model(self, model: impl Into<String>) -> ReActAgentBuilder<P, WithModel> {
        ReActAgentBuilder {
            provider: self.provider,
            system_prompt: self.system_prompt,
            model: Some(model.into()),
            tools: self.tools,
            max_steps: self.max_steps,
            with_conversation: self.with_conversation,
            middleware_chain: self.middleware_chain,
            llm_params: self.llm_params,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<P, S> ReActAgentBuilder<P, S>
where
    P: LlmProvider + Send + Sync + 'static,
{
    /// 设置系统提示词
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
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

    /// 设置最大步骤数
    pub fn max_steps(mut self, max: usize) -> Self {
        self.max_steps = max;
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
    pub fn with_conversation(mut self, enabled: bool) -> Self {
        self.with_conversation = enabled;
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
}

impl<P> ReActAgentBuilder<P, WithModel>
where
    P: LlmProvider + Send + Sync + 'static,
{
    /// 构建 Agent。
    pub fn build(self) -> ReActAgent<P> {
        let provider = self.provider;
        let conversation_manager = if self.with_conversation {
            let mut conv = ConversationManager::new();
            if let Some(ref prompt) = self.system_prompt {
                conv = conv.with_system_prompt(prompt.clone());
            }
            Some(Arc::new(Mutex::new(conv)))
        } else {
            None
        };

        // 创建执行能力
        let provider_arc = Arc::new(provider);
        let execution = build_default_execution(crate::agent::ExecutionBuildConfig {
            provider: provider_arc.clone(),
            model: self.model.clone(),
            tools: self.tools.clone(),
            system_prompt: self.system_prompt.clone(),
            max_steps: self.max_steps,
            max_tool_concurrency: 1,
            conversation_manager,
            middleware_chain: self.middleware_chain.clone(),
            enhanced_config: crate::agent::tool_call_config::ToolCallEnhancedConfig::default(),
            llm_params: self.llm_params.clone(),
        });

        ReActAgent {
            _provider: provider_arc,
            model: self.model,
            tools: self.tools,
            max_steps: self.max_steps,
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
    fn test_react_agent_builder() {
        let _agent = ReActAgentBuilder::<MockProvider>::new(MockProvider)
            .model("gpt-4o-mini")
            .max_steps(15)
            .build();
    }

    /// 验证 Typestate 状态转换：`.model()` 将构建器从 NoModel 状态转为 WithModel 状态。
    #[test]
    fn test_builder_typestate_transition() {
        let builder: ReActAgentBuilder<MockProvider, NoModel> =
            ReActAgentBuilder::<MockProvider>::new(MockProvider);
        let builder: ReActAgentBuilder<MockProvider, WithModel> = builder.model("gpt-4o-mini");
        let _agent = builder.build();
    }
}
