//! 默认运行时实现模块
//!
//! # 概述
//!
//! `DefaultRuntime` 是 Agentkit 的默认运行时实现，提供完整的工具调用循环（Tool-Calling Loop）。
//! 它负责与 LLM Provider 交互、执行工具调用、管理对话历史等。
//!
//! # 重要说明
//!
//! **Runtime 必须指定默认模型**。模型是 Runtime 配置的核心部分，Provider 仅提供 AI 能力，
//! 而 Runtime 作为决策单元负责选择合适的模型。
//!
//! # 使用示例
//!
//! ## 基本使用（必须指定模型）
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use agentkit_runtime::{DefaultRuntime, ToolRegistry};
//! use agentkit_core::provider::LlmProvider;
//! use agentkit_core::agent::types::AgentInput;
//!
//! # async fn example(provider: Arc<dyn LlmProvider>) -> Result<(), Box<dyn std::error::Error>> {
//! // 创建工具注册表
//! let tools = ToolRegistry::new();
//!
//! // 创建运行时（必须指定模型）
//! let runtime = DefaultRuntime::new(provider, tools, "gpt-4o-mini");
//! runtime
//!     .with_system_prompt("你是一个有用的助手");
//!
//! // 执行对话
//! let input = AgentInput::new("你好");
//! let output = runtime.run(input).await?;
//! println!("回复：{}", output.text().unwrap_or("无回复"));
//! # Ok(())
//! # }
//! ```
//!
//! ## 流式执行
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use futures_util::StreamExt;
//! use agentkit_runtime::{DefaultRuntime, ToolRegistry, ChannelEvent};
//! use agentkit_core::provider::LlmProvider;
//! use agentkit_core::agent::types::AgentInput;
//!
//! # async fn example(provider: Arc<dyn LlmProvider>) -> Result<(), Box<dyn std::error::Error>> {
//! let runtime = DefaultRuntime::new(provider, ToolRegistry::new(), "gpt-4o-mini");
//! let input = AgentInput { messages: vec![], metadata: None };
//!
//! // 流式执行
//! let mut stream = runtime.run_stream(input);
//! while let Some(event) = stream.next().await {
//!     match event {
//!         Ok(ChannelEvent::TokenDelta(delta)) => {
//!             print!("{}", delta.delta);  // 打印 token
//!         }
//!         Ok(ChannelEvent::ToolCall(call)) => {
//!             println!("\n调用工具：{}", call.name);
//!         }
//!         Ok(ChannelEvent::ToolResult(result)) => {
//!             println!("工具结果：{}", result.output);
//!         }
//!         _ => {}
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## 使用构建器
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use agentkit_runtime::{DefaultRuntimeBuilder, RuntimeConfig, ToolRegistry};
//! use agentkit_core::provider::LlmProvider;
//!
//! # async fn example(provider: Arc<dyn LlmProvider>) -> Result<(), Box<dyn std::error::Error>> {
//! // 必须指定 model
//! let runtime = DefaultRuntimeBuilder::new()
//!     .provider(provider)
//!     .tools(ToolRegistry::new())
//!     .model("gpt-4o-mini")  // ← 必须设置
//!     .system_prompt("你是一个有用的助手")
//!     .max_steps(10)
//!     .max_tool_concurrency(3)
//!     .build()?;
//! # Ok(())
//! # }
//! ```
//!
//! # 架构说明
//!
//! ## 执行流程
//!
//! ```text
//! 1. 接收用户输入
//!    │
//!    ▼
//! 2. 添加系统提示词（如果有）
//!    │
//!    ▼
//! 3. 调用 LLM Provider（带工具定义）
//!    │
//!    ▼
//! 4. 检查是否有工具调用
//!    │
//!    ├─ 无工具调用 ──► 返回结果，结束
//!    │
//!    ▼
//! 5. 有工具调用
//!    │
//!    ▼
//! 6. 执行策略检查（Policy Check）
//!    │
//!    ▼
//! 7. 执行工具（支持并发）
//!    │
//!    ▼
//! 8. 将工具结果添加到对话历史
//!    │
//!    ▼
//! 9. 回到步骤 3，继续循环
//!    │
//!    ▼
//! 10. 达到最大步数 ──► 返回错误
//! ```
//!
//! # 配置说明
//!
//! ## RuntimeConfig
//!
//! - `max_steps`: 最大执行步数（默认 8），防止无限循环
//! - `max_tool_concurrency`: 工具并发执行数（默认 1）
//! - `enable_tool_logging`: 是否启用工具执行日志
//! - `debug_mode`: 是否启用详细调试模式
//!
//! ## 工具策略
//!
//! 通过 `ToolPolicy` trait 实现工具调用的 allow/deny 检查。
//! 内置策略：
//! - `DefaultToolPolicy`: 默认策略，拦截危险命令
//! - `AllowAllToolPolicy`: 允许所有工具调用

use std::sync::Arc;

use async_trait::async_trait;
use futures_util::stream;
use futures_util::{StreamExt, stream::BoxStream};
use serde_json::json;
use tracing::{debug, info, warn};

use agentkit_core::agent::{
    Agent, AgentContext, AgentDecision, AgentInput, AgentOutput, ToolCallRecord,
};
use agentkit_core::channel::types::{ChannelEvent, DebugEvent, ErrorEvent, TokenDeltaEvent};
use agentkit_core::error::{AgentError, DiagnosticError};
use agentkit_core::provider::LlmProvider;
use agentkit_core::provider::types::{ChatMessage, ChatRequest, Role};
use agentkit_core::runtime::{NoopRuntimeObserver, Runtime, RuntimeObserver};
use agentkit_core::tool::types::{ToolCall, ToolResult};

use crate::runtime::policy::{DefaultToolPolicy, ToolPolicy};
use crate::runtime::tool_execution::{
    execute_tool_call_with_policy_and_observer, tool_result_to_message,
};
use crate::runtime::tool_registry::{ToolRegistry, ToolSource};

/// 运行时配置
///
/// 用于控制 `DefaultRuntime` 的行为。
///
/// # 字段说明
///
/// - `model`: 默认使用的模型名称（必需）
/// - `max_steps`: 最大执行步数，防止无限循环（默认 8）
/// - `max_tool_concurrency`: 工具并发执行数（默认 1）
/// - `enable_tool_logging`: 是否启用工具执行日志
/// - `debug_mode`: 是否启用详细调试模式
///
/// # 设计理念
///
/// **Runtime 必须指定默认模型**。模型是 Runtime 配置的核心部分，Provider 仅提供 AI 能力，
/// 而 Runtime 作为决策单元负责选择合适的模型。
///
/// # 示例
///
/// ```rust
/// use agentkit_runtime::RuntimeConfig;
///
/// // 必须指定 model
/// let config = RuntimeConfig::new("gpt-4o-mini")
///     .with_max_steps(10)
///     .with_max_tool_concurrency(3);
/// ```
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// 默认使用的模型名称
    pub model: String,
    /// 最大执行步数
    pub max_steps: usize,
    /// 工具并发执行数
    pub max_tool_concurrency: usize,
    /// 是否启用工具执行日志
    pub enable_tool_logging: bool,
    /// 是否启用详细调试模式
    pub debug_mode: bool,
}

impl RuntimeConfig {
    /// 创建新的运行时配置（必须指定 model）
    ///
    /// # 参数
    ///
    /// - `model`: 默认使用的模型名称
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit_runtime::RuntimeConfig;
    ///
    /// let config = RuntimeConfig::new("gpt-4o-mini");
    /// ```
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            max_steps: 8,
            max_tool_concurrency: 1,
            enable_tool_logging: true,
            debug_mode: false,
        }
    }

    /// 设置最大执行步数
    pub fn with_max_steps(mut self, max_steps: usize) -> Self {
        self.max_steps = max_steps;
        self
    }

    /// 设置工具并发执行数
    pub fn with_max_tool_concurrency(mut self, max_tool_concurrency: usize) -> Self {
        self.max_tool_concurrency = max_tool_concurrency;
        self
    }

    /// 设置是否启用工具执行日志
    pub fn with_tool_logging(mut self, enable_tool_logging: bool) -> Self {
        self.enable_tool_logging = enable_tool_logging;
        self
    }

    /// 设置是否启用详细调试模式
    pub fn with_debug_mode(mut self, debug_mode: bool) -> Self {
        self.debug_mode = debug_mode;
        self
    }
}

/// 默认的运行时实现
///
/// 提供完整的 tool-calling loop，支持：
/// - 非流式执行（`run`）
/// - 流式执行（`run_stream`）
/// - 工具策略（`ToolPolicy`）
/// - 统一观测（`RuntimeObserver`）
/// - 多种工具来源（内置、Skill、MCP、A2A）
///
/// # 示例
///
/// ```rust,no_run
/// use std::sync::Arc;
/// use agentkit_runtime::{DefaultRuntime, ToolRegistry};
/// use agentkit_core::provider::LlmProvider;
///
/// # async fn example(provider: Arc<dyn LlmProvider>) {
/// // Runtime 必须指定 model
/// let runtime = DefaultRuntime::new(provider, ToolRegistry::new(), "gpt-4o-mini")
///     .with_system_prompt("你是一个有用的助手");
/// # }
/// ```
pub struct DefaultRuntime {
    /// LLM Provider
    provider: Arc<dyn LlmProvider>,
    /// 系统提示词
    system_prompt: Option<String>,
    /// 默认使用的模型
    model: String,
    /// 工具注册表
    tools: ToolRegistry,
    /// 工具策略
    policy: Arc<dyn ToolPolicy>,
    /// 观测器
    observer: Arc<dyn RuntimeObserver>,
    /// 运行时配置
    config: RuntimeConfig,
}

impl DefaultRuntime {
    /// 创建新的运行时（必须指定 model）
    ///
    /// # 参数
    ///
    /// - `provider`: LLM Provider
    /// - `tools`: 工具注册表
    /// - `model`: 默认使用的模型名称
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use std::sync::Arc;
    /// use agentkit_runtime::{DefaultRuntime, ToolRegistry};
    /// use agentkit_core::provider::LlmProvider;
    ///
    /// # async fn example(provider: Arc<dyn LlmProvider>) {
    /// let runtime = DefaultRuntime::new(provider, ToolRegistry::new(), "gpt-4o-mini");
    /// # }
    /// ```
    pub fn new(
        provider: Arc<dyn LlmProvider>,
        tools: ToolRegistry,
        model: impl Into<String>,
    ) -> Self {
        let model = model.into();
        Self {
            provider,
            system_prompt: None,
            model: model.clone(),
            tools,
            policy: Arc::new(DefaultToolPolicy::new()),
            observer: Arc::new(NoopRuntimeObserver),
            config: RuntimeConfig::new(model),
        }
    }

    /// 使用自定义配置创建运行时
    ///
    /// # 参数
    ///
    /// - `provider`: LLM Provider
    /// - `tools`: 工具注册表
    /// - `config`: 运行时配置（必须包含 model）
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use std::sync::Arc;
    /// use agentkit_runtime::{DefaultRuntime, RuntimeConfig, ToolRegistry};
    /// use agentkit_core::provider::LlmProvider;
    ///
    /// # async fn example(provider: Arc<dyn LlmProvider>) {
    /// let config = RuntimeConfig::new("gpt-4o-mini")
    ///     .with_max_steps(10);
    /// let runtime = DefaultRuntime::with_config(provider, ToolRegistry::new(), config);
    /// # }
    /// ```
    pub fn with_config(
        provider: Arc<dyn LlmProvider>,
        tools: ToolRegistry,
        config: RuntimeConfig,
    ) -> Self {
        Self {
            provider,
            system_prompt: None,
            model: config.model.clone(),
            tools,
            policy: Arc::new(DefaultToolPolicy::new()),
            observer: Arc::new(NoopRuntimeObserver),
            config,
        }
    }

    /// 设置系统提示词
    pub fn with_system_prompt(mut self, system_prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(system_prompt.into());
        self
    }

    /// 设置工具策略
    pub fn with_policy(mut self, policy: Arc<dyn ToolPolicy>) -> Self {
        self.policy = policy;
        self
    }

    /// 设置观测器
    pub fn with_observer(mut self, observer: Arc<dyn RuntimeObserver>) -> Self {
        self.observer = observer;
        self
    }

    /// 设置运行时配置
    pub fn with_config_mut(mut self, config: RuntimeConfig) -> Self {
        self.config = config;
        self
    }

    /// 设置最大步数
    pub fn with_max_steps(mut self, max_steps: usize) -> Self {
        self.config.max_steps = max_steps;
        self
    }

    /// 设置最大工具并发数
    pub fn with_max_tool_concurrency(mut self, max_concurrency: usize) -> Self {
        self.config.max_tool_concurrency = max_concurrency.max(1);
        self
    }

    /// 获取工具注册表的引用
    pub fn tools(&self) -> &ToolRegistry {
        &self.tools
    }

    /// 获取可变工具注册表
    pub fn tools_mut(&mut self) -> &mut ToolRegistry {
        &mut self.tools
    }

    /// 从其他注册表添加工具
    pub fn add_tools(&mut self, registry: ToolRegistry) {
        self.tools = self.tools.clone().merge(registry);
    }

    /// 启用/禁用工具
    pub fn set_tool_enabled(&mut self, name: &str, enabled: bool) {
        self.tools.set_tool_enabled(name, enabled);
    }

    /// 按来源获取工具数量
    pub fn tool_count_by_source(&self, source: ToolSource) -> usize {
        self.tools.filter_by_source(source).len()
    }

    fn emit(&self, event: ChannelEvent) {
        self.observer.on_event(event);
    }

    async fn execute_tool_calls(&self, calls: &[ToolCall]) -> Result<Vec<ToolResult>, AgentError> {
        if calls.is_empty() {
            return Ok(vec![]);
        }

        let max = self.config.max_tool_concurrency.max(1);
        if max == 1 || calls.len() == 1 {
            let mut out: Vec<ToolResult> = Vec::with_capacity(calls.len());
            for call in calls {
                out.push(
                    execute_tool_call_with_policy_and_observer(
                        &self.tools,
                        &self.policy,
                        &self.observer,
                        call,
                    )
                    .await?,
                );
            }
            return Ok(out);
        }

        let tools = self.tools.clone();
        let policy = self.policy.clone();
        let observer = self.observer.clone();

        let results: Vec<Result<(usize, ToolResult), AgentError>> =
            stream::iter(calls.iter().cloned().enumerate().map(|(idx, call)| {
                let tools = tools.clone();
                let policy = policy.clone();
                let observer = observer.clone();
                async move {
                    let r = execute_tool_call_with_policy_and_observer(
                        &tools, &policy, &observer, &call,
                    )
                    .await?;
                    Ok((idx, r))
                }
            }))
            .buffer_unordered(max)
            .collect()
            .await;

        let mut ok: Vec<(usize, ToolResult)> = Vec::with_capacity(results.len());
        for r in results {
            ok.push(r?);
        }
        ok.sort_by_key(|(idx, _)| *idx);
        Ok(ok.into_iter().map(|(_, v)| v).collect())
    }

    /// 流式执行
    pub fn run_stream(
        &self,
        input: AgentInput,
    ) -> BoxStream<'static, Result<ChannelEvent, AgentError>> {
        // 简化实现：先将文本输入转换为消息
        let mut messages = vec![ChatMessage::user(input.text)];
        if let Some(ref prompt) = self.system_prompt {
            messages.insert(0, ChatMessage::system(prompt.clone()));
        }

        let provider = self.provider.clone();
        let tools = self.tools.clone();
        let policy = self.policy.clone();
        let observer = self.observer.clone();
        let max_steps = self.config.max_steps;
        let max_tool_concurrency = self.config.max_tool_concurrency;
        let model = self.model.clone();

        let stream = async_stream::try_stream! {
            let tool_defs = tools.definitions();

            info!(
                tool_count = tool_defs.len(),
                max_steps,
                max_tool_concurrency,
                "stream_runtime.start"
            );

            for step in 0..max_steps {
                debug!(step, messages_len = messages.len(), "stream_runtime.step.start");

                let request = ChatRequest {
                    messages: messages.clone(),
                    model: Some(model.clone()),
                    tools: Some(tool_defs.clone()),
                    temperature: None,
                    max_tokens: None,
                    response_format: None,
                    metadata: None,
                    top_p: None,
                    top_k: None,
                    frequency_penalty: None,
                    presence_penalty: None,
                    stop: None,
                    extra: None,
                };

                let mut assistant_text = String::new();
                let mut tool_calls: Vec<ToolCall> = Vec::new();

                let mut s = match provider.stream_chat(request) {
                    Ok(v) => v,
                    Err(e) => {
                        let diag = e.diagnostic();
                        let err = AgentError::Message(format!("provider error ({}): {}", diag.kind, diag.message));
                        let ev = ChannelEvent::Error(ErrorEvent {
                            kind: "provider".to_string(),
                            message: err.to_string(),
                            data: Some(json!({"step": step})),
                        });
                        observer.on_event(ev.clone());
                        yield ev;
                        break;
                    }
                };

                while let Some(item) = s.next().await {
                    let chunk = match item {
                        Ok(v) => v,
                        Err(e) => {
                            let diag = e.diagnostic();
                            let err = AgentError::Message(format!("provider error ({}): {}", diag.kind, diag.message));
                            let ev = ChannelEvent::Error(ErrorEvent {
                                kind: "provider".to_string(),
                                message: err.to_string(),
                                data: Some(json!({"step": step})),
                            });
                            observer.on_event(ev.clone());
                            yield ev;
                            break;
                        }
                    };

                    if let Some(delta) = chunk.delta {
                        assistant_text.push_str(&delta);
                        let ev = ChannelEvent::TokenDelta(TokenDeltaEvent { delta });
                        observer.on_event(ev.clone());
                        yield ev;
                    }

                    if !chunk.tool_calls.is_empty() {
                        tool_calls.extend(chunk.tool_calls);
                    }
                }

                let assistant_msg = ChatMessage {
                    role: Role::Assistant,
                    content: assistant_text,
                    name: None,
                };

                messages.push(assistant_msg.clone());
                let ev = ChannelEvent::Message(assistant_msg);
                observer.on_event(ev.clone());
                yield ev;

                if tool_calls.is_empty() {
                    let ev = ChannelEvent::Debug(DebugEvent {
                        message: "step.end(no_tool_calls)".to_string(),
                        data: Some(json!({"step": step})),
                    });
                    observer.on_event(ev.clone());
                    yield ev;
                    break;
                }

                info!(
                    step,
                    tool_call_count = tool_calls.len(),
                    "stream_runtime.tool_calls"
                );

                let max = max_tool_concurrency.max(1);
                let results: Vec<Result<(usize, ToolResult), AgentError>> = stream::iter(
                    tool_calls
                        .iter()
                        .cloned()
                        .enumerate()
                        .map(|(idx, call)| {
                            let tools = tools.clone();
                            let policy = policy.clone();
                            let observer = observer.clone();
                            async move {
                                let r = execute_tool_call_with_policy_and_observer(
                                    &tools, &policy, &observer, &call,
                                )
                                .await?;
                                Ok((idx, r))
                            }
                        }),
                )
                .buffer_unordered(max)
                .collect()
                .await;

                let mut ok: Vec<(usize, ToolResult)> = Vec::with_capacity(results.len());
                for r in results {
                    match r {
                        Ok(v) => ok.push(v),
                        Err(e) => {
                            let ev = ChannelEvent::Error(ErrorEvent {
                                kind: "tool".to_string(),
                                message: e.to_string(),
                                data: Some(json!({"step": step})),
                            });
                            observer.on_event(ev.clone());
                            yield ev;
                            break;
                        }
                    }
                }
                ok.sort_by_key(|(idx, _)| *idx);

                for (idx, result) in ok.into_iter() {
                    let call = &tool_calls[idx];

                    let ev = ChannelEvent::ToolCall(call.clone());
                    observer.on_event(ev.clone());
                    yield ev;

                    let ev = ChannelEvent::ToolResult(result.clone());
                    observer.on_event(ev.clone());
                    yield ev;

                    let tool_msg = tool_result_to_message(&result, &call.name);
                    messages.push(tool_msg);
                }

                let ev = ChannelEvent::Debug(DebugEvent {
                    message: "step.end".to_string(),
                    data: Some(json!({"step": step, "tool_calls": tool_calls.len()})),
                });
                observer.on_event(ev.clone());
                yield ev;
            }

            info!("stream_runtime.done");
        };

        Box::pin(stream)
    }

    /// 使用 Agent 运行运行时。
    ///
    /// 这是 Runtime 的高级用法，允许传入自定义的 Agent 实现。
    /// Runtime 负责执行 Agent 的决策（调用 LLM、执行工具）。
    ///
    /// # 参数
    ///
    /// * `agent` - Agent 实例
    /// * `input` - Agent 输入
    ///
    /// # 返回
    ///
    /// 返回 `AgentOutput` 包含执行结果
    pub async fn run_with_agent<A>(
        &self,
        agent: &A,
        input: impl Into<AgentInput>,
    ) -> Result<AgentOutput, AgentError>
    where
        A: Agent + ?Sized,
    {
        let input = input.into();
        let mut context = AgentContext::new(input.clone(), self.config.max_steps);

        // 添加系统提示词
        if let Some(ref prompt) = self.system_prompt {
            context.add_message(ChatMessage::system(prompt.clone()));
        }

        info!(
            agent.name = agent.name(),
            max_steps = self.config.max_steps,
            tool_count = self.tools.enabled_len(),
            "runtime.run_with_agent.start"
        );

        let mut tool_call_history = Vec::new();

        // 运行循环：思考 → 执行 → 观察
        loop {
            // 1. Agent 思考
            let decision = agent.think(&context).await;
            debug!(decision = ?decision, "agent.think");

            match decision {
                AgentDecision::Chat { request } => {
                    // 2. 调用 LLM
                    let response = self.provider.chat(request).await.map_err(|e| {
                        let diag = e.diagnostic();
                        AgentError::Message(format!(
                            "provider error ({}): {}",
                            diag.kind, diag.message
                        ))
                    })?;
                    context.add_message(response.message.clone());

                    // 检查是否有工具调用
                    if !response.tool_calls.is_empty() {
                        // 执行工具调用
                        let tool_results = self.execute_tool_calls(&response.tool_calls).await?;

                        // 添加工具结果到上下文
                        for (call, result) in response.tool_calls.iter().zip(tool_results.iter()) {
                            context.add_tool_result(call.name.clone(), result.output.clone());
                            tool_call_history.push(ToolCallRecord {
                                name: call.name.clone(),
                                input: call.input.clone(),
                                result: result.output.clone(),
                            });
                        }
                    } else {
                        // 无工具调用，返回结果
                        return Ok(AgentOutput::with_history(
                            json!({"content": response.message.content}),
                            context.messages,
                            tool_call_history,
                        ));
                    }
                }
                AgentDecision::ToolCall { name, input } => {
                    // 直接调用工具
                    let result = self
                        .tools
                        .call_tool(&name, input.clone())
                        .await
                        .map_err(|e| AgentError::Message(format!("tool error: {}", e)))?;
                    context.add_tool_result(name.clone(), result.clone());
                    tool_call_history.push(ToolCallRecord {
                        name,
                        input,
                        result: result.clone(),
                    });
                }
                AgentDecision::Return(value) => {
                    // 直接返回
                    return Ok(AgentOutput::with_history(
                        value,
                        context.messages,
                        tool_call_history,
                    ));
                }
                AgentDecision::ThinkAgain => {
                    context.step += 1;
                    if context.step >= context.max_steps {
                        return Err(AgentError::Message("达到最大步骤数限制".to_string()));
                    }
                    continue;
                }
                AgentDecision::Stop => {
                    return Ok(AgentOutput::with_history(
                        json!({}),
                        context.messages,
                        tool_call_history,
                    ));
                }
            }

            // 检查步数
            context.step += 1;
            if context.step >= context.max_steps {
                return Err(AgentError::Message("达到最大步骤数限制".to_string()));
            }
        }
    }
}

#[async_trait]
impl Runtime for DefaultRuntime {
    async fn run(&self, input: AgentInput) -> Result<AgentOutput, AgentError> {
        info!(
            max_steps = self.config.max_steps,
            tool_count = self.tools.enabled_len(),
            "runtime.run.start"
        );

        // 将文本输入转换为消息
        let mut messages = vec![ChatMessage::user(input.text)];
        if let Some(ref prompt) = self.system_prompt {
            messages.insert(0, ChatMessage::system(prompt.clone()));
        }

        let tool_defs = self.tools.definitions();
        let mut tool_results: Vec<ToolResult> = Vec::new();

        for step in 0..self.config.max_steps {
            debug!(step, messages_len = messages.len(), "runtime.step.start");

            let request = ChatRequest {
                messages: messages.clone(),
                model: Some(self.model.clone()),
                tools: Some(tool_defs.clone()),
                temperature: None,
                max_tokens: None,
                response_format: None,
                metadata: None,
                top_p: None,
                top_k: None,
                frequency_penalty: None,
                presence_penalty: None,
                stop: None,
                extra: None,
            };

            let resp = self.provider.chat(request).await.map_err(|e| {
                let diag = e.diagnostic();
                AgentError::Message(format!("provider error ({}): {}", diag.kind, diag.message))
            })?;

            messages.push(resp.message.clone());

            if resp.tool_calls.is_empty() {
                info!(
                    step,
                    tool_results_len = tool_results.len(),
                    "runtime.run.done"
                );
                return Ok(AgentOutput::with_history(
                    json!({"content": resp.message.content}),
                    messages,
                    Vec::new(),
                ));
            }

            let results = self.execute_tool_calls(&resp.tool_calls).await?;
            for (idx, result) in results.into_iter().enumerate() {
                let call = &resp.tool_calls[idx];
                tool_results.push(result.clone());

                self.emit(ChannelEvent::ToolCall(call.clone()));
                self.emit(ChannelEvent::ToolResult(result.clone()));

                let tool_msg = tool_result_to_message(&result, call.name.as_str());
                messages.push(tool_msg);
            }

            self.emit(ChannelEvent::Debug(DebugEvent {
                message: "step.end".to_string(),
                data: Some(json!({"step": step, "tool_calls": resp.tool_calls.len()})),
            }));
        }

        warn!(
            max_steps = self.config.max_steps,
            tool_results_len = tool_results.len(),
            "runtime.run.max_steps_exceeded"
        );

        Err(AgentError::Message(format!(
            "超过最大步数限制（max_steps={}），仍未结束工具调用流程",
            self.config.max_steps
        )))
    }
}

/// 运行时构建器
///
/// # 使用示例
///
/// ```rust,no_run
/// use std::sync::Arc;
/// use agentkit_runtime::{DefaultRuntimeBuilder, ToolRegistry};
/// use agentkit_core::provider::LlmProvider;
///
/// # async fn example(provider: Arc<dyn LlmProvider>) -> Result<(), Box<dyn std::error::Error>> {
/// // 必须指定 model
/// let runtime = DefaultRuntimeBuilder::new()
///     .provider(provider)
///     .tools(ToolRegistry::new())
///     .model("gpt-4o-mini")  // ← 必须设置
///     .system_prompt("你是一个有用的助手")
///     .build()?;
/// # Ok(())
/// # }
/// ```
pub struct DefaultRuntimeBuilder {
    provider: Option<Arc<dyn LlmProvider>>,
    tools: ToolRegistry,
    system_prompt: Option<String>,
    model: Option<String>,
    policy: Option<Arc<dyn ToolPolicy>>,
    observer: Option<Arc<dyn RuntimeObserver>>,
    config: Option<RuntimeConfig>,
}

impl DefaultRuntimeBuilder {
    pub fn new() -> Self {
        Self {
            provider: None,
            tools: ToolRegistry::new(),
            system_prompt: None,
            model: None,
            policy: None,
            observer: None,
            config: None,
        }
    }

    pub fn provider(mut self, provider: Arc<dyn LlmProvider>) -> Self {
        self.provider = Some(provider);
        self
    }

    pub fn tools(mut self, tools: ToolRegistry) -> Self {
        self.tools = tools;
        self
    }

    pub fn add_tool<T: agentkit_core::tool::Tool + 'static>(mut self, tool: T) -> Self {
        self.tools = self.tools.register(tool);
        self
    }

    pub fn add_tools(mut self, tools: ToolRegistry) -> Self {
        self.tools = self.tools.merge(tools);
        self
    }

    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// 设置默认模型（必须调用）
    ///
    /// # 参数
    ///
    /// - `model`: 默认使用的模型名称
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use std::sync::Arc;
    /// use agentkit_runtime::DefaultRuntimeBuilder;
    /// use agentkit_core::provider::LlmProvider;
    ///
    /// # async fn example(provider: Arc<dyn LlmProvider>) -> Result<(), Box<dyn std::error::Error>> {
    /// let runtime = DefaultRuntimeBuilder::new()
    ///     .provider(provider)
    ///     .model("gpt-4o-mini")  // ← 必须设置
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn policy(mut self, policy: Arc<dyn ToolPolicy>) -> Self {
        self.policy = Some(policy);
        self
    }

    pub fn observer(mut self, observer: Arc<dyn RuntimeObserver>) -> Self {
        self.observer = Some(observer);
        self
    }

    pub fn config(mut self, config: RuntimeConfig) -> Self {
        self.config = Some(config);
        self
    }

    pub fn max_steps(mut self, max: usize) -> Self {
        if let Some(ref mut config) = self.config {
            config.max_steps = max;
        } else {
            self.config = Some(RuntimeConfig::new("placeholder").with_max_steps(max));
        }
        self
    }

    pub fn max_tool_concurrency(mut self, max: usize) -> Self {
        if let Some(ref mut config) = self.config {
            config.max_tool_concurrency = max.max(1);
        } else {
            self.config = Some(RuntimeConfig::new("placeholder").with_max_tool_concurrency(max));
        }
        self
    }

    pub fn build(self) -> Result<DefaultRuntime, AgentError> {
        let provider = self
            .provider
            .ok_or_else(|| AgentError::Message("必须提供 LlmProvider".to_string()))?;

        let model = self
            .model
            .ok_or_else(|| AgentError::Message("必须指定 model".to_string()))?;

        let runtime = if let Some(config) = self.config {
            DefaultRuntime::with_config(provider, self.tools, config)
        } else {
            DefaultRuntime::new(provider, self.tools, model.clone())
        };

        let mut runtime = runtime;

        if let Some(prompt) = self.system_prompt {
            runtime = runtime.with_system_prompt(prompt);
        }

        if let Some(policy) = self.policy {
            runtime = runtime.with_policy(policy);
        }

        if let Some(observer) = self.observer {
            runtime = runtime.with_observer(observer);
        }

        Ok(runtime)
    }
}
