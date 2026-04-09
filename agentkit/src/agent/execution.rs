//! 共享执行能力模块
//!
//! # 概述
//!
//! `DefaultExecution` 内聚了所有 Runtime 的执行能力，包括：
//! - 非流式执行（run）
//! - 流式执行（run_stream）
//! - 工具调用循环
//! - 并发控制
//! - 策略检查
//! - 观测器协议
//!
//! 所有 Agent 类型都可以组合此结构来获得执行能力。

use std::sync::Arc;

use async_stream::try_stream;
use futures_util::{StreamExt, stream, stream::BoxStream};
use serde_json::{Value, json};
use tokio::sync::Mutex;
use tracing::{debug, info};

use agentkit_core::agent::{
    Agent, AgentContext, AgentDecision, AgentError, AgentInput, AgentOutput, ToolCallRecord,
};

use crate::conversation::ConversationManager;

// 直接导入 agent 模块中的类型
use crate::agent::policy::{DefaultToolPolicy, ToolPolicy};
use crate::agent::tool_execution::{
    execute_tool_call_with_middleware, execute_tool_call_with_policy_and_observer,
    tool_result_to_message,
};
use crate::agent::tool_registry::ToolRegistry;
use crate::middleware::MiddlewareChain;

// 导入 agentkit_core 类型
use agentkit_core::channel::types::{ChannelEvent, ErrorEvent, TokenDeltaEvent};
use agentkit_core::channel::{ChannelObserver, NoopChannelObserver};
use agentkit_core::error::DiagnosticError;
use agentkit_core::provider::LlmProvider;
use agentkit_core::provider::types::{ChatMessage, ChatRequest, Role};
use agentkit_core::tool::types::{ToolCall, ToolResult};

/// 默认执行实现（内聚所有 Runtime 能力）
///
/// 此结构体封装了所有执行相关的逻辑，包括：
/// - 工具调用循环
/// - 流式执行
/// - 并发控制
/// - 策略检查
/// - 观测器协议
///
/// # 使用方式
///
/// Agent 通过组合此结构来获得执行能力：
///
/// ```rust,no_run
/// pub struct MyAgent<P> {
///     provider: P,
///     execution: DefaultExecution,
///     // ... 其他字段
/// }
///
/// impl<P> Agent for MyAgent<P> {
///     fn execution(&self) -> &DefaultExecution {
///         &self.execution
///     }
/// }
/// ```
pub struct DefaultExecution {
    /// LLM Provider
    pub(crate) provider: Arc<dyn LlmProvider>,
    /// 默认使用的模型
    pub(crate) model: String,
    /// 系统提示词
    pub(crate) system_prompt: Option<String>,
    /// 工具注册表
    pub(crate) tools: ToolRegistry,
    /// 工具策略
    pub(crate) policy: Arc<dyn ToolPolicy>,
    /// 观测器
    pub(crate) observer: Arc<dyn ChannelObserver>,
    /// 最大执行步数
    pub(crate) max_steps: usize,
    /// 工具并发执行数
    pub(crate) max_tool_concurrency: usize,
    /// 是否启用工具日志
    pub(crate) enable_tool_logging: bool,
    /// 对话管理器（可选）
    pub(crate) conversation_manager: Option<Arc<Mutex<ConversationManager>>>,
    /// 中间件链
    pub(crate) middleware_chain: MiddlewareChain,
}

impl DefaultExecution {
    /// 创建新的执行实例
    ///
    /// # 参数
    ///
    /// - `provider`: LLM Provider
    /// - `model`: 默认使用的模型名称
    /// - `tools`: 工具注册表
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use agentkit::agent::execution::{DefaultExecution, ToolRegistry};
    /// use agentkit::provider::OpenAiProvider;
    /// use agentkit::tools::ShellTool;
    ///
    /// let provider = OpenAiProvider::from_env()?;
    /// let tools = ToolRegistry::new().register(ShellTool);
    /// let execution = DefaultExecution::new(
    ///     Arc::new(provider),
    ///     "gpt-4o-mini",
    ///     tools,
    /// );
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(
        provider: Arc<dyn LlmProvider>,
        model: impl Into<String>,
        tools: ToolRegistry,
    ) -> Self {
        let model = model.into();
        Self {
            provider,
            model: model.clone(),
            system_prompt: None,
            tools,
            policy: Arc::new(DefaultToolPolicy::new()),
            observer: Arc::new(NoopChannelObserver),
            max_steps: 10,
            max_tool_concurrency: 1,
            enable_tool_logging: true,
            conversation_manager: None,
            middleware_chain: MiddlewareChain::new(),
        }
    }

    /// 设置系统提示词
    pub fn with_system_prompt(mut self, system_prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(system_prompt.into());
        self
    }

    /// 设置系统提示词（可选）
    pub fn with_system_prompt_opt(mut self, system_prompt: Option<String>) -> Self {
        self.system_prompt = system_prompt;
        self
    }

    /// 设置工具策略
    pub fn with_policy(mut self, policy: Arc<dyn ToolPolicy>) -> Self {
        self.policy = policy;
        self
    }

    /// 设置观测器
    pub fn with_observer(mut self, observer: Arc<dyn ChannelObserver>) -> Self {
        self.observer = observer;
        self
    }

    /// 设置最大步数
    pub fn with_max_steps(mut self, max_steps: usize) -> Self {
        self.max_steps = max_steps;
        self
    }

    /// 设置最大工具并发数
    pub fn with_max_tool_concurrency(mut self, max_concurrency: usize) -> Self {
        self.max_tool_concurrency = max_concurrency.max(1);
        self
    }

    /// 设置是否启用工具日志
    pub fn with_tool_logging(mut self, enable_tool_logging: bool) -> Self {
        self.enable_tool_logging = enable_tool_logging;
        self
    }

    /// 设置对话管理器
    pub fn with_conversation_manager(
        mut self,
        conversation_manager: Option<Arc<Mutex<ConversationManager>>>,
    ) -> Self {
        self.conversation_manager = conversation_manager;
        self
    }

    /// 设置中间件链
    pub fn with_middleware_chain(mut self, middleware_chain: MiddlewareChain) -> Self {
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

    /// 构建初始消息列表
    pub(crate) fn build_messages(&self, input: &AgentInput) -> Vec<ChatMessage> {
        let mut messages = Vec::new();

        // 添加系统提示词
        if let Some(ref prompt) = self.system_prompt {
            messages.push(ChatMessage::system(prompt.clone()));
        }

        // 如果启用了对话历史，从历史中获取消息
        if let Some(ref conv_arc) = self.conversation_manager {
            let conv = futures_executor::block_on(conv_arc.lock());
            messages.extend(conv.get_messages().to_vec());
        }

        // 添加用户消息
        messages.push(ChatMessage::user(input.text.clone()));

        messages
    }

    /// 非流式执行
    ///
    /// # 参数
    ///
    /// - `agent`: Agent 实例（用于决策）
    /// - `input`: 用户输入
    ///
    /// # 返回
    ///
    /// 返回 `AgentOutput` 包含执行结果
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use agentkit::agent::execution::DefaultExecution;
    /// use agentkit_core::agent::{Agent, AgentInput};
    ///
    /// # async fn example(execution: &DefaultExecution, agent: &dyn Agent) -> Result<(), Box<dyn std::error::Error>> {
    /// let output = execution.run(agent, AgentInput::new("你好")).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run(
        &self,
        agent: &dyn Agent,
        input: AgentInput,
    ) -> Result<AgentOutput, AgentError> {
        let result = self._run_loop(agent, input).await;

        // 处理错误情况的中间件钩子
        match result {
            Ok(output) => Ok(output),
            Err(mut e) => {
                let middleware_result = self.middleware_chain.process_error(&mut e).await;

                // 如果中间件处理成功，返回修改后的错误
                // 如果中间件处理失败，返回原始错误
                match middleware_result {
                    Ok(_) => Err(e),
                    Err(_) => Err(e),
                }
            }
        }
    }

    /// 内部实现：运行循环
    async fn _run_loop(
        &self,
        agent: &dyn Agent,
        mut input: AgentInput,
    ) -> Result<AgentOutput, AgentError> {
        // 执行请求前中间件钩子
        self.middleware_chain
            .process_request(&mut input)
            .await
            .map_err(|e| AgentError::Message(format!("中间件处理失败：{}", e)))?;

        let mut messages = self.build_messages(&input);
        let mut tool_call_records = Vec::new();
        let mut step = 0;

        info!(
            agent.name = agent.name(),
            max_steps = self.max_steps,
            tool_count = self.tools.enabled_len(),
            "execution.run.start"
        );

        loop {
            if step >= self.max_steps {
                return Err(AgentError::MaxStepsExceeded {
                    max_steps: self.max_steps,
                });
            }

            // 创建上下文
            let context = AgentContext {
                input: input.clone(),
                messages: messages.clone(),
                tool_results: Vec::new(),
                step,
                max_steps: self.max_steps,
            };

            // 1. Agent 思考
            let decision = agent.think(&context).await;
            debug!(decision = ?decision, "agent.think");

            match decision {
                AgentDecision::Chat { request } => {
                    // 2. 调用 LLM
                    let response = self.provider.chat(*request).await.map_err(|e| {
                        let diag = e.diagnostic();
                        AgentError::Message(format!(
                            "provider error ({}): {}",
                            diag.kind, diag.message
                        ))
                    })?;

                    messages.push(response.message.clone());

                    // 3. 检查工具调用
                    if response.tool_calls.is_empty() {
                        // 无工具调用，返回最终结果
                        let mut output = Ok(AgentOutput::with_history(
                            json!({"content": response.message.content}),
                            messages.clone(),
                            tool_call_records.clone(),
                        ));

                        // 执行响应后中间件钩子
                        if let Ok(ref mut out) = output {
                            self.middleware_chain
                                .process_response(out)
                                .await
                                .map_err(|e| {
                                    AgentError::Message(format!("中间件响应处理失败：{}", e))
                                })?;
                        }

                        // 如果启用了对话历史，保存消息
                        if let Some(ref conv_arc) = self.conversation_manager {
                            let mut conv = conv_arc.lock().await;
                            conv.add_user_message(input.text.clone());
                            conv.add_assistant_message(response.message.content.clone());
                        }

                        info!("execution.run.done");
                        return output;
                    }

                    // 4. 执行工具调用
                    let _tool_results = self
                        ._execute_tool_calls(
                            &response.tool_calls,
                            &mut messages,
                            &mut tool_call_records,
                        )
                        .await?;

                    step += 1;
                }
                AgentDecision::ToolCall {
                    name,
                    input: tool_input,
                } => {
                    // 直接工具调用
                    self._execute_direct_tool(
                        &name,
                        tool_input,
                        &mut messages,
                        &mut tool_call_records,
                    )
                    .await?;
                    step += 1;
                }
                AgentDecision::Return(value) => {
                    info!("execution.run.done (Return)");
                    let mut output = Ok(AgentOutput::with_history(
                        value,
                        messages,
                        tool_call_records,
                    ));

                    // 执行响应后中间件钩子
                    if let Ok(ref mut out) = output {
                        self.middleware_chain
                            .process_response(out)
                            .await
                            .map_err(|e| {
                                AgentError::Message(format!("中间件响应处理失败：{}", e))
                            })?;
                    }

                    return output;
                }
                AgentDecision::Stop => {
                    info!("execution.run.done (Stop)");
                    let mut output = Ok(AgentOutput::with_history(
                        Value::Null,
                        messages,
                        tool_call_records,
                    ));

                    // 执行响应后中间件钩子
                    if let Ok(ref mut out) = output {
                        self.middleware_chain
                            .process_response(out)
                            .await
                            .map_err(|e| {
                                AgentError::Message(format!("中间件响应处理失败：{}", e))
                            })?;
                    }

                    return output;
                }
                AgentDecision::ThinkAgain => {
                    step += 1;
                }
            }
        }
    }

    /// 执行工具调用列表
    async fn _execute_tool_calls(
        &self,
        calls: &[ToolCall],
        messages: &mut Vec<ChatMessage>,
        records: &mut Vec<ToolCallRecord>,
    ) -> Result<Vec<ToolResult>, AgentError> {
        let max = self.max_tool_concurrency.max(1);

        if max == 1 || calls.len() == 1 {
            // 串行执行
            let mut results = Vec::with_capacity(calls.len());
            for call in calls {
                let result = execute_tool_call_with_middleware(
                    &self.tools,
                    &self.policy,
                    &self.observer,
                    call,
                    &self.middleware_chain,
                )
                .await
                .map_err(|e| AgentError::Message(format!("工具执行失败：{}", e)))?;
                results.push(result.clone());

                // 添加到记录
                records.push(ToolCallRecord {
                    name: call.name.clone(),
                    input: call.input.clone(),
                    result: result.output.clone(),
                });

                // 添加到消息历史
                messages.push(tool_result_to_message(&result, &call.name));
            }
            return Ok(results);
        }

        // 并发执行
        let results: Vec<Result<(usize, ToolResult), AgentError>> =
            stream::iter(calls.iter().cloned().enumerate().map(|(idx, call)| {
                let tools = self.tools.clone();
                let policy = self.policy.clone();
                let observer = self.observer.clone();
                let middleware_chain = self.middleware_chain.clone();
                async move {
                    let r = execute_tool_call_with_middleware(
                        &tools,
                        &policy,
                        &observer,
                        &call,
                        &middleware_chain,
                    )
                    .await
                    .map_err(|e| AgentError::Message(format!("工具执行失败：{}", e)))?;
                    Ok((idx, r))
                }
            }))
            .buffer_unordered(max)
            .collect()
            .await;

        // 收集结果
        let mut ok: Vec<(usize, ToolResult)> = Vec::with_capacity(results.len());
        for r in results {
            let (idx, result) =
                r.map_err(|e| AgentError::Message(format!("工具执行失败：{}", e)))?;
            ok.push((idx, result.clone()));

            // 添加到记录
            records.push(ToolCallRecord {
                name: calls[idx].name.clone(),
                input: calls[idx].input.clone(),
                result: result.output.clone(),
            });

            // 添加到消息历史
            messages.push(tool_result_to_message(&result, &calls[idx].name));
        }
        ok.sort_by_key(|(idx, _)| *idx);
        Ok(ok.into_iter().map(|(_, v)| v).collect())
    }

    /// 执行单个工具调用
    async fn _execute_direct_tool(
        &self,
        name: &str,
        tool_input: Value,
        messages: &mut Vec<ChatMessage>,
        records: &mut Vec<ToolCallRecord>,
    ) -> Result<(), AgentError> {
        let tool_call_id = format!("local_call_{}", name);
        let call = ToolCall {
            id: tool_call_id.clone(),
            name: name.to_string(),
            input: tool_input.clone(),
        };

        let result = execute_tool_call_with_middleware(
            &self.tools,
            &self.policy,
            &self.observer,
            &call,
            &self.middleware_chain,
        )
        .await
        .map_err(|e| AgentError::Message(format!("工具执行失败：{}", e)))?;

        records.push(ToolCallRecord {
            name: name.to_string(),
            input: tool_input.clone(),
            result: result.output.clone(),
        });

        messages.push(tool_result_to_message(&result, name));

        Ok(())
    }

    /// 流式执行
    ///
    /// 注意：此方法不支持 Agent 决策，只执行简单的工具调用循环。
    /// 如需使用 Agent 决策，请使用 `run` 方法。
    ///
    /// # 参数
    ///
    /// - `input`: 用户输入
    ///
    /// # 返回
    ///
    /// 返回事件流
    pub fn run_stream_simple(
        &self,
        input: AgentInput,
    ) -> BoxStream<'static, Result<ChannelEvent, AgentError>> {
        let provider = self.provider.clone();
        let tools = self.tools.clone();
        let policy = self.policy.clone();
        let observer = self.observer.clone();
        let max_steps = self.max_steps;
        let max_tool_concurrency = self.max_tool_concurrency;
        let model = self.model.clone();
        let system_prompt = self.system_prompt.clone();

        let stream = try_stream! {
            let mut messages = Vec::new();

            // 添加系统提示词
            if let Some(ref prompt) = system_prompt {
                messages.push(ChatMessage::system(prompt.clone()));
            }

            // 添加用户消息
            messages.push(ChatMessage::user(input.text.clone()));

            let tool_defs = tools.definitions();

            info!(
                tool_count = tool_defs.len(),
                max_steps,
                max_tool_concurrency,
                "stream_execution.start"
            );

            for step in 0..max_steps {
                let request = ChatRequest {
                    messages: messages.clone(),
                    model: Some(model.clone()),
                    tools: if !tool_defs.is_empty() { Some(tool_defs.clone()) } else { None },
                    temperature: Some(0.7),
                    ..Default::default()
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
                    break;
                }

                info!(
                    step,
                    tool_call_count = tool_calls.len(),
                    "stream_execution.tool_calls"
                );

                // 执行工具调用
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
                                .await
                                .map_err(|e| AgentError::Message(format!("工具执行失败：{}", e)))?;
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
            }

            info!("stream_execution.done");
        };

        Box::pin(stream)
    }
}
