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

/// 运行时适配器抽象
pub mod runtime_adapter;

use async_trait::async_trait;
use futures_util::stream::BoxStream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

use crate::channel::types::ChannelEvent;
use crate::provider::types::ChatRequest;

/// Agent 决策结果。
///
/// Agent 通过 `think()` 方法返回决策，Runtime 或其他执行器负责执行。
/// 对话执行模式。
///
/// `MapAll` 和 `Reduce` 本质上都是 `Chat` 的变体（需要调用 LLM），
/// 通过此枚举区分执行方式，减少 `AgentDecision` 的枚举复杂度。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatMode {
    /// 单次对话（默认）。
    Single,
    /// 并行处理多个对话请求（Map 阶段）。
    ///
    /// 所有请求会按 `max_concurrency` 限制并发执行，
    /// 结果追加到消息历史后继续循环。
    MapAll,
    /// 归约对话（Reduce 阶段）。
    ///
    /// 处理单个 ChatRequest 后返回最终结果。
    Reduce,
}

#[derive(Debug, Clone)]
pub enum AgentDecision {
    /// 调用 LLM 进行对话。
    Chat {
        /// 对话请求列表。
        ///
        /// `Single`/`Reduce` 模式为一个请求，`MapAll` 模式为多个请求。
        requests: Vec<ChatRequest>,
        /// 最大并发数（仅 `MapAll` 模式使用）。
        max_concurrency: usize,
        /// 对话执行模式。
        mode: ChatMode,
    },
    /// 调用工具。
    ToolCall {
        /// 工具调用 ID。
        ///
        /// 与 `ToolCall.id` 一致，用于关联调用与结果。
        tool_call_id: String,
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

impl AgentDecision {
    /// 创建单次对话决策。
    pub fn chat(request: ChatRequest) -> Self {
        Self::Chat {
            requests: vec![request],
            max_concurrency: 1,
            mode: ChatMode::Single,
        }
    }

    /// 创建 Map 阶段决策（并发执行多个对话请求）。
    pub fn map_all(requests: Vec<ChatRequest>, max_concurrency: usize) -> Self {
        Self::Chat {
            requests,
            max_concurrency,
            mode: ChatMode::MapAll,
        }
    }

    /// 创建 Reduce 阶段决策（处理单个请求后返回最终结果）。
    pub fn reduce(request: ChatRequest) -> Self {
        Self::Chat {
            requests: vec![request],
            max_concurrency: 1,
            mode: ChatMode::Reduce,
        }
    }
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
    ///
    /// 所有 LLM 参数（temperature 等）默认为 None，使用模型默认值。
    /// 可通过 `default_chat_request_with()` 传入自定义参数。
    pub fn default_chat_request(&self) -> crate::provider::types::ChatRequest {
        crate::provider::types::ChatRequest {
            messages: self.messages.clone(),
            model: None,
            tools: None,
            params: crate::provider::types::LlmParams::default(),
            metadata: None,
        }
    }

    /// 创建带 LLM 参数的对话请求。
    pub fn default_chat_request_with(
        &self,
        params: &crate::provider::types::LlmParams,
    ) -> crate::provider::types::ChatRequest {
        let mut request = self.default_chat_request();
        params.apply_to(&mut request);
        request
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
/// use rucora_core::agent::AgentInput;
///
/// // 简单文本输入
/// let input = AgentInput::new("你好").unwrap();
///
/// // 使用 builder 模式
/// let input = AgentInput::builder("帮我查询天气")
///     .unwrap()
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
    ///
    /// # Errors
    ///
    /// 当文本为空时返回 `AgentError::Message`。
    pub fn new(text: impl Into<String>) -> Result<Self, AgentError> {
        let text = text.into();
        if text.is_empty() {
            return Err(AgentError::Message(
                "AgentInput text must not be empty".to_string(),
            ));
        }
        Ok(Self {
            text,
            context: serde_json::Value::Object(serde_json::Map::new()),
        })
    }

    /// 从文本和上下文创建输入。
    ///
    /// # Errors
    ///
    /// 当文本为空时返回 `AgentError::Message`。
    pub fn with_context(
        text: impl Into<String>,
        context: serde_json::Value,
    ) -> Result<Self, AgentError> {
        let text = text.into();
        if text.is_empty() {
            return Err(AgentError::Message(
                "AgentInput text must not be empty".to_string(),
            ));
        }
        Ok(Self { text, context })
    }

    /// 创建 builder。
    ///
    /// # Errors
    ///
    /// 当文本为空时返回 `AgentError::Message`。
    pub fn builder(text: impl Into<String>) -> Result<AgentInputBuilder, AgentError> {
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
    ///
    /// # Errors
    ///
    /// 当文本为空时返回 `AgentError::Message`。
    pub fn new(text: impl Into<String>) -> Result<Self, AgentError> {
        let text = text.into();
        if text.is_empty() {
            return Err(AgentError::Message(
                "AgentInput text must not be empty".to_string(),
            ));
        }
        Ok(Self {
            text,
            context: serde_json::Value::Object(serde_json::Map::new()),
        })
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
        Self::new(text).expect("AgentInput text must not be empty")
    }
}

impl From<&str> for AgentInput {
    fn from(text: &str) -> Self {
        Self::new(text).expect("AgentInput text must not be empty")
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
/// use rucora_core::agent::AgentOutput;
/// use serde_json::json;
///
/// // 创建输出
/// let output = AgentOutput::new(json!({"content": "Hello"}));
///
/// // 提取文本内容
/// if let Some(content) = output.value.get("content").and_then(|v| v.as_str()) {
///     assert_eq!(content, "Hello");
/// }
///
/// // 访问对话历史
/// assert_eq!(output.messages.len(), 0);
///
/// // 访问工具调用
/// assert_eq!(output.tool_calls.len(), 0);
/// ```
#[derive(Debug, Clone)]
pub struct AgentOutput {
    /// 主要输出内容（通常是 JSON 格式，包含 `content` 字段）。
    pub value: Value,
    /// 对话历史。
    pub messages: Vec<crate::provider::types::ChatMessage>,
    /// 工具调用记录。
    pub tool_calls: Vec<ToolCallRecord>,
    /// Token 使用统计（累计所有 LLM 调用的 usage）。
    pub usage: Option<crate::provider::types::Usage>,
}

impl AgentOutput {
    /// 创建新的输出。
    pub fn new(value: Value) -> Self {
        Self {
            value,
            messages: Vec::new(),
            tool_calls: Vec::new(),
            usage: None,
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
            usage: None,
        }
    }

    /// 创建带 usage 的输出。
    pub fn with_usage(
        value: Value,
        messages: Vec<crate::provider::types::ChatMessage>,
        tool_calls: Vec<ToolCallRecord>,
        usage: Option<crate::provider::types::Usage>,
    ) -> Self {
        Self {
            value,
            messages,
            tool_calls,
            usage,
        }
    }

    /// 获取文本内容（如果存在）。
    pub fn text(&self) -> Option<&str> {
        self.value.get("content").and_then(|v| v.as_str())
    }

    /// 获取文本内容，如果不存在则返回空字符串。
    pub fn text_unwrap(&self) -> &str {
        self.text().unwrap_or("")
    }

    /// 获取文本内容，如果不存在则返回默认值。
    pub fn text_or<'a>(&'a self, default: &'a str) -> &'a str {
        self.text().unwrap_or(default)
    }

    /// 消费自身，获取文本内容的所有权。
    pub fn into_text(self) -> String {
        self.text().map(String::from).unwrap_or_default()
    }

    /// 获取对话历史长度。
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// 获取工具调用次数。
    pub fn tool_call_count(&self) -> usize {
        self.tool_calls.len()
    }

    /// 获取 Token 使用统计。
    pub fn usage(&self) -> Option<&crate::provider::types::Usage> {
        self.usage.as_ref()
    }

    /// 获取总 Token 数。
    pub fn total_tokens(&self) -> u32 {
        self.usage.as_ref().map_or(0, |u| u.total_tokens)
    }

    /// 获取提示词 Token 数。
    pub fn prompt_tokens(&self) -> u32 {
        self.usage.as_ref().map_or(0, |u| u.prompt_tokens)
    }

    /// 获取输出 Token 数。
    pub fn completion_tokens(&self) -> u32 {
        self.usage.as_ref().map_or(0, |u| u.completion_tokens)
    }

    /// 格式化 Token 使用信息。
    pub fn usage_summary(&self) -> String {
        match &self.usage {
            Some(u) => format!(
                "Tokens: {} total ({} prompt + {} completion)",
                u.total_tokens, u.prompt_tokens, u.completion_tokens
            ),
            None => "Tokens: N/A".to_string(),
        }
    }
}

impl std::fmt::Display for AgentOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.text() {
            Some(text) => write!(f, "{text}"),
            None => Ok(()),
        }
    }
}

/// 工具调用记录。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    /// 工具名称。
    pub name: String,
    /// 工具调用 ID，用于关联调用与结果。
    pub tool_call_id: String,
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
/// ```rust
/// use rucora_core::agent::{Agent, AgentContext, AgentDecision};
/// use async_trait::async_trait;
///
/// struct MyAgent;
///
/// #[async_trait]
/// impl Agent for MyAgent {
///     async fn think(&self, _context: &AgentContext) -> AgentDecision {
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
/// ```rust,ignore
/// use rucora::agent::execution::DefaultExecution;
/// use rucora_core::agent::Agent;
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
    /// 此默认实现仅适用于**纯推理 Agent**（无需 LLM 调用和工具调用，例如自定义的
    /// 决策型 Agent）。需要 LLM 对话、工具执行或流式输出的 Agent 请使用
    /// `run_with(executor, input)` 或 `run_stream()`。
    ///
    /// # 默认行为
    ///
    /// 默认实现会循环调用 `think()` 直到返回 `Return` 或 `Stop`，并受
    /// `AgentContext.max_steps`（默认 20）限制。
    ///
    /// 如果 `think()` 返回 `Chat`（含 `MapAll`/`Reduce` 模式）或 `ToolCall`，
    /// 则返回 `AgentError::RequiresRuntime`——这些决策需要执行器（LLM 调用、
    /// 工具执行）支持，请改用 `run_with(executor, input)`。
    ///
    /// # 何时使用默认 `run()` vs `run_with()`
    ///
    /// - **纯推理 Agent**（无需 LLM/工具调用）：直接使用 `run()` 即可。
    /// - **需要工具调用或 LLM 的 Agent**：使用 `run_with(executor, input)`，传入一个
    ///   `AgentExecutor` 实现（如 `DefaultExecution`）。
    /// - **需要流式输出的 Agent**：使用 `run_stream()` 或 `run_with().run_stream()`。
    ///
    /// # 示例
    ///
    /// ## 纯推理 Agent（无 LLM/工具调用）
    /// ```rust
    /// use rucora_core::agent::{Agent, AgentContext, AgentDecision, AgentInput, AgentOutput};
    /// use async_trait::async_trait;
    ///
    /// struct EchoAgent;
    ///
    /// #[async_trait]
    /// impl Agent for EchoAgent {
    ///     async fn think(&self, context: &AgentContext) -> AgentDecision {
    ///         let value = serde_json::json!({"content": context.input.text()});
    ///         AgentDecision::Return(value)
    ///     }
    ///
    ///     fn name(&self) -> &str { "echo" }
    /// }
    ///
    /// #[tokio::main(flavor = "current_thread")]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let output = EchoAgent.run(AgentInput::new("你好")?).await?;
    ///     assert_eq!(output.text().unwrap(), "你好");
    ///     Ok(())
    /// }
    /// ```
    ///
    /// ## 需要 LLM/工具执行的 Agent
    /// ```rust,ignore
    /// use rucora::agent::execution::DefaultExecution;
    /// use rucora::agent::ToolAgent;
    /// use rucora_core::agent::{Agent, AgentInput, AgentExecutor};
    ///
    /// # async fn example(agent: &impl Agent, executor: &dyn AgentExecutor) -> Result<(), Box<dyn std::error::Error>> {
    /// let output = agent.run_with(executor, AgentInput::new("你好")?).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn run(&self, input: AgentInput) -> Result<AgentOutput, AgentError> {
        // 默认最大步骤数：20
        // 需要自定义请使用 `run_with(executor, input)` 方法
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
                AgentDecision::Chat { .. } | AgentDecision::ToolCall { .. } => {
                    // 这些决策需要 LLM 调用或工具执行，默认 run() 仅适用于纯推理 Agent。
                    // 请改用 run_with(executor, input) 或 run_stream()。
                    return Err(AgentError::RequiresRuntime);
                }
            }
        }
    }

    /// 运行 Agent（带超时控制）。
    ///
    /// 此方法允许设置 Agent 级别的整体超时时间。超时后，Agent 会停止执行
    /// 并返回 `AgentError::Timeout` 错误。
    ///
    /// # 参数
    ///
    /// - `input`: 用户输入
    /// - `timeout`: 超时时间
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// use rucora_core::agent::{Agent, AgentInput};
    /// use std::time::Duration;
    ///
    /// # async fn example(agent: &dyn Agent) -> Result<(), Box<dyn std::error::Error>> {
    /// let input = AgentInput::new("请在30秒内完成这个任务");
    /// let output = agent.run_with_timeout(input, Duration::from_secs(30)).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn run_with_timeout(
        &self,
        input: AgentInput,
        timeout: std::time::Duration,
    ) -> Result<AgentOutput, AgentError> {
        tokio::time::timeout(timeout, self.run(input))
            .await
            .map_err(|_| AgentError::Timeout { duration: timeout })?
    }

    /// 并发运行多个独立输入。
    ///
    /// 默认实现为每个输入 `tokio::task::spawn` 一个独立任务，可在多核 CPU 上
    /// 真正并行执行，然后通过 `buffer_unordered` 按完成顺序收集结果。
    /// 适用于翻译、批量问答等场景。
    ///
    /// 此方法要求 `Self: 'static`，因此以 `Arc<Self>` 接收者调用。
    ///
    /// # 参数
    ///
    /// - `inputs`: 多个用户输入
    /// - `max_concurrency`: 最大并发数
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// use rucora_core::agent::{Agent, AgentInput};
    ///
    /// # async fn example(agent: std::sync::Arc<impl Agent>) -> Result<(), Box<dyn std::error::Error>> {
    /// let inputs = vec![
    ///     AgentInput::new("Hello")?,
    ///     AgentInput::new("World")?,
    /// ];
    /// let outputs = agent.run_batch(inputs, 4).await;
    /// for result in &outputs {
    ///     match result {
    ///         Ok(output) => println!("{}", output.text().unwrap_or("")),
    ///         Err(e) => eprintln!("失败：{e}"),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # 重试
    ///
    /// 需要重试时，使用 `ResilientProvider` 包裹底层 provider 即可，
    /// 无需在 `run_batch` 层额外处理。
    async fn run_batch(
        self: Arc<Self>,
        inputs: Vec<AgentInput>,
        max_concurrency: usize,
    ) -> Vec<Result<AgentOutput, AgentError>>
    where
        Self: 'static,
    {
        use futures_util::StreamExt;

        let tasks = inputs.into_iter().map(|input| {
            let this = self.clone();
            tokio::task::spawn(async move { this.run(input).await })
        });
        let results: Vec<_> = futures_util::stream::iter(tasks)
            .buffer_unordered(max_concurrency)
            .map(|handle| {
                handle.unwrap_or_else(|join_err| {
                    Err(AgentError::Message(format!("run_batch 任务执行失败：{join_err}")))
                })
            })
            .collect()
            .await;

        results
    }

    /// 运行 Agent（流式）。
    ///
    /// 默认实现返回一个包含错误信息的 stream，表示此 Agent 不支持流式输出。
    /// 需要流式支持的 Agent 应重写此方法，或使用 `run_with()` 配合流式执行器。
    ///
    /// # 何时重写此方法
    ///
    /// - Agent 需要流式输出 Token 级增量
    /// - Agent 需要流式工具调用反馈
    /// - 构建聊天机器人等需要实时交互的场景
    ///
    /// # 示例
    ///
    /// ## 使用默认实（不支持流式）
    /// ```rust,ignore
    /// use rucora_core::agent::{Agent, AgentInput};
    /// use futures_util::StreamExt;
    ///
    /// # async fn example(agent: &dyn Agent) -> Result<(), Box<dyn std::error::Error>> {
    /// let mut stream = agent.run_stream(AgentInput::new("你好"));
    /// while let Some(event) = stream.next().await {
    ///     match event? {
    ///         rucora_core::channel::types::ChannelEvent::TokenDelta(delta) => {
    ///             print!("{}", delta.delta);
    ///         }
    ///         _ => {}
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## 使用 `DefaultExecution` 提供流式支持
    /// ```rust,ignore
    /// use rucora::agent::execution::DefaultExecution;
    /// use rucora_core::agent::{Agent, AgentInput, AgentExecutor};
    ///
    /// # async fn example(agent: &impl Agent, executor: &dyn AgentExecutor) -> Result<(), Box<dyn std::error::Error>> {
    /// let stream = executor.run_stream(AgentInput::new("你好"));
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
    /// use rucora_core::agent::{Agent, AgentInput, AgentExecutor};
    ///
    /// # async fn example(agent: &impl Agent, executor: &dyn AgentExecutor) -> Result<(), Box<dyn std::error::Error>> {
    /// let output = agent.run_with(executor, AgentInput::new("你好")?).await?;
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

/// 重新导出运行时适配器相关类型
pub use runtime_adapter::{
    LogLevel, NativeRuntimeAdapter, RestrictedRuntimeAdapter, RuntimeAdapter, RuntimeCapabilities,
    RuntimeError, RuntimePlatform, ShellResult,
};
