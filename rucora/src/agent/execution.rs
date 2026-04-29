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

use rucora_core::agent::{
    Agent, AgentContext, AgentDecision, AgentError, AgentInput, AgentOutput, ToolCallRecord,
};

use crate::conversation::ConversationManager;

// 直接导入 agent 模块中的类型
use crate::agent::loop_detector::{LoopDetectionResult, LoopDetector, LoopDetectorConfig};
use crate::agent::policy::{DefaultToolPolicy, ToolPolicy};
use crate::agent::tool_call_config::{ToolCallEnhancedConfig, ToolCallEnhancedRuntime};
use crate::agent::tool_execution::{
    execute_tool_call_enhanced, execute_tool_call_with_policy_and_observer, tool_result_to_message,
};
use crate::agent::tool_registry::ToolRegistry;
use crate::middleware::MiddlewareChain;

// ========== 历史消息孤儿清理 ==========

/// 清理消息历史中的孤儿 Tool 消息。
///
/// 当 context 压缩或历史截断后，可能出现没有对应 assistant
/// 配对的 tool 角色消息。部分 LLM Provider（如 Anthropic、MiniMax）
/// 会因此返回 400 错误。
///
/// 本函数扫描历史消息，删除所有孤儿 tool 消息，并返回删除数量。
///
/// 注意：ChatMessage 不包含 tool_calls 字段（tool_calls 在 ChatResponse
/// 中作为独立字段返回），因此本函数基于消息顺序和角色来判定配对关系，
/// 而非检查 content 中是否包含 "tool_calls" 文本。
pub(crate) fn remove_orphaned_tool_messages(messages: &mut Vec<ChatMessage>) -> usize {
    let mut removed = 0usize;
    let mut i = 0;

    while i < messages.len() {
        // 只检查 tool 角色的消息
        if messages[i].role != Role::Tool {
            i += 1;
            continue;
        }

        // 向前查找最近的非-tool 消息
        let parent_idx = (0..i).rev().find(|&j| messages[j].role != Role::Tool);

        let is_orphan = match parent_idx {
            None => true, // 前面没有任何消息，肯定是孤儿
            Some(idx) => {
                // 如果最近的非-tool 消息不是 assistant，则该 tool 消息是孤儿
                messages[idx].role != Role::Assistant
            }
        };

        if is_orphan {
            messages.remove(i);
            removed += 1;
            // 不递增 i，因为删除后当前位置是下一条消息
        } else {
            i += 1;
        }
    }

    if removed > 0 {
        tracing::warn!(
            count = removed,
            "移除了 {removed} 个孤儿 tool 消息（没有对应的 assistant 父消息）"
        );
    }

    removed
}

// ========== Context Overflow 内联恢复 ==========

/// 检测 provider 错误消息是否为 context window 溢出
///
/// 参考实现: zeroclaw `reliable::is_context_window_exceeded`
fn is_context_overflow_error(message: &str) -> bool {
    let m = message.to_ascii_lowercase();
    m.contains("context length")
        || m.contains("context window")
        || m.contains("maximum context")
        || m.contains("token limit")
        || m.contains("too many tokens")
        || m.contains("input too long")
        || m.contains("exceeds the limit")
        || m.contains("reduce the length")
}

/// 快速裁剪旧 tool 消息内容以释放 context 空间。
///
/// 将除最近 `protect_last_n` 条之外的所有 tool 消息内容截断到 2000 字符。
/// 返回节省的总字符数。
///
/// 参考实现: zeroclaw `fast_trim_tool_results`
fn fast_trim_tool_results(messages: &mut [ChatMessage], protect_last_n: usize) -> usize {
    const TRIM_TO: usize = 2000;
    let mut saved = 0;
    let cutoff = messages.len().saturating_sub(protect_last_n);
    for msg in &mut messages[..cutoff] {
        if msg.role == Role::Tool && msg.content.len() > TRIM_TO {
            let original_len = msg.content.len();
            msg.content = truncate_tool_content(&msg.content, TRIM_TO);
            saved += original_len - msg.content.len();
        }
    }
    saved
}

/// 紧急删除最旧的非系统消息（约 1/3 历史）。
///
/// Tool 组（assistant + 紧跟的 tool 消息）作为原子单元整体删除，
/// 保证 tool_calls/tool_results 配对完整性。
/// 返回删除的消息数。
///
/// 参考实现: zeroclaw `emergency_history_trim`
fn emergency_history_trim(messages: &mut Vec<ChatMessage>, keep_recent: usize) -> usize {
    let mut dropped = 0;
    let target_drop = (messages.len() / 3).max(1);
    let mut i = 0;
    while dropped < target_drop && i < messages.len().saturating_sub(keep_recent) {
        if messages[i].role == Role::System {
            i += 1;
        } else if messages[i].role == Role::Assistant {
            // 计算紧跟的 tool 消息数，以原子组删除
            let mut tool_count = 0;
            while i + 1 + tool_count < messages.len().saturating_sub(keep_recent)
                && messages[i + 1 + tool_count].role == Role::Tool
            {
                tool_count += 1;
            }
            for _ in 0..=tool_count {
                messages.remove(i);
                dropped += 1;
            }
        } else {
            messages.remove(i);
            dropped += 1;
        }
    }
    dropped += remove_orphaned_tool_messages(messages);
    dropped
}

/// 裁剪 tool 消息内容，保留头部 2/3 + 尾部 1/3，中间插入省略标记
fn truncate_tool_content(content: &str, max_chars: usize) -> String {
    if content.len() <= max_chars {
        return content.to_string();
    }
    let head_len = max_chars * 2 / 3;
    let tail_len = max_chars.saturating_sub(head_len);
    // 找安全 char boundary
    let head_end = floor_char_boundary(content, head_len);
    let tail_start_raw = content.len().saturating_sub(tail_len);
    let mut tail_start = tail_start_raw;
    while tail_start < content.len() && !content.is_char_boundary(tail_start) {
        tail_start += 1;
    }
    if head_end >= tail_start {
        return content[..floor_char_boundary(content, max_chars)].to_string();
    }
    let truncated = tail_start - head_end;
    format!(
        "{}\n\n[... {truncated} 字符已截断 ...]\n\n{}",
        &content[..head_end],
        &content[tail_start..]
    )
}

/// 找 `<= i` 处最近的 char boundary（MSRV 兼容替代 `str::floor_char_boundary`）
fn floor_char_boundary(s: &str, i: usize) -> usize {
    if i >= s.len() {
        return s.len();
    }
    let mut pos = i;
    while pos > 0 && !s.is_char_boundary(pos) {
        pos -= 1;
    }
    pos
}

// 导入 rucora_core 类型
use rucora_core::channel::types::{ChannelEvent, ErrorEvent, TokenDeltaEvent};
use rucora_core::channel::{ChannelObserver, NoopChannelObserver};
use rucora_core::error::DiagnosticError;
use rucora_core::provider::LlmProvider;
use rucora_core::provider::types::{ChatMessage, ChatRequest, Role, Usage};
use rucora_core::tool::types::{ToolCall, ToolResult};

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
    /// 工具调用增强配置（重试、超时、熔断、缓存等）
    pub(crate) enhanced_config: ToolCallEnhancedConfig,
    /// 工具调用增强运行时状态
    pub(crate) enhanced_runtime: ToolCallEnhancedRuntime,
    /// 循环检测器配置
    pub(crate) loop_detector_config: LoopDetectorConfig,
    /// LLM 请求参数（temperature、top_p 等）
    pub(crate) llm_params: rucora_core::provider::types::LlmParams,
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
    /// use rucora::agent::execution::{DefaultExecution, ToolRegistry};
    /// use rucora::provider::OpenAiProvider;
    /// use rucora::tools::ShellTool;
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
            model,
            system_prompt: None,
            tools,
            policy: Arc::new(DefaultToolPolicy::new()),
            observer: Arc::new(NoopChannelObserver),
            max_steps: 10,
            max_tool_concurrency: 1,
            enable_tool_logging: true,
            conversation_manager: None,
            middleware_chain: MiddlewareChain::new(),
            enhanced_config: ToolCallEnhancedConfig::default(),
            enhanced_runtime: ToolCallEnhancedRuntime::new(),
            loop_detector_config: LoopDetectorConfig::default(),
            llm_params: rucora_core::provider::types::LlmParams::default(),
        }
    }

    /// 设置循环检测器配置
    pub fn with_loop_detector_config(mut self, config: LoopDetectorConfig) -> Self {
        self.loop_detector_config = config;
        self
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

    /// 设置工具调用增强配置（重试、超时、熔断器、缓存等）
    ///
    /// 默认所有增强特性均关闭，通过此方法按需启用。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use rucora::agent::{
    ///     execution::DefaultExecution,
    ///     RetryConfig, TimeoutConfig, ToolCallEnhancedConfig,
    /// };
    /// use std::time::Duration;
    ///
    /// let config = ToolCallEnhancedConfig::new()
    ///     .with_retry(RetryConfig::exponential(3))
    ///     .with_timeout(TimeoutConfig::default_timeout(Duration::from_secs(30)));
    ///
    /// // execution.with_enhanced_config(config)
    /// ```
    pub fn with_enhanced_config(mut self, config: ToolCallEnhancedConfig) -> Self {
        self.enhanced_config = config;
        self
    }

    /// 设置 LLM 请求参数（temperature、top_p、max_tokens 等）
    pub fn with_llm_params(mut self, params: rucora_core::provider::types::LlmParams) -> Self {
        self.llm_params = params;
        self
    }

    /// 构建初始消息列表
    pub(crate) fn build_messages(&self, input: &AgentInput) -> Vec<ChatMessage> {
        let mut messages = Vec::new();

        // 如果启用了对话历史，从历史中获取消息
        if let Some(ref conv_arc) = self.conversation_manager {
            let conv = futures_executor::block_on(conv_arc.lock());
            messages.extend(conv.get_messages().to_vec());
        } else if let Some(ref prompt) = self.system_prompt {
            // 未启用对话历史时，由执行器负责注入系统提示词
            messages.push(ChatMessage::system(prompt.clone()));
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
    /// use rucora::agent::execution::DefaultExecution;
    /// use rucora_core::agent::{Agent, AgentInput};
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

                // 中间件处理结果不影响返回，始终返回原始错误
                // 中间件可用于记录日志或副作用
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
            .map_err(|e| AgentError::Message(format!("中间件处理失败：{e}")))?;

        let mut messages = self.build_messages(&input);
        let mut tool_call_records = Vec::new();
        let mut step = 0;
        let mut total_usage: Option<Usage> = None;
        // 循环检测器（防止 Agent 陷入无限重复调用同一工具）
        let mut loop_detector = LoopDetector::new(self.loop_detector_config.clone());

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

            // 每次迭代前清理孤儿 tool 消息，防止 provider 返回 400 错误
            remove_orphaned_tool_messages(&mut messages);

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
                AgentDecision::Chat { mut request } => {
                    // 2. 自动注入工具定义 (修复库设计缺陷：避免 Agent 必须手动注入)
                    if request.tools.is_none() && self.tools.enabled_len() > 0 {
                        let tool_defs = self.tools.definitions();
                        if !tool_defs.is_empty() {
                            info!(
                                tool_count = tool_defs.len(),
                                "自动向 LLM 注入 {} 个工具定义",
                                tool_defs.len()
                            );
                            request.tools = Some(tool_defs);
                        }
                    }

                    // 3. 调用 LLM（含 context overflow 内联恢复）
                    let response = match self.provider.chat(*request).await {
                        Ok(r) => r,
                        Err(e) => {
                            let diag = e.diagnostic();
                            // 检测 context window 溢出错误
                            if is_context_overflow_error(&diag.message) {
                                tracing::warn!(
                                    step,
                                    "Context window exceeded, attempting in-loop recovery"
                                );
                                // Step 1: 快速裁剪旧 tool 消息（廉价操作）
                                let saved = fast_trim_tool_results(&mut messages, 4);
                                if saved > 0 {
                                    tracing::info!(
                                        chars_saved = saved,
                                        "Context recovery: trimmed old tool results, retrying"
                                    );
                                    continue;
                                }
                                // Step 2: 紧急丢弃最旧的非系统消息
                                let dropped = emergency_history_trim(&mut messages, 4);
                                if dropped > 0 {
                                    tracing::info!(
                                        dropped,
                                        "Context recovery: dropped old messages, retrying"
                                    );
                                    continue;
                                }
                                // 无法恢复
                                tracing::error!(
                                    "Context overflow unrecoverable: no trimmable messages"
                                );
                            }
                            return Err(AgentError::Message(format!(
                                "provider error ({}): {}",
                                diag.kind, diag.message
                            )));
                        }
                    };

                    messages.push(response.message.clone());

                    // 累计 usage
                    if let Some(u) = &response.usage {
                        total_usage = Some(match &total_usage {
                            Some(curr) => Usage {
                                prompt_tokens: curr.prompt_tokens + u.prompt_tokens,
                                completion_tokens: curr.completion_tokens + u.completion_tokens,
                                total_tokens: curr.total_tokens + u.total_tokens,
                            },
                            None => u.clone(),
                        });
                    }

                    // 3. 检查工具调用
                    if response.tool_calls.is_empty() {
                        // 无工具调用，返回最终结果
                        let mut output = Ok(AgentOutput::with_usage(
                            json!({"content": response.message.content}),
                            messages.clone(),
                            tool_call_records.clone(),
                            total_usage,
                        ));

                        // 执行响应后中间件钩子
                        if let Ok(ref mut out) = output {
                            self.middleware_chain
                                .process_response(out)
                                .await
                                .map_err(|e| {
                                    AgentError::Message(format!("中间件响应处理失败：{e}"))
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
                            &mut loop_detector,
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
                        &mut loop_detector,
                    )
                    .await?;
                    step += 1;
                }
                AgentDecision::Return(value) => {
                    info!("execution.run.done (Return)");
                    let mut output = Ok(AgentOutput::with_usage(
                        value,
                        messages,
                        tool_call_records,
                        total_usage,
                    ));

                    // 执行响应后中间件钩子
                    if let Ok(ref mut out) = output {
                        self.middleware_chain
                            .process_response(out)
                            .await
                            .map_err(|e| AgentError::Message(format!("中间件响应处理失败：{e}")))?;
                    }

                    return output;
                }
                AgentDecision::Stop => {
                    info!("execution.run.done (Stop)");
                    let mut output = Ok(AgentOutput::with_usage(
                        Value::Null,
                        messages,
                        tool_call_records,
                        total_usage,
                    ));

                    // 执行响应后中间件钩子
                    if let Ok(ref mut out) = output {
                        self.middleware_chain
                            .process_response(out)
                            .await
                            .map_err(|e| AgentError::Message(format!("中间件响应处理失败：{e}")))?;
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
        loop_detector: &mut LoopDetector,
    ) -> Result<Vec<ToolResult>, AgentError> {
        let max = self.max_tool_concurrency.max(1);

        if max == 1 || calls.len() == 1 {
            // 串行执行
            let mut results = Vec::with_capacity(calls.len());
            for call in calls {
                // 使用增强执行（含重试、超时、熔断、缓存）
                let result = execute_tool_call_enhanced(
                    &self.tools,
                    &self.policy,
                    &self.observer,
                    call,
                    &self.middleware_chain,
                    &self.enhanced_config,
                    &self.enhanced_runtime,
                )
                .await
                .map_err(|e| AgentError::Message(format!("工具执行失败：{e}")))?;

                // 循环检测
                let detection =
                    loop_detector.record(&call.name, &call.input, &result.output.to_string());
                match detection {
                    LoopDetectionResult::Ok => {}
                    LoopDetectionResult::Warning(msg) => {
                        // 注入系统消息提示 LLM 调整策略
                        tracing::warn!(tool = %call.name, "{}", msg);
                        messages.push(ChatMessage::system(msg));
                    }
                    LoopDetectionResult::Block(msg) => {
                        // 用阻止消息替换工具输出，并注入消息
                        tracing::warn!(tool = %call.name, "{}", msg);
                        let blocked_result = ToolResult {
                            tool_call_id: result.tool_call_id.clone(),
                            output: serde_json::Value::String(msg.clone()),
                        };
                        results.push(blocked_result.clone());
                        records.push(ToolCallRecord {
                            name: call.name.clone(),
                            input: call.input.clone(),
                            result: blocked_result.output.clone(),
                        });
                        messages.push(tool_result_to_message(&blocked_result, &call.name));
                        continue;
                    }
                    LoopDetectionResult::Break(msg) => {
                        tracing::error!(tool = %call.name, "{}", msg);
                        return Err(AgentError::Message(format!("[LoopDetector] {msg}")));
                    }
                }

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

        // 并发执行（细粒度并发：按工具名取并发数）
        let results: Vec<Result<(usize, ToolResult), AgentError>> =
            stream::iter(calls.iter().cloned().enumerate().map(|(idx, call)| {
                let tools = self.tools.clone();
                let policy = self.policy.clone();
                let observer = self.observer.clone();
                let middleware_chain = self.middleware_chain.clone();
                let enhanced_config = self.enhanced_config.clone();
                let enhanced_runtime = self.enhanced_runtime.clone();
                // 细粒度并发：取该工具对应的并发数（这里每个任务独立限流，
                // 实际并发上限由 buffer_unordered(max) 控制）
                async move {
                    let r = execute_tool_call_enhanced(
                        &tools,
                        &policy,
                        &observer,
                        &call,
                        &middleware_chain,
                        &enhanced_config,
                        &enhanced_runtime,
                    )
                    .await
                    .map_err(|e| AgentError::Message(format!("工具执行失败：{e}")))?;
                    Ok((idx, r))
                }
            }))
            .buffer_unordered(max)
            .collect()
            .await;

        // 收集结果
        let mut ok: Vec<(usize, ToolResult)> = Vec::with_capacity(results.len());
        for r in results {
            let (idx, result) = r.map_err(|e| AgentError::Message(format!("工具执行失败：{e}")))?;

            // 循环检测（并发路径：串行处理检测结果）
            let detection = loop_detector.record(
                &calls[idx].name,
                &calls[idx].input,
                &result.output.to_string(),
            );
            match detection {
                LoopDetectionResult::Ok => {
                    ok.push((idx, result.clone()));
                    records.push(ToolCallRecord {
                        name: calls[idx].name.clone(),
                        input: calls[idx].input.clone(),
                        result: result.output.clone(),
                    });
                    messages.push(tool_result_to_message(&result, &calls[idx].name));
                }
                LoopDetectionResult::Warning(ref msg) => {
                    tracing::warn!(tool = %calls[idx].name, "{}", msg);
                    ok.push((idx, result.clone()));
                    records.push(ToolCallRecord {
                        name: calls[idx].name.clone(),
                        input: calls[idx].input.clone(),
                        result: result.output.clone(),
                    });
                    messages.push(tool_result_to_message(&result, &calls[idx].name));
                }
                LoopDetectionResult::Block(msg) => {
                    tracing::warn!(tool = %calls[idx].name, "{}", msg);
                    let blocked = ToolResult {
                        tool_call_id: result.tool_call_id.clone(),
                        output: serde_json::Value::String(msg),
                    };
                    ok.push((idx, blocked.clone()));
                    records.push(ToolCallRecord {
                        name: calls[idx].name.clone(),
                        input: calls[idx].input.clone(),
                        result: blocked.output.clone(),
                    });
                    messages.push(tool_result_to_message(&blocked, &calls[idx].name));
                }
                LoopDetectionResult::Break(msg) => {
                    tracing::error!(tool = %calls[idx].name, "{}", msg);
                    return Err(AgentError::Message(format!("[LoopDetector] {msg}")));
                }
            }
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
        loop_detector: &mut LoopDetector,
    ) -> Result<(), AgentError> {
        let tool_call_id = format!("local_call_{name}");
        let call = ToolCall {
            id: tool_call_id.clone(),
            name: name.to_string(),
            input: tool_input.clone(),
        };

        let result = execute_tool_call_enhanced(
            &self.tools,
            &self.policy,
            &self.observer,
            &call,
            &self.middleware_chain,
            &self.enhanced_config,
            &self.enhanced_runtime,
        )
        .await
        .map_err(|e| AgentError::Message(format!("工具执行失败：{e}")))?;

        // 循环检测
        let detection = loop_detector.record(name, &tool_input, &result.output.to_string());
        match detection {
            LoopDetectionResult::Ok => {}
            LoopDetectionResult::Warning(msg) => {
                tracing::warn!(tool = %name, "{}", msg);
                messages.push(ChatMessage::system(msg));
            }
            LoopDetectionResult::Block(msg) => {
                tracing::warn!(tool = %name, "{}", msg);
                let blocked = ToolResult {
                    tool_call_id: result.tool_call_id,
                    output: serde_json::Value::String(msg),
                };
                records.push(ToolCallRecord {
                    name: name.to_string(),
                    input: tool_input,
                    result: blocked.output.clone(),
                });
                messages.push(tool_result_to_message(&blocked, name));
                return Ok(());
            }
            LoopDetectionResult::Break(msg) => {
                tracing::error!(tool = %name, "{}", msg);
                return Err(AgentError::Message(format!("[LoopDetector] {msg}")));
            }
        }

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
        let llm_params = self.llm_params.clone();

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
                let mut request = ChatRequest {
                    messages: messages.clone(),
                    model: Some(model.clone()),
                    tools: if !tool_defs.is_empty() { Some(tool_defs.clone()) } else { None },
                    ..Default::default()
                };
                llm_params.apply_to(&mut request);

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
                                .map_err(|e| AgentError::Message(format!("工具执行失败：{e}")))?;
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

                for (idx, result) in ok {
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

    /// 高层流式 API：运行并返回拼接后的最终文本。
    ///
    /// 此方法消费事件流，自动拼接 `TokenDelta` 为完整文本返回。
    /// 适用于只需要最终文本、不需要逐帧处理的场景。
    ///
    /// # 参数
    ///
    /// - `input`: 用户输入
    ///
    /// # 返回
    ///
    /// 返回拼接后的完整文本内容。如果遇到错误则返回错误。
    pub async fn run_stream_text(
        &self,
        input: AgentInput,
    ) -> Result<String, rucora_core::agent::AgentError> {
        use futures_util::StreamExt;
        use rucora_core::channel::types::ChannelEvent;

        let mut stream = self.run_stream_simple(input);
        let mut text = String::new();

        while let Some(event) = stream.next().await {
            match event? {
                ChannelEvent::TokenDelta(delta) => {
                    text.push_str(&delta.delta);
                }
                ChannelEvent::Error(err) => {
                    return Err(rucora_core::agent::AgentError::Message(err.message));
                }
                _ => {}
            }
        }

        Ok(text)
    }
}
