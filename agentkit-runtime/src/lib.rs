//! agentkit 的最小运行时（runtime）示例。
//!
//! 该 crate 的职责是提供“编排层”的实现（如何调用 provider、如何循环、如何调用工具等）。
//! 目前仅提供一个最小的 `SimpleAgent`，用于演示如何基于 `agentkit-core` 的 trait 进行组装。

use std::{collections::HashMap, sync::Arc};

use agentkit_core::{
    agent::{
        Agent,
        types::{AgentInput, AgentOutput},
    },
    channel::types::ChannelEvent,
    error::{AgentError, ToolError},
    provider::{
        LlmProvider,
        types::{ChatMessage, ChatRequest, Role},
    },
    runtime::Runtime,
    tool::{
        Tool,
        types::{DEFAULT_TOOL_OUTPUT_MAX_BYTES, ToolCall, ToolDefinition, ToolResult},
    },
};
use async_trait::async_trait;
use futures_util::{StreamExt, stream::BoxStream};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::time::{Duration, sleep, timeout};
use tracing::{debug, info, warn};

/// 将 UTF-8 字符串按“字节数”安全截断。
///
/// - `max_bytes` 是字节上限（不是字符数）。
/// - 截断点会回退到 UTF-8 字符边界，避免产生非法字符串。
/// - 截断后会追加一段提示文本，便于下游识别输出被裁剪。
fn truncate_utf8_to_bytes(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    let mut out = s[..end].to_string();
    out.push_str("\n... [output truncated]");
    out
}

/// 对工具输出/结构化结果应用大小限制。
///
/// 该函数用于避免工具返回内容过大导致：
/// - 消息体膨胀、占用过多 token
/// - 日志/传输层异常
///
/// 行为：
/// - 若序列化后的 JSON 字符串超过 `max_bytes`，会把 payload 转为截断后的字符串
/// - 最终保证返回值是 JSON Object，并附带：
///   - `truncated`: 是否发生截断
///   - `max_bytes`: 本次使用的上限
fn apply_output_limit(payload: Value, max_bytes: usize) -> Value {
    // Always attach the protocol fields.
    let serialized = payload.to_string();
    let truncated = serialized.len() > max_bytes;
    let limited_payload = if truncated {
        Value::String(truncate_utf8_to_bytes(&serialized, max_bytes))
    } else {
        payload
    };

    // Ensure we return a JSON object.
    let mut obj = match limited_payload {
        Value::Object(map) => Value::Object(map),
        other => json!({"value": other}),
    };

    if let Some(map) = obj.as_object_mut() {
        map.insert("truncated".to_string(), Value::Bool(truncated));
        map.insert("max_bytes".to_string(), json!(max_bytes));
    }
    obj
}

#[derive(Debug, Clone)]
/// 单次工具调用的上下文信息。
///
/// 主要用于把 `ToolCall` 传递给策略（policy）做安全检查。
pub struct ToolCallContext {
    pub tool_call: ToolCall,
}

#[async_trait]
/// 工具调用策略（Policy）。
///
/// 用于在执行工具前进行 allow/deny 检查。
///
/// 返回 `Ok(())` 表示允许执行；返回 `Err(ToolError::PolicyDenied{..})`
/// 表示拒绝并携带规则与原因。
pub trait ToolPolicy: Send + Sync {
    async fn check(&self, ctx: &ToolCallContext) -> Result<(), ToolError>;
}

#[derive(Debug, Default, Clone)]
/// 允许所有工具调用的策略（不做任何拦截）。
pub struct AllowAllToolPolicy;

#[async_trait]
impl ToolPolicy for AllowAllToolPolicy {
    async fn check(&self, _ctx: &ToolCallContext) -> Result<(), ToolError> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
/// 审计事件。
///
/// 用于记录工具调用在执行过程中的关键节点（开始/拒绝/错误/完成）。
pub enum AuditEvent {
    ToolCallStart {
        tool_name: String,
        tool_call_id: String,
        input_len: usize,
    },
    ToolCallDenied {
        tool_name: String,
        tool_call_id: String,
        rule_id: String,
        reason: String,
    },
    ToolCallError {
        tool_name: String,
        tool_call_id: String,
        message: String,
    },
    ToolCallDone {
        tool_name: String,
        tool_call_id: String,
        output_len: usize,
        elapsed_ms: u64,
    },
}

/// 审计事件接收器。
///
/// 你可以实现该 trait，把审计事件写入日志、指标系统或数据库。
pub trait AuditSink: Send + Sync {
    fn record(&self, event: AuditEvent);
}

#[derive(Debug, Default, Clone)]
/// 空实现审计接收器（丢弃所有事件）。
pub struct NoopAuditSink;

impl AuditSink for NoopAuditSink {
    fn record(&self, _event: AuditEvent) {}
}

#[derive(Debug, Clone)]
/// Provider 调用重试配置。
///
/// 用于 `ResilientProvider`：在 provider.chat 失败时按指数退避进行重试。
pub struct RetryConfig {
    pub max_retries: usize,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub timeout_ms: Option<u64>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 2,
            base_delay_ms: 200,
            max_delay_ms: 2_000,
            timeout_ms: None,
        }
    }
}

#[derive(Clone, Debug)]
/// 可取消的句柄。
///
/// 目前主要用于 `ResilientProvider::stream_chat_cancellable`：
/// 外部可在需要时中断流式输出。
pub struct CancelHandle {
    cancelled: Arc<AtomicBool>,
}

impl CancelHandle {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

#[derive(Clone)]
/// 一个带重试/超时/可取消能力的 Provider 包装器。
///
/// - 对 `chat()` 提供重试 + 可选超时
/// - 对 `stream_chat()` 保持透传（避免流式重试导致语义重复）
/// - 额外提供 `stream_chat_cancellable()` 以支持外部取消
pub struct ResilientProvider {
    inner: Arc<dyn LlmProvider>,
    cfg: RetryConfig,
}

impl ResilientProvider {
    pub fn new(inner: Arc<dyn LlmProvider>) -> Self {
        Self {
            inner,
            cfg: RetryConfig::default(),
        }
    }

    pub fn with_config(mut self, cfg: RetryConfig) -> Self {
        self.cfg = cfg;
        self
    }

    fn backoff_delay_ms(&self, attempt: usize) -> u64 {
        let pow = 1u64.checked_shl(attempt.min(16) as u32).unwrap_or(u64::MAX);
        let delay = self.cfg.base_delay_ms.saturating_mul(pow);
        delay.min(self.cfg.max_delay_ms)
    }

    pub fn stream_chat_cancellable(
        &self,
        request: ChatRequest,
    ) -> Result<
        (
            CancelHandle,
            BoxStream<
                'static,
                Result<
                    agentkit_core::provider::types::ChatStreamChunk,
                    agentkit_core::error::ProviderError,
                >,
            >,
        ),
        agentkit_core::error::ProviderError,
    > {
        let cancelled = Arc::new(AtomicBool::new(false));
        let handle = CancelHandle {
            cancelled: cancelled.clone(),
        };

        let inner_stream = self.inner.stream_chat(request)?;
        let stream = async_stream::try_stream! {
            futures_util::pin_mut!(inner_stream);
            while let Some(item) = inner_stream.next().await {
                if cancelled.load(Ordering::SeqCst) {
                    break;
                }
                yield item?;
            }
        };

        Ok((handle, Box::pin(stream)))
    }
}

#[async_trait]
impl LlmProvider for ResilientProvider {
    async fn chat(
        &self,
        request: ChatRequest,
    ) -> Result<agentkit_core::provider::types::ChatResponse, agentkit_core::error::ProviderError>
    {
        let mut attempt = 0usize;
        loop {
            let fut = self.inner.chat(request.clone());
            let result = if let Some(ms) = self.cfg.timeout_ms {
                timeout(Duration::from_millis(ms), fut).await.map_err(|_| {
                    agentkit_core::error::ProviderError::Message(format!(
                        "provider chat timeout after {}ms",
                        ms
                    ))
                })?
            } else {
                fut.await
            };

            match result {
                Ok(v) => return Ok(v),
                Err(e) => {
                    if attempt >= self.cfg.max_retries {
                        return Err(e);
                    }
                    let delay = self.backoff_delay_ms(attempt);
                    warn!(attempt, delay_ms = delay, error = %e, "provider.chat.retry");
                    sleep(Duration::from_millis(delay)).await;
                    attempt += 1;
                }
            }
        }
    }

    fn stream_chat(
        &self,
        request: ChatRequest,
    ) -> Result<
        BoxStream<
            'static,
            Result<
                agentkit_core::provider::types::ChatStreamChunk,
                agentkit_core::error::ProviderError,
            >,
        >,
        agentkit_core::error::ProviderError,
    > {
        // 流式重试很容易导致语义重复；这里先只提供 timeout/取消的外部能力。
        self.inner.stream_chat(request)
    }
}

#[derive(Debug, Clone, Default)]
/// 命令执行类工具的 allow/deny 配置。
///
/// 该配置用于 `DefaultToolPolicy`：
/// - `allowed_commands` 非空时，仅允许列表内命令
/// - `denied_commands` 用于显式禁止（优先级更高）
pub struct CommandPolicyConfig {
    pub allowed_commands: Vec<String>,
    pub denied_commands: Vec<String>,
}

impl CommandPolicyConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn allow_command(mut self, cmd: impl Into<String>) -> Self {
        self.allowed_commands.push(cmd.into());
        self
    }

    pub fn deny_command(mut self, cmd: impl Into<String>) -> Self {
        self.denied_commands.push(cmd.into());
        self
    }
}

#[derive(Debug, Clone)]
/// 默认工具策略。
///
/// 目前主要针对两类“命令执行”工具：
/// - `shell`：包含命令与参数
/// - `cmd_exec`：直接执行命令
///
/// 默认会拦截危险命令，并阻止常见 shell 操作符（防止链式/重定向）。
pub struct DefaultToolPolicy {
    shell: CommandPolicyConfig,
    cmd_exec: CommandPolicyConfig,
}

impl DefaultToolPolicy {
    pub fn new() -> Self {
        Self {
            shell: CommandPolicyConfig::new(),
            cmd_exec: CommandPolicyConfig::new().allow_command("curl"),
        }
    }

    pub fn with_shell_config(mut self, cfg: CommandPolicyConfig) -> Self {
        self.shell = cfg;
        self
    }

    pub fn with_cmd_exec_config(mut self, cfg: CommandPolicyConfig) -> Self {
        self.cmd_exec = cfg;
        self
    }

    fn extract_command_line(tool_name: &str, input: &Value) -> Option<String> {
        match tool_name {
            "shell" => {
                let command = input.get("command")?.as_str()?.trim().to_string();
                let args = input
                    .get("args")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|x| x.as_str())
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                if args.is_empty() {
                    Some(command)
                } else {
                    Some(format!("{} {}", command, args.join(" ")))
                }
            }
            "cmd_exec" => Some(input.get("command")?.as_str()?.trim().to_string()),
            _ => None,
        }
    }

    fn first_token(command_line: &str) -> Option<String> {
        let t = command_line.trim();
        if t.is_empty() {
            return None;
        }
        let mut token = t
            .split_whitespace()
            .next()?
            .trim_matches('"')
            .trim_matches('\'');
        if token.ends_with(".exe") {
            token = token.trim_end_matches(".exe");
        }
        Some(token.to_ascii_lowercase())
    }

    fn is_dangerous_command(cmd: &str) -> bool {
        matches!(
            cmd,
            "rm" | "del"
                | "erase"
                | "rmdir"
                | "rd"
                | "format"
                | "mkfs"
                | "dd"
                | "shutdown"
                | "reboot"
                | "poweroff"
                | "reg"
                | "diskpart"
                | "bcdedit"
                | "sc"
                | "net"
        )
    }

    fn contains_shell_operators(command_line: &str) -> bool {
        let forbidden = ["|", "&&", ";", ">", "<", "`", "$(", "\n", "\r"];
        forbidden.iter().any(|x| command_line.contains(x))
    }

    fn check_command(cfg: &CommandPolicyConfig, command_line: &str) -> Result<(), ToolError> {
        if Self::contains_shell_operators(command_line) {
            return Err(ToolError::PolicyDenied {
                rule_id: "default.shell_operators".to_string(),
                reason: "command contains forbidden shell operators".to_string(),
            });
        }

        let cmd = Self::first_token(command_line).ok_or_else(|| ToolError::PolicyDenied {
            rule_id: "default.empty_command".to_string(),
            reason: "empty command".to_string(),
        })?;

        if cfg
            .denied_commands
            .iter()
            .any(|x| x.eq_ignore_ascii_case(&cmd))
        {
            return Err(ToolError::PolicyDenied {
                rule_id: "config.denied_command".to_string(),
                reason: format!("command '{}' is denied", cmd),
            });
        }

        if Self::is_dangerous_command(&cmd) {
            return Err(ToolError::PolicyDenied {
                rule_id: "default.dangerous_command".to_string(),
                reason: format!("dangerous command '{}' is blocked by default", cmd),
            });
        }

        if !cfg.allowed_commands.is_empty()
            && !cfg
                .allowed_commands
                .iter()
                .any(|x| x.eq_ignore_ascii_case(&cmd))
        {
            return Err(ToolError::PolicyDenied {
                rule_id: "config.not_allowed".to_string(),
                reason: format!("command '{}' is not in allowlist", cmd),
            });
        }

        Ok(())
    }
}

#[async_trait]
impl ToolPolicy for DefaultToolPolicy {
    async fn check(&self, ctx: &ToolCallContext) -> Result<(), ToolError> {
        let name = ctx.tool_call.name.as_str();
        let input = &ctx.tool_call.input;
        let Some(command_line) = Self::extract_command_line(name, input) else {
            return Ok(());
        };

        match name {
            "shell" => Self::check_command(&self.shell, &command_line),
            "cmd_exec" => Self::check_command(&self.cmd_exec, &command_line),
            _ => Ok(()),
        }
    }
}

/// 一个最简 Agent：仅调用一次 `LlmProvider::chat`，不包含 tool loop。
pub struct SimpleAgent<P> {
    /// LLM provider 实例。
    provider: P,
    /// 可选系统提示词。
    system_prompt: Option<String>,
}

/// Tool 注册表。
///
/// 用途：把一组 `Tool` 按名称组织起来，便于在 tool-calling loop 中按 `ToolCall.name` 查找。
#[derive(Default, Clone)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

/// Skill 注册表。
///
/// 注意：由于移除了 Skill trait，这个注册表暂时留作占位符。
/// 在实际使用中，技能应该作为具体类型直接调用。
#[derive(Default, Clone)]
pub struct SkillRegistry {
    // 暂时留空，后续可以根据需要重新设计
}

impl ToolRegistry {
    /// 创建一个空注册表。
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// 注册一个工具。
    ///
    /// 注意：如果同名工具重复注册，后者会覆盖前者。
    pub fn register<T: Tool + 'static>(mut self, tool: T) -> Self {
        self.tools.insert(tool.name().to_string(), Arc::new(tool));
        self
    }

    /// 注册一个工具（trait object 版本）。
    ///
    /// 使用场景：
    /// - 上层把工具以 `Arc<dyn Tool>` 的形式动态组装（例如把 skills 适配成 tools）。
    ///
    /// 注意：如果同名工具重复注册，后者会覆盖前者。
    pub fn register_arc(mut self, tool: Arc<dyn Tool>) -> Self {
        self.tools.insert(tool.name().to_string(), tool);
        self
    }

    /// 获取工具定义列表（用于发给 provider 进行 tool/function 注册）。
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools
            .values()
            .map(|tool| ToolDefinition {
                name: tool.name().to_string(),
                description: tool.description().map(|s| s.to_string()),
                input_schema: tool.input_schema(),
            })
            .collect()
    }

    /// 按名称查找工具。
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }
}

/// 为默认 runtime 实现 core 层的 `Runtime` 规范。
///
/// 这样上层代码就可以依赖 `agentkit_core::runtime::Runtime` 这个抽象，
/// 默认情况下用 `agentkit-runtime` 提供的实现；同时也允许用户自定义 runtime。
#[async_trait]
impl<P> Runtime for SimpleAgent<P>
where
    P: LlmProvider,
{
    async fn run(&self, input: AgentInput) -> Result<AgentOutput, AgentError> {
        // 复用现有 Agent::run 逻辑，避免重复实现。
        <Self as Agent>::run(self, input).await
    }
}

impl SkillRegistry {
    /// 创建一个空注册表。
    pub fn new() -> Self {
        Self {}
    }
}

/// 支持工具调用的最小 Agent。
///
/// 该 Agent 的核心逻辑：
/// 1. 调用 `provider.chat()` 让模型生成回复或请求工具（`tool_calls`）
/// 2. 如果模型请求工具：执行工具并把结果追加回 messages
/// 3. 重复 1-2 直到模型停止请求工具或达到步数上限
pub struct ToolCallingAgent<P> {
    /// LLM provider 实例。
    provider: P,
    /// 可选系统提示词。
    system_prompt: Option<String>,
    /// 可用工具注册表。
    tools: ToolRegistry,
    /// 工具安全策略（allow/deny）。
    policy: Arc<dyn ToolPolicy>,
    /// 审计记录器。
    audit: Arc<dyn AuditSink>,
    /// 最大循环步数，避免模型陷入无限调用。
    max_steps: usize,
}

impl<P> ToolCallingAgent<P> {
    /// 创建一个支持工具调用的 Agent。
    pub fn new(provider: P, tools: ToolRegistry) -> Self {
        Self {
            provider,
            system_prompt: None,
            tools,
            policy: Arc::new(DefaultToolPolicy::new()),
            audit: Arc::new(NoopAuditSink),
            max_steps: 8,
        }
    }

    /// 设置系统提示词（会在运行时插入到 messages 开头）。
    pub fn with_system_prompt(mut self, system_prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(system_prompt.into());
        self
    }

    pub fn with_policy(mut self, policy: Arc<dyn ToolPolicy>) -> Self {
        self.policy = policy;
        self
    }

    pub fn with_audit(mut self, audit: Arc<dyn AuditSink>) -> Self {
        self.audit = audit;
        self
    }

    /// 设置最大循环步数。
    pub fn with_max_steps(mut self, max_steps: usize) -> Self {
        self.max_steps = max_steps;
        self
    }

    /// 执行单个工具调用，并返回 `ToolResult`。
    async fn execute_tool_call(&self, call: &ToolCall) -> Result<ToolResult, AgentError> {
        let input_len = call.input.to_string().len();
        let input_preview = {
            const MAX: usize = 800;
            let s = call.input.to_string();
            if s.len() <= MAX {
                s
            } else {
                format!("{}...<truncated:{}>", &s[..MAX], s.len())
            }
        };
        self.audit.record(AuditEvent::ToolCallStart {
            tool_name: call.name.clone(),
            tool_call_id: call.id.clone(),
            input_len,
        });
        info!(
            tool.name = %call.name,
            tool.call_id = %call.id,
            tool.input_len = input_len,
            "tool_call.execute.start"
        );
        debug!(
            tool.name = %call.name,
            tool.call_id = %call.id,
            tool.input = %input_preview,
            "tool_call.execute.input"
        );

        let ctx = ToolCallContext {
            tool_call: call.clone(),
        };

        if let Err(e) = self.policy.check(&ctx).await {
            match &e {
                ToolError::PolicyDenied { rule_id, reason } => {
                    self.audit.record(AuditEvent::ToolCallDenied {
                        tool_name: call.name.clone(),
                        tool_call_id: call.id.clone(),
                        rule_id: rule_id.clone(),
                        reason: reason.clone(),
                    });
                    let out = apply_output_limit(
                        json!({
                            "ok": false,
                            "error": {
                                "kind": "policy_denied",
                                "rule_id": rule_id,
                                "reason": reason
                            }
                        }),
                        DEFAULT_TOOL_OUTPUT_MAX_BYTES,
                    );
                    debug!(
                        tool.name = %call.name,
                        tool.call_id = %call.id,
                        policy.rule_id = %rule_id,
                        "tool_call.execute.denied"
                    );
                    return Ok(ToolResult {
                        tool_call_id: call.id.clone(),
                        output: out,
                    });
                }
                _ => {
                    self.audit.record(AuditEvent::ToolCallError {
                        tool_name: call.name.clone(),
                        tool_call_id: call.id.clone(),
                        message: e.to_string(),
                    });
                    let out = apply_output_limit(
                        json!({
                            "ok": false,
                            "error": {
                                "kind": "policy_error",
                                "message": e.to_string()
                            }
                        }),
                        DEFAULT_TOOL_OUTPUT_MAX_BYTES,
                    );
                    debug!(
                        tool.name = %call.name,
                        tool.call_id = %call.id,
                        error = %e.to_string(),
                        "tool_call.execute.policy_error"
                    );
                    return Ok(ToolResult {
                        tool_call_id: call.id.clone(),
                        output: out,
                    });
                }
            }
        }

        let start = std::time::Instant::now();
        let tool = self.tools.get(&call.name).ok_or_else(|| {
            AgentError::Message(format!(
                "未找到工具：{} (tool_call_id={})",
                call.name, call.id
            ))
        })?;

        let tool_output = match tool.call(call.input.clone()).await {
            Ok(v) => json!({"ok": true, "output": v}),
            Err(ToolError::PolicyDenied { rule_id, reason }) => {
                self.audit.record(AuditEvent::ToolCallDenied {
                    tool_name: call.name.clone(),
                    tool_call_id: call.id.clone(),
                    rule_id: rule_id.clone(),
                    reason: reason.clone(),
                });
                json!({
                    "ok": false,
                    "error": {"kind": "policy_denied", "rule_id": rule_id, "reason": reason}
                })
            }
            Err(e) => {
                self.audit.record(AuditEvent::ToolCallError {
                    tool_name: call.name.clone(),
                    tool_call_id: call.id.clone(),
                    message: e.to_string(),
                });
                json!({
                    "ok": false,
                    "error": {"kind": "tool_error", "message": e.to_string()}
                })
            }
        };

        let tool_output = apply_output_limit(tool_output, DEFAULT_TOOL_OUTPUT_MAX_BYTES);
        let output_preview = {
            const MAX: usize = 1200;
            let s = tool_output.to_string();
            if s.len() <= MAX {
                s
            } else {
                format!("{}...<truncated:{}>", &s[..MAX], s.len())
            }
        };

        let elapsed_ms = start.elapsed().as_millis() as u64;
        let output_len = tool_output.to_string().len();
        self.audit.record(AuditEvent::ToolCallDone {
            tool_name: call.name.clone(),
            tool_call_id: call.id.clone(),
            output_len,
            elapsed_ms,
        });
        info!(
            tool.name = %call.name,
            tool.call_id = %call.id,
            tool.output_len = output_len,
            tool.elapsed_ms = elapsed_ms,
            "tool_call.execute.done"
        );
        debug!(
            tool.name = %call.name,
            tool.call_id = %call.id,
            tool.output = %output_preview,
            "tool_call.execute.output"
        );

        Ok(ToolResult {
            tool_call_id: call.id.clone(),
            output: tool_output,
        })
    }

    /// 将工具结果转换为“工具消息”追加回对话历史。
    fn tool_result_to_message(result: &ToolResult, tool_name: &str) -> ChatMessage {
        let payload = Value::Object(
            [
                (
                    "tool_call_id".to_string(),
                    Value::String(result.tool_call_id.clone()),
                ),
                ("output".to_string(), result.output.clone()),
            ]
            .into_iter()
            .collect(),
        );

        ChatMessage {
            role: Role::Tool,
            content: payload.to_string(),
            name: Some(tool_name.to_string()),
        }
    }
}

/// 支持流式输出的 ToolCalling Agent。
///
/// 说明：
/// - 使用 core 层现有的 `LlmProvider::stream_chat()` 与 `ChatStreamChunk`。
/// - 当前实现把 token/delta 以 `ChannelEvent::Raw({type: token, delta})` 发出。
/// - 完整 assistant message 在每个 step 流结束后以 `ChannelEvent::Message` 发出。
pub struct StreamingToolCallingAgent {
    provider: Arc<dyn LlmProvider>,
    system_prompt: Option<String>,
    tools: ToolRegistry,
    policy: Arc<dyn ToolPolicy>,
    audit: Arc<dyn AuditSink>,
    max_steps: usize,
}

impl StreamingToolCallingAgent {
    pub fn new(provider: Arc<dyn LlmProvider>, tools: ToolRegistry) -> Self {
        Self {
            provider,
            system_prompt: None,
            tools,
            policy: Arc::new(DefaultToolPolicy::new()),
            audit: Arc::new(NoopAuditSink),
            max_steps: 8,
        }
    }

    pub fn with_system_prompt(mut self, system_prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(system_prompt.into());
        self
    }

    pub fn with_policy(mut self, policy: Arc<dyn ToolPolicy>) -> Self {
        self.policy = policy;
        self
    }

    pub fn with_audit(mut self, audit: Arc<dyn AuditSink>) -> Self {
        self.audit = audit;
        self
    }

    pub fn with_max_steps(mut self, max_steps: usize) -> Self {
        self.max_steps = max_steps;
        self
    }

    pub fn run_stream(
        &self,
        mut input: AgentInput,
    ) -> BoxStream<'static, Result<ChannelEvent, AgentError>> {
        let provider = self.provider.clone();
        let tools = self.tools.clone();
        let policy = self.policy.clone();
        let audit = self.audit.clone();
        let system_prompt = self.system_prompt.clone();
        let max_steps = self.max_steps;

        let stream = async_stream::try_stream! {
            if let Some(system_prompt) = &system_prompt {
                input.messages.insert(
                    0,
                    ChatMessage {
                        role: Role::System,
                        content: system_prompt.clone(),
                        name: None,
                    },
                );
            }

            let tool_defs = tools.definitions();
            let mut messages = input.messages;

            for step in 0..max_steps {
                debug!(step, messages_len = messages.len(), "stream_agent.step.start");

                let request = ChatRequest {
                    messages: messages.clone(),
                    model: None,
                    tools: Some(tool_defs.clone()),
                    temperature: None,
                    max_tokens: None,
                    response_format: None,
                    metadata: input.metadata.clone(),
                };

                let mut assistant_text = String::new();
                let mut tool_calls: Vec<ToolCall> = Vec::new();

                let mut s = provider
                    .stream_chat(request)
                    .map_err(|e| AgentError::Message(e.to_string()))?;

                while let Some(item) = s.next().await {
                    let chunk = item.map_err(|e| AgentError::Message(e.to_string()))?;

                    if let Some(delta) = chunk.delta {
                        assistant_text.push_str(&delta);
                        yield ChannelEvent::Raw(json!({"type": "token", "delta": delta}));
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
                yield ChannelEvent::Message(assistant_msg);

                if tool_calls.is_empty() {
                    break;
                }

                for call in tool_calls.iter() {
                    yield ChannelEvent::ToolCall(call.clone());

                    let exec_agent = ToolCallingAgent::<Arc<dyn LlmProvider>> {
                        provider: provider.clone(),
                        system_prompt: None,
                        tools: tools.clone(),
                        policy: policy.clone(),
                        audit: audit.clone(),
                        max_steps,
                    };
                    let result = exec_agent.execute_tool_call(call).await?;
                    yield ChannelEvent::ToolResult(result.clone());
                    let tool_msg = ToolCallingAgent::<Arc<dyn LlmProvider>>::tool_result_to_message(
                        &result,
                        &call.name,
                    );
                    messages.push(tool_msg);
                }
            }
        };

        Box::pin(stream)
    }
}

impl<P> SimpleAgent<P> {
    /// 创建一个 `SimpleAgent`。
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            system_prompt: None,
        }
    }

    /// 设置系统提示词（会在运行时插入到 messages 开头）。
    pub fn with_system_prompt(mut self, system_prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(system_prompt.into());
        self
    }
}

#[async_trait]
impl<P> Agent for SimpleAgent<P>
where
    P: LlmProvider,
{
    /// 执行一次对话：把输入 messages 发送给 provider 并返回最终输出。
    async fn run(&self, mut input: AgentInput) -> Result<AgentOutput, AgentError> {
        if let Some(system_prompt) = &self.system_prompt {
            input.messages.insert(
                0,
                ChatMessage {
                    role: Role::System,
                    content: system_prompt.clone(),
                    name: None,
                },
            );
        }

        let request = ChatRequest {
            messages: input.messages,
            model: None,
            tools: None,
            temperature: None,
            max_tokens: None,
            response_format: None,
            metadata: input.metadata,
        };

        let resp = self
            .provider
            .chat(request)
            .await
            .map_err(|e| AgentError::Message(e.to_string()))?;

        Ok(AgentOutput {
            message: resp.message,
            tool_results: vec![],
        })
    }
}

#[async_trait]
impl<P> Agent for ToolCallingAgent<P>
where
    P: LlmProvider,
{
    /// 执行带工具调用的对话循环。
    async fn run(&self, mut input: AgentInput) -> Result<AgentOutput, AgentError> {
        info!(max_steps = self.max_steps, "agent.run.start");
        if let Some(system_prompt) = &self.system_prompt {
            input.messages.insert(
                0,
                ChatMessage {
                    role: Role::System,
                    content: system_prompt.clone(),
                    name: None,
                },
            );
        }

        // tool 定义会提供给 provider，用于 function-calling/tool-calling 注册。
        let tool_defs = self.tools.definitions();

        let mut messages = input.messages;
        let mut tool_results: Vec<ToolResult> = Vec::new();

        for step in 0..self.max_steps {
            debug!(step, messages_len = messages.len(), "agent.step.start");

            let preview = |s: &str| {
                const MAX: usize = 800;
                if s.len() <= MAX {
                    s.to_string()
                } else {
                    format!("{}...<truncated:{}>", &s[..MAX], s.len())
                }
            };

            let last_user_preview = messages
                .iter()
                .rev()
                .find(|m| m.role == Role::User)
                .map(|m| preview(&m.content));

            debug!(
                step,
                tools_len = tool_defs.len(),
                last_user = last_user_preview.as_deref().unwrap_or(""),
                "agent.step.provider_chat.start"
            );

            let request = ChatRequest {
                messages: messages.clone(),
                model: None,
                tools: Some(tool_defs.clone()),
                temperature: None,
                max_tokens: None,
                response_format: None,
                metadata: input.metadata.clone(),
            };

            let chat_start = std::time::Instant::now();

            let resp = self
                .provider
                .chat(request)
                .await
                .map_err(|e| AgentError::Message(e.to_string()))?;

            let chat_elapsed_ms = chat_start.elapsed().as_millis() as u64;
            debug!(
                step,
                chat_elapsed_ms,
                assistant_content_len = resp.message.content.len(),
                tool_calls_len = resp.tool_calls.len(),
                "agent.step.provider_chat.done"
            );

            if !resp.tool_calls.is_empty() {
                let calls_preview: Vec<String> = resp
                    .tool_calls
                    .iter()
                    .map(|c| {
                        let input = preview(&c.input.to_string());
                        format!("{}(id={}, input={})", c.name, c.id, input)
                    })
                    .collect();
                debug!(step, tool_calls = ?calls_preview, "agent.step.tool_calls");
            }

            // 追加 assistant 回复到对话历史。
            messages.push(resp.message.clone());

            // 如果没有工具调用，则直接返回最终消息。
            if resp.tool_calls.is_empty() {
                info!(
                    step,
                    tool_results_len = tool_results.len(),
                    "agent.run.done"
                );
                return Ok(AgentOutput {
                    message: resp.message,
                    tool_results,
                });
            }

            // 执行工具调用，并将结果追加回 messages。
            for call in resp.tool_calls.iter() {
                let result = self.execute_tool_call(call).await?;
                tool_results.push(result.clone());

                let tool_msg = Self::tool_result_to_message(&result, &call.name);
                messages.push(tool_msg);
            }
        }

        warn!(
            max_steps = self.max_steps,
            tool_results_len = tool_results.len(),
            "agent.run.max_steps_exceeded"
        );
        Err(AgentError::Message(format!(
            "超过最大步数限制（max_steps={}），仍未结束工具调用流程",
            self.max_steps
        )))
    }
}

/// 为支持工具调用的默认 Agent 实现 `Runtime` 规范。
///
/// 目的：上层依赖 `agentkit_core::runtime::Runtime` 抽象时，
/// 可以直接替换为用户自定义的 runtime 实现。
#[async_trait]
impl<P> Runtime for ToolCallingAgent<P>
where
    P: LlmProvider,
{
    async fn run(&self, input: AgentInput) -> Result<AgentOutput, AgentError> {
        // 复用现有 Agent::run 逻辑，避免重复实现。
        <Self as Agent>::run(self, input).await
    }
}
