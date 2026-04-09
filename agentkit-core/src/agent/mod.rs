//! Agent（智能体）核心抽象模块
//!
//! # 概述
//!
//! 本模块定义了 Agent 的抽象接口。Agent 是能够思考、决策和行动的自主实体。
//!
//! # 核心概念
//!
//! ## 决策与执行分离
//!
//! - **Agent trait**: 负责思考、决策、规划（大脑）
//! - **AgentExecutor trait**: 负责执行、调用、编排（身体）
//!
//! Agent 通过 `think()` 方法返回决策，`AgentExecutor` 或其他执行器负责执行。

pub mod types;

use async_trait::async_trait;
use futures_util::stream::BoxStream;
use serde_json::Value;

use crate::channel::types::ChannelEvent;

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
            top_p: None,
            top_k: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            extra: None,
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
            context: serde_json::Value::Object(serde_json::Map::new()),
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
/// - 使用内置执行能力（处理复杂任务）
/// - 调用 Tool/MCP/Skill/A2A 等外部能力
///
/// # 设计原则
///
/// Agent trait 只定义决策接口（`think`），执行能力（`run`/`run_stream`）由具体实现提供。
///
/// ## 决策与执行分离
///
/// - **决策层** (`think`): 每个 Agent 类型有不同的思考策略
/// - **执行层** (`run`/`run_stream`): 所有 Agent 共享相同的执行能力
///
/// ## 使用方式
///
/// ```rust,no_run
/// use agentkit_core::agent::{Agent, AgentContext, AgentDecision, AgentInput, AgentOutput};
/// use async_trait::async_trait;
///
/// struct MyAgent;
///
/// #[async_trait]
/// impl Agent for MyAgent {
///     async fn think(&self, context: &AgentContext) -> AgentDecision {
///         // 自定义决策逻辑
///         AgentDecision::Return(serde_json::json!({"content": "Hello"}))
///     }
///
///     fn name(&self) -> &str { "my_agent" }
/// }
/// ```
///
/// # 内置执行能力
///
/// 如果 Agent 需要工具调用、流式输出等能力，可以组合 `DefaultExecution`：
///
/// ```rust,no_run
/// use agentkit::agent::execution::DefaultExecution;
/// use agentkit_core::agent::Agent;
///
/// struct MyAgent {
///     execution: DefaultExecution,
///     // ... 其他字段
/// }
///
/// impl Agent for MyAgent {
///     // ... 实现 think 方法
///     
///     // DefaultExecution 提供默认的 run/run_stream 实现
/// }
/// ```
#[async_trait]
pub trait Agent: Send + Sync {
    /// 思考：分析当前情况，决定下一步行动。
    ///
    /// 这是 Agent 的核心方法，返回决策结果。
    async fn think(&self, context: &AgentContext) -> AgentDecision;

    /// 获取 Agent 名称。
    fn name(&self) -> &str;

    /// 获取 Agent 描述（可选）。
    ///
    /// 返回 Agent 的简短描述，用于调试和日志。
    fn description(&self) -> Option<&str> {
        None
    }

    /// 运行 Agent（非流式）。
    ///
    /// 默认实现适用于简单场景（直接返回结果）。
    /// 需要工具调用等复杂能力的 Agent 应该使用 `run_with()` 方法配合 `AgentExecutor`。
    ///
    /// # 默认行为
    ///
    /// 默认实现会循环调用 `think()` 直到返回 `Return` 或 `Stop`。
    /// 如果返回 `Chat` 或 `ToolCall`，会返回错误（需要 Runtime 支持）。
    ///
    /// # 配置
    ///
    /// 默认最大步骤数为 20。如果需要自定义，请使用 `run_with()` 方法。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use agentkit_core::agent::{Agent, AgentInput};
    ///
    /// # async fn example(agent: &dyn Agent) -> Result<(), Box<dyn std::error::Error>> {
    /// let output = agent.run(AgentInput::new("你好")).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn run(&self, input: AgentInput) -> Result<AgentOutput, AgentError> {
        // 默认最大步骤数：20
        const DEFAULT_MAX_STEPS: usize = 20;

        let mut context = AgentContext::new(input.clone(), DEFAULT_MAX_STEPS);

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
                        return Err(AgentError::MaxStepsExceeded {
                            max_steps: context.max_steps,
                        });
                    }
                }
                AgentDecision::Chat { request: _ } => {
                    // Chat 决策需要 LLM 调用，默认实现无法处理
                    return Err(AgentError::RequiresRuntime);
                }
                AgentDecision::ToolCall { .. } => {
                    // ToolCall 决策需要工具执行，默认实现无法处理
                    return Err(AgentError::RequiresRuntime);
                }
            }
        }
    }

    /// 运行 Agent（流式）。
    ///
    /// 默认实现返回错误（需要具体实现提供流式能力）。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use agentkit_core::agent::{Agent, AgentInput};
    /// use futures_util::StreamExt;
    ///
    /// # async fn example(agent: &dyn Agent) -> Result<(), Box<dyn std::error::Error>> {
    /// let mut stream = agent.run_stream(AgentInput::new("你好"));
    /// while let Some(event) = stream.next().await {
    ///     match event? {
    ///         agentkit_core::channel::types::ChannelEvent::TokenDelta(delta) => {
    ///             print!("{}", delta.delta);
    ///         }
    ///         _ => {}
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn run_stream(
        &self,
        _input: AgentInput,
    ) -> BoxStream<'static, Result<ChannelEvent, AgentError>> {
        use futures_util::stream;
        Box::pin(stream::once(async {
            Err(AgentError::Message("此 Agent 不支持流式输出".to_string()))
        }))
    }

    /// 运行 Agent（使用执行器）。
    ///
    /// 此方法允许使用外部执行器来运行 Agent。
    /// 这是实现 dyn 兼容的关键方法。
    ///
    /// # 参数
    ///
    /// - `executor`: 执行器，负责实际的运行逻辑
    /// - `input`: 用户输入
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use agentkit_core::agent::{Agent, AgentInput, AgentExecutor};
    ///
    /// # async fn example(agent: &impl Agent, executor: &dyn AgentExecutor) -> Result<(), Box<dyn std::error::Error>> {
    /// let output = agent.run_with(executor, AgentInput::new("你好")).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn run_with(
        &self,
        executor: &dyn AgentExecutor,
        input: AgentInput,
    ) -> Result<AgentOutput, AgentError>
    where
        Self: Sized,
    {
        executor.run(self, input).await
    }
}

/// Agent 执行器 trait
///
/// 用于执行 Agent 的运行逻辑，支持工具调用、流式输出等。
/// 这是实现 dyn 兼容的关键。
#[async_trait]
pub trait AgentExecutor: Send + Sync {
    /// 运行 Agent
    async fn run(&self, agent: &dyn Agent, input: AgentInput) -> Result<AgentOutput, AgentError>;

    /// 流式运行 Agent
    ///
    /// 注意：由于生命周期限制，此方法不支持 Agent 决策。
    /// 它只执行简单的工具调用循环。
    fn run_stream(&self, input: AgentInput)
    -> BoxStream<'static, Result<ChannelEvent, AgentError>>;
}

// 重新导出统一的 AgentError 定义
pub use crate::error::AgentError;
