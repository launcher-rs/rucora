//! Agent（智能体）核心抽象模块
//!
//! # 概述
//!
//! 本模块定义了 Agent 的抽象接口。Agent 是能够思考、决策和行动的自主实体。
//!
//! # 核心概念
//!
//! ## Agent vs Runtime
//!
//! - **Agent**: 负责思考、决策、规划（大脑）
//! - **Runtime**: 负责执行、调用、编排（身体）

pub mod types;

use async_trait::async_trait;
use serde_json::Value;

/// Agent 决策结果。
///
/// Agent 通过 `think()` 方法返回决策，Runtime 或其他执行器负责执行。
#[derive(Debug, Clone)]
pub enum AgentDecision {
    /// 调用 LLM 进行对话。
    Chat {
        /// 对话请求。
        request: crate::provider::types::ChatRequest,
    },
    /// 调用工具。
    ToolCall {
        /// 工具名称。
        name: String,
        /// 工具输入参数。
        input: Value,
    },
    /// 直接返回结果。
    Return(Value),
    /// 需要更多思考（继续循环）。
    ThinkAgain,
    /// 停止执行。
    Stop,
}

/// Agent 上下文。
///
/// 包含 Agent 思考所需的所有信息。
#[derive(Debug, Clone)]
pub struct AgentContext {
    /// 用户原始输入。
    pub input: AgentInput,
    /// 对话历史。
    pub messages: Vec<crate::provider::types::ChatMessage>,
    /// 工具调用结果。
    pub tool_results: Vec<ToolResult>,
    /// 当前步骤数。
    pub step: usize,
    /// 最大步骤数。
    pub max_steps: usize,
}

impl AgentContext {
    /// 创建新的上下文。
    pub fn new(input: AgentInput, max_steps: usize) -> Self {
        Self {
            input,
            messages: Vec::new(),
            tool_results: Vec::new(),
            step: 0,
            max_steps,
        }
    }

    /// 添加消息到历史。
    pub fn add_message(&mut self, message: crate::provider::types::ChatMessage) {
        self.messages.push(message);
    }

    /// 添加工具调用结果。
    pub fn add_tool_result(&mut self, tool_name: String, result: Value) {
        self.tool_results.push(ToolResult { tool_name, result });
    }

    /// 创建默认的对话请求。
    pub fn default_chat_request(&self) -> crate::provider::types::ChatRequest {
        crate::provider::types::ChatRequest {
            messages: self.messages.clone(),
            model: None,
            tools: None,
            temperature: Some(0.7),
            max_tokens: None,
            response_format: None,
            metadata: None,
        }
    }
}

/// 工具调用结果。
#[derive(Debug, Clone)]
pub struct ToolResult {
    /// 工具名称。
    pub tool_name: String,
    /// 工具返回结果。
    pub result: Value,
}

/// Agent 输入。
///
/// 用于向 Agent 传递用户请求。
///
/// # 使用示例
///
/// ```rust
/// use agentkit_core::agent::AgentInput;
///
/// // 简单文本输入
/// let input = AgentInput::new("你好");
///
/// // 使用 builder 模式
/// let input = AgentInput::builder("帮我查询天气")
///     .with_context("user_location", "北京")
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct AgentInput {
    /// 文本输入。
    pub text: String,
    /// 额外上下文数据。
    pub context: serde_json::Value,
}

impl AgentInput {
    /// 从文本创建输入。
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            context: serde_json::Value::Null,
        }
    }

    /// 从文本和上下文创建输入。
    pub fn with_context(text: impl Into<String>, context: serde_json::Value) -> Self {
        Self {
            text: text.into(),
            context,
        }
    }

    /// 创建 builder。
    pub fn builder(text: impl Into<String>) -> AgentInputBuilder {
        AgentInputBuilder::new(text)
    }

    /// 获取文本内容。
    pub fn text(&self) -> &str {
        &self.text
    }

    /// 获取上下文数据。
    pub fn context(&self) -> &serde_json::Value {
        &self.context
    }
}

/// AgentInput 构建器。
pub struct AgentInputBuilder {
    text: String,
    context: serde_json::Value,
}

impl AgentInputBuilder {
    /// 创建新的构建器。
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            context: serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    /// 添加上下文键值对。
    pub fn with_context(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        if let serde_json::Value::Object(ref mut map) = self.context {
            map.insert(key.into(), value.into());
        }
        self
    }

    /// 设置完整上下文。
    pub fn context(mut self, context: serde_json::Value) -> Self {
        self.context = context;
        self
    }

    /// 构建输入。
    pub fn build(self) -> AgentInput {
        AgentInput {
            text: self.text,
            context: self.context,
        }
    }
}

impl From<String> for AgentInput {
    fn from(text: String) -> Self {
        Self::new(text)
    }
}

impl From<&str> for AgentInput {
    fn from(text: &str) -> Self {
        Self::new(text)
    }
}

/// Agent 输出。
///
/// 包含 Agent 执行的结果和相关信息。
///
/// # 字段说明
///
/// - `value`: 主要输出内容（通常是 JSON 格式）
/// - `messages`: 对话历史
/// - `tool_calls`: 工具调用记录
///
/// # 使用示例
///
/// ```rust
/// use agentkit_core::agent::AgentOutput;
///
/// // 访问输出内容
/// let output: AgentOutput = get_output();
///
/// // 提取文本内容
/// if let Some(content) = output.value.get("content").and_then(|v| v.as_str()) {
///     println!("回复：{}", content);
/// }
///
/// // 访问对话历史
/// println!("对话轮数：{}", output.messages.len());
///
/// // 访问工具调用
/// println!("工具调用次数：{}", output.tool_calls.len());
/// ```
#[derive(Debug, Clone)]
pub struct AgentOutput {
    /// 主要输出内容（通常是 JSON 格式，包含 `content` 字段）。
    pub value: Value,
    /// 对话历史。
    pub messages: Vec<crate::provider::types::ChatMessage>,
    /// 工具调用记录。
    pub tool_calls: Vec<ToolCallRecord>,
}

impl AgentOutput {
    /// 创建新的输出。
    pub fn new(value: Value) -> Self {
        Self {
            value,
            messages: Vec::new(),
            tool_calls: Vec::new(),
        }
    }

    /// 创建带历史的输出。
    pub fn with_history(
        value: Value,
        messages: Vec<crate::provider::types::ChatMessage>,
        tool_calls: Vec<ToolCallRecord>,
    ) -> Self {
        Self {
            value,
            messages,
            tool_calls,
        }
    }

    /// 获取文本内容（如果存在）。
    pub fn text(&self) -> Option<&str> {
        self.value.get("content").and_then(|v| v.as_str())
    }

    /// 获取对话历史长度。
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// 获取工具调用次数。
    pub fn tool_call_count(&self) -> usize {
        self.tool_calls.len()
    }
}

/// 工具调用记录。
#[derive(Debug, Clone)]
pub struct ToolCallRecord {
    /// 工具名称。
    pub name: String,
    /// 输入参数。
    pub input: Value,
    /// 返回结果。
    pub result: Value,
}

/// Agent trait - 智能体的抽象接口。
///
/// Agent 负责思考、决策和规划。它可以：
/// - 独立运行（处理简单任务）
/// - 嵌入 Runtime 中（处理复杂任务）
/// - 调用 Tool/MCP/Skill/A2A 等外部能力
#[async_trait]
pub trait Agent: Send + Sync {
    /// 思考：分析当前情况，决定下一步行动。
    async fn think(&self, context: &AgentContext) -> AgentDecision;

    /// 获取 Agent 名称。
    fn name(&self) -> &str;

    /// 获取 Agent 描述。
    fn description(&self) -> Option<&str> {
        None
    }

    /// 运行 Agent（独立模式）。
    async fn run(&self, input: impl Into<AgentInput> + Send) -> Result<AgentOutput, AgentError> {
        let input = input.into();
        let mut context = AgentContext::new(input.clone(), 10);

        loop {
            let decision = self.think(&context).await;

            match decision {
                AgentDecision::Return(value) => {
                    return Ok(AgentOutput::with_history(
                        value,
                        context.messages,
                        Vec::new(),
                    ));
                }
                AgentDecision::Stop => {
                    return Ok(AgentOutput::with_history(
                        Value::Null,
                        context.messages,
                        Vec::new(),
                    ));
                }
                AgentDecision::ThinkAgain => {
                    context.step += 1;
                    if context.step >= context.max_steps {
                        return Err(AgentError::MaxStepsReached);
                    }
                }
                AgentDecision::Chat { .. } | AgentDecision::ToolCall { .. } => {
                    return Err(AgentError::RequiresRuntime);
                }
            }
        }
    }
}

/// Agent 错误类型。
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    /// 达到最大步骤数。
    #[error("达到最大步骤数限制")]
    MaxStepsReached,

    /// 需要 Runtime 支持。
    #[error("此决策需要 Runtime 支持，请使用 Runtime 模式运行")]
    RequiresRuntime,

    /// 通用错误消息。
    #[error("{0}")]
    Message(String),
}

/// 默认 Agent 实现。
///
/// 这是一个简单的 Agent，适合大多数对话场景。
/// 它使用 LLM Provider 进行思考，返回对话决策。
///
/// # 使用示例
///
/// ```rust,no_run
/// use agentkit_core::agent::{Agent, DefaultAgent};
/// use agentkit_core::provider::LlmProvider;
///
/// # async fn example<P: LlmProvider + 'static>(provider: P) -> Result<(), Box<dyn std::error::Error>> {
/// let agent = DefaultAgent::builder()
///     .provider(provider)
///     .system_prompt("你是有用的助手")
///     .build();
///
/// let output = agent.run("你好").await?;
/// # Ok(())
/// # }
/// ```
pub struct DefaultAgent<P> {
    #[allow(dead_code)] // provider 字段在运行时通过 think() 方法间接使用
    provider: P,
    system_prompt: Option<String>,
    default_model: Option<String>,
}

/// DefaultAgent 构建器。
pub struct DefaultAgentBuilder<P> {
    provider: Option<P>,
    system_prompt: Option<String>,
    default_model: Option<String>,
}

impl<P> DefaultAgentBuilder<P> {
    /// 创建新的构建器。
    pub fn new() -> Self {
        Self {
            provider: None,
            system_prompt: None,
            default_model: None,
        }
    }
}

impl<P> Default for DefaultAgentBuilder<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P> DefaultAgentBuilder<P>
where
    P: crate::provider::LlmProvider,
{
    /// 设置 Provider。
    pub fn provider(mut self, provider: P) -> Self {
        self.provider = Some(provider);
        self
    }

    /// 设置系统提示词。
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// 设置默认模型。
    pub fn default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = Some(model.into());
        self
    }

    /// 构建 Agent。
    pub fn build(self) -> DefaultAgent<P> {
        DefaultAgent {
            provider: self
                .provider
                .expect("Provider is required for DefaultAgent"),
            system_prompt: self.system_prompt,
            default_model: self.default_model,
        }
    }
}

#[async_trait]
impl<P> Agent for DefaultAgent<P>
where
    P: crate::provider::LlmProvider + Send + Sync,
{
    async fn think(&self, context: &AgentContext) -> AgentDecision {
        let mut request = context.default_chat_request();

        if let Some(ref prompt) = self.system_prompt {
            if request.messages.is_empty()
                || request.messages.first().map(|m| &m.role)
                    != Some(&crate::provider::types::Role::System)
            {
                request.messages.insert(
                    0,
                    crate::provider::types::ChatMessage::system(prompt.clone()),
                );
            }
        }

        if let Some(ref model) = self.default_model {
            request.model = Some(model.clone());
        }

        AgentDecision::Chat { request }
    }

    fn name(&self) -> &str {
        "default_agent"
    }

    fn description(&self) -> Option<&str> {
        Some("默认 Agent 实现，使用 LLM 进行对话")
    }
}
