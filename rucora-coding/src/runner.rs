//! Coding 会话运行器。
//!
//! [`CodingSession`] 把三件事粘合在一起：
//!
//! 1. **事件**：消费 `Agent::run_stream` 产出的 `ChannelEvent` 流，
//!    翻译为统一的 [`AgentEvent`] 发布到 [`EventBus`]
//! 2. **审批**：持有 [`ApprovalToolPolicy`]，对外提供裁决接口
//!    （GUI 在收到 `ApprovalRequired` 事件后调用 [`CodingSession::resolve_approval`]）
//! 3. **持久化**：任务、转录与状态写入 [`FileSessionStore`]，
//!    会话生命周期独立于任何 UI 窗口
//!
//! # 事件顺序保证
//!
//! 流式引擎的事件顺序为（每个步骤）：`TokenDelta*` → `Message(assistant)` →
//! `ToolCall` → `ToolResult`。翻译后的事件保持该顺序，序号单调递增。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use futures_util::{StreamExt, stream::BoxStream};
use rucora_core::agent::{Agent, AgentError};
use rucora_core::channel::types::ChannelEvent;
use rucora_core::provider::types::{ChatMessage, Role};
use rucora_core::tool::types::ToolCall;
use serde_json::Value;
use tracing::{debug, info, warn};

use tokio::sync::Notify;

use crate::context::ContextAssembler;
use crate::diff::ChangeTracker;
use crate::events::{AgentEvent, EventBus, SequencedEvent};
use crate::permissions::{ApprovalDecision, ApprovalRequest, ApprovalToolPolicy};
use crate::session::{FileSessionStore, SessionRecord, SessionStatus, TranscriptEntry};

/// 从会话转录重建对话历史（供多轮对话/恢复会话注入模型输入）。
///
/// 映射关系：
/// - `Task` → 用户消息
/// - `ToolCall` → assistant 工具调用 + 工具结果消息（保持配对）
/// - `AssistantText` → 助手文本消息
pub fn transcript_to_history(record: &SessionRecord) -> Vec<ChatMessage> {
    let mut messages = Vec::new();
    for entry in &record.transcript {
        match entry {
            TranscriptEntry::Task { task } => {
                messages.push(ChatMessage::user(task.clone()));
            }
            TranscriptEntry::ToolCall {
                name,
                call_id,
                input,
                output,
                ..
            } => {
                messages.push(ChatMessage::assistant_with_tool_calls(
                    String::new(),
                    vec![rucora_core::tool::types::ToolCall {
                        id: call_id.clone(),
                        name: name.clone(),
                        input: input.clone(),
                    }],
                ));
                // 工具结果文本：优先取 output.output（执行器包装结构）
                let result_text = output
                    .get("output")
                    .and_then(|v| v.as_str())
                    .map_or_else(|| output.to_string(), ToString::to_string);
                messages.push(ChatMessage::tool_result(
                    name.clone(),
                    call_id.clone(),
                    result_text,
                ));
            }
            TranscriptEntry::AssistantText { text } => {
                messages.push(ChatMessage::assistant(text.clone()));
            }
        }
    }
    messages
}

/// 一次任务的执行结果。
#[derive(Debug, Clone)]
pub struct TaskOutcome {
    /// 最终回复文本（全部增量的累积）。
    pub text: String,
    /// 工具调用次数。
    pub tool_calls: usize,
    /// 任务耗时（毫秒）。
    pub duration_ms: u64,
    /// 任务是否成功完成（失败/取消时为 `false`）。
    pub success: bool,
    /// 是否被用户取消。
    pub cancelled: bool,
    /// 本次任务捕获的文件变更数（未接入 ChangeTracker 时为 0）。
    pub changes: usize,
}

/// 取消状态：跨任务共享，`cancel()` 唤醒正在消费流的任务。
#[derive(Default)]
struct CancelState {
    cancelled: std::sync::atomic::AtomicBool,
    notify: Notify,
}

impl CancelState {
    fn mark_cancelled(&self) {
        self.cancelled
            .store(true, std::sync::atomic::Ordering::Relaxed);
        self.notify.notify_waiters();
    }

    async fn wait(&self) {
        while !self.cancelled.load(std::sync::atomic::Ordering::Relaxed) {
            self.notify.notified().await;
        }
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// Coding 会话。
///
/// 聚合 Agent、事件总线、审批策略与会话存储。
/// 会话运行不依赖任何 UI——即使消费者全部退出，`run_task` 也会继续执行；
/// 可通过 [`CodingSession::cancel`] 随时取消当前任务。
pub struct CodingSession {
    agent: Arc<dyn Agent>,
    bus: Arc<EventBus>,
    policy: Arc<ApprovalToolPolicy>,
    store: FileSessionStore,
    record: Mutex<SessionRecord>,
    workspace: PathBuf,
    cancel_state: Arc<CancelState>,
    tracker: Option<Arc<ChangeTracker>>,
}

/// 工具调用的关联信息（从 ToolCall 事件捕获，供 ToolResult 落转录）。
struct ToolCallInfo {
    name: String,
    input: Value,
}

impl CodingSession {
    /// 启动一个新会话（创建记录并发布 `SessionStarted` 事件）。
    ///
    /// # Errors
    ///
    /// 会话记录写入失败时返回 [`AgentError`]。
    pub fn start(
        agent: Arc<dyn Agent>,
        workspace: &Path,
        store: FileSessionStore,
        bus: Arc<EventBus>,
        policy: Arc<ApprovalToolPolicy>,
        tracker: Option<Arc<ChangeTracker>>,
        session_id: Option<&str>,
    ) -> Result<Arc<Self>, AgentError> {
        let record = session_id.map_or_else(|| SessionRecord::new(workspace), |id| {
            SessionRecord::with_id(workspace, id)
        });
        let session_id = record.id.clone();
        store
            .save(&record)
            .map_err(|e| AgentError::Message(format!("会话记录写入失败：{e}")))?;

        info!(session_id = %session_id, workspace = %workspace.display(), "coding.session.started");
        bus.publish(AgentEvent::SessionStarted { session_id });
        Ok(Arc::new(Self {
            agent,
            bus,
            policy,
            store,
            record: Mutex::new(record),
            workspace: workspace.to_path_buf(),
            cancel_state: Arc::new(CancelState::default()),
            tracker,
        }))
    }

    /// 从持久化记录恢复会话（多轮对话/重启后继续）。
    ///
    /// 恢复后，后续每个任务的模型输入都会带上此前任务的对话历史
    /// （用户消息、工具调用与结果、助手回复），实现真正的多轮会话。
    ///
    /// # Errors
    ///
    /// 记录加载失败或写入失败时返回 [`AgentError`]。
    pub fn resume(
        agent: Arc<dyn Agent>,
        workspace: &Path,
        store: FileSessionStore,
        bus: Arc<EventBus>,
        policy: Arc<ApprovalToolPolicy>,
        tracker: Option<Arc<ChangeTracker>>,
        record_id: &str,
    ) -> Result<Arc<Self>, AgentError> {
        let mut record = store
            .load(record_id)
            .map_err(|e| AgentError::Message(format!("会话记录加载失败：{e}")))?;
        record.status = SessionStatus::Running;
        record.touch();
        store
            .save(&record)
            .map_err(|e| AgentError::Message(format!("会话记录写入失败：{e}")))?;

        bus.publish(AgentEvent::SessionResumed {
            session_id: record.id.clone(),
        });

        info!(session_id = %record.id, tasks = record.tasks.len(), "coding.session.resumed");
        Ok(Arc::new(Self {
            agent,
            bus,
            policy,
            store,
            record: Mutex::new(record),
            workspace: workspace.to_path_buf(),
            cancel_state: Arc::new(CancelState::default()),
            tracker,
        }))
    }

    /// 从当前转录重建对话历史。
    fn history_from_transcript(&self) -> Vec<ChatMessage> {
        transcript_to_history(&self.record.lock().expect("会话记录锁中毒"))
    }

    /// 会话 ID。
    pub fn id(&self) -> String {
        self.record.lock().expect("会话记录锁中毒").id.clone()
    }

    /// 工作目录。
    pub fn workspace(&self) -> &Path {
        &self.workspace
    }

    /// 事件总线（供 UI 订阅）。
    pub fn events(&self) -> Arc<EventBus> {
        self.bus.clone()
    }

    /// 全量事件快照（含序号）。
    pub fn event_history(&self) -> Vec<SequencedEvent> {
        self.bus.history()
    }

    /// 裁决一个待审批请求。
    ///
    /// # 返回
    ///
    /// 请求存在且已送达裁决时返回 `true`。
    pub fn resolve_approval(&self, request_id: &str, decision: ApprovalDecision) -> bool {
        self.policy.resolve(request_id, decision)
    }

    /// 当前待审批请求快照。
    pub fn pending_approvals(&self) -> Vec<ApprovalRequest> {
        self.policy.pending_requests()
    }

    /// 当前会话记录快照。
    pub fn record_snapshot(&self) -> SessionRecord {
        self.record.lock().expect("会话记录锁中毒").clone()
    }

    /// 变更追踪器（未接入时返回 `None`）。
    pub fn change_tracker(&self) -> Option<Arc<ChangeTracker>> {
        self.tracker.clone()
    }

    /// 会话期间捕获的文件变更统一 diff（未接入追踪器时为空）。
    pub fn change_diffs(&self) -> Vec<crate::diff::FileDiff> {
        self.tracker.as_ref().map_or_else(Vec::new, |t| t.diffs())
    }

    /// 回滚全部已捕获的文件变更。
    ///
    /// # 返回
    ///
    /// 成功回滚的文件数；未接入追踪器时为 `Ok(0)`。
    ///
    /// # Errors
    ///
    /// 文件写入/删除失败时返回 IO 错误。
    pub fn rollback_all(&self) -> std::io::Result<usize> {
        let count = match &self.tracker {
            Some(tracker) => tracker.rollback_all()?,
            None => return Ok(0),
        };
        // 回滚状态同步进会话记录并落盘
        if let Some(tracker) = &self.tracker {
            let mut record = self.record.lock().expect("会话记录锁中毒");
            record.changes = tracker.changes();
            record.touch();
        }
        self.persist();
        Ok(count)
    }

    /// 执行一个任务（完整闭环：流式事件 → 翻译 → 持久化）。
    ///
    /// # Errors
    ///
    /// Agent 输入构造失败或 Agent 运行出错时返回 [`AgentError`]。
    /// 注意：工具层面的失败（含审批拒绝）不会中断任务——它们以结构化
    /// 结果回传给模型，由模型决定后续动作。
    pub async fn run_task(&self, task: &str) -> Result<TaskOutcome, AgentError> {
        self.run_task_inner(task, None).await
    }

    /// 执行一个任务（带项目上下文注入）。
    ///
    /// 先用 [`ContextAssembler`] 组装"项目指令 + 仓库结构 + 任务"作为输入，
    /// 其余行为与 [`CodingSession::run_task`] 一致。
    ///
    /// # Errors
    ///
    /// 同 [`CodingSession::run_task`]。
    pub async fn run_task_with_context(
        &self,
        task: &str,
        assembler: &ContextAssembler,
    ) -> Result<TaskOutcome, AgentError> {
        self.run_task_inner(task, Some(assembler)).await
    }

    /// 任务执行内部实现。
    async fn run_task_inner(
        &self,
        task: &str,
        context: Option<&ContextAssembler>,
    ) -> Result<TaskOutcome, AgentError> {
        let started_at = Instant::now();
        let task_text = task.to_string();
        // 历史快照必须在推送当前任务之前（否则当前任务会出现在历史里）
        let history = self.history_from_transcript();
        self.publish_and_record(
            AgentEvent::TaskStarted { task: task_text.clone() },
            |record| {
                record.tasks.push(task_text.clone());
                record.transcript.push(TranscriptEntry::Task { task: task_text });
            },
        );

        let input_text = context.map_or_else(|| task.to_string(), |a| a.assemble(task));
        let input = rucora_core::agent::AgentInput::new(input_text)?.with_history(history);
        let stream = self.agent.run_stream(input);
        let (text, tool_calls, failure) = self.consume_stream(stream).await;

        let cancelled = self.cancel_state.is_cancelled();
        let duration_ms = started_at.elapsed().as_millis() as u64;
        let changes = self
            .tracker
            .as_ref()
            .map_or(0, |tracker| tracker.changes().len());
        self.finish_task(&text, tool_calls, duration_ms, failure.as_deref(), cancelled);

        match (failure, cancelled) {
            (None, false) => Ok(TaskOutcome {
                text,
                tool_calls,
                duration_ms,
                success: true,
                cancelled: false,
                changes,
            }),
            (None, true) => Ok(TaskOutcome {
                text,
                tool_calls,
                duration_ms,
                success: false,
                cancelled: true,
                changes,
            }),
            (Some(message), _) => Err(AgentError::Message(message)),
        }
    }

    /// 取消当前正在执行的任务。
    ///
    /// 流消费循环在下一个事件边界处退出；待审批请求被拒绝（模型可见）；
    /// 会话状态标记为 [`SessionStatus::Cancelled`] 并发布
    /// [`AgentEvent::Cancelled`]。任务返回
    /// `Ok(TaskOutcome { cancelled: true, .. })`。
    pub fn cancel(&self) {
        // 拒绝所有待审批请求，避免审批等待悬挂
        for request in self.policy.pending_requests() {
            self.policy.resolve(&request.request_id, ApprovalDecision::Deny);
        }
        self.cancel_state.mark_cancelled();
        debug!("coding.session.cancel_requested");
    }

    /// 消费 Agent 的 ChannelEvent 流，翻译并发布 AgentEvent。
    ///
    /// 返回（累积文本，工具调用数，失败原因）。取消时失败原因为 `None`，
    /// 由调用方通过取消状态区分。
    async fn consume_stream(
        &self,
        mut stream: BoxStream<'static, Result<ChannelEvent, AgentError>>,
    ) -> (String, usize, Option<String>) {
        let mut final_text = String::new();
        let mut tool_calls = 0usize;
        let mut failure: Option<String> = None;
        let mut cancelled = false;
        // call_id → 调用信息（ToolResult 只带 call_id，名字/输入由配对的 ToolCall 提供）
        let mut call_infos: HashMap<String, ToolCallInfo> = HashMap::new();
        // 当前步骤累积的文本段（Message 事件用于切分步骤）
        let mut step_text = String::new();

        loop {
            // 取消优先：在事件边界处响应 cancel()
            let item = tokio::select! {
                biased;
                // wait() 内部先检查取消标记：即使 cancel 发生在本轮处理期间，
                // 下一轮 select 也会立即走取消分支
                _ = self.cancel_state.wait() => {
                    cancelled = true;
                    break;
                }
                item = stream.next() => item,
            };
            let Some(result) = item else {
                break; // 流自然结束
            };

            match result {
                Ok(ChannelEvent::TokenDelta(delta)) => {
                    final_text.push_str(&delta.delta);
                    step_text.push_str(&delta.delta);
                    self.bus
                        .publish(AgentEvent::MessageDelta { delta: delta.delta });
                }
                Ok(ChannelEvent::Message(message)) => {
                    if message.role == Role::Assistant {
                        let has_tool_calls = !message.tool_calls().is_empty();
                        self.bus.publish(AgentEvent::AssistantMessage {
                            text: step_text.clone(),
                            has_tool_calls,
                        });
                        step_text.clear();
                    }
                    // 系统注入消息（循环检测警告等）不进入 UI 事件流
                }
                Ok(ChannelEvent::ToolCall(call)) => {
                    tool_calls += 1;
                    let ToolCall { id, name, input } = call;
                    call_infos.insert(
                        id.clone(),
                        ToolCallInfo {
                            name: name.clone(),
                            input: input.clone(),
                        },
                    );
                    self.bus.publish(AgentEvent::ToolStarted {
                        call_id: id,
                        name,
                        input,
                    });
                }
                Ok(ChannelEvent::ToolResult(result)) => {
                    let info = call_infos.remove(&result.tool_call_id);
                    let name = info.as_ref().map_or_else(String::new, |info| info.name.clone());
                    self.bus.publish(AgentEvent::ToolCompleted {
                        call_id: result.tool_call_id.clone(),
                        name: name.clone(),
                        output: result.output.clone(),
                        success: result.success,
                    });
                    self.record_and_save(|record| {
                        record.transcript.push(TranscriptEntry::ToolCall {
                            name,
                            call_id: result.tool_call_id.clone(),
                            input: info.map_or(Value::Null, |info| info.input),
                            output: result.output.clone(),
                            success: result.success,
                        });
                    });
                }
                Ok(ChannelEvent::Error(error)) => {
                    warn!(kind = %error.kind, message = %error.message, "coding.session.stream_error");
                    failure = Some(error.message);
                    break;
                }
                // Skill / Memory / Debug / Raw 事件对 UI 非必需，忽略
                Ok(_) => {}
                Err(err) => {
                    failure = Some(err.to_string());
                    break;
                }
            }

            if failure.is_some() {
                break;
            }
        }

        if cancelled {
            return (final_text, tool_calls, None);
        }
        (final_text, tool_calls, failure)
    }

    /// 发布事件并同步修改会话记录，随后落盘。
    fn publish_and_record(
        &self,
        event: AgentEvent,
        mutate: impl FnOnce(&mut SessionRecord),
    ) -> u64 {
        let seq = self.bus.publish(event);
        {
            let mut record = self.record.lock().expect("会话记录锁中毒");
            mutate(&mut record);
            record.touch();
        }
        self.persist();
        seq
    }

    /// 仅修改会话记录并落盘（工具结果由 ToolResult 事件驱动）。
    fn record_and_save(&self, mutate: impl FnOnce(&mut SessionRecord)) {
        {
            let mut record = self.record.lock().expect("会话记录锁中毒");
            mutate(&mut record);
            record.touch();
        }
        self.persist();
    }

    /// 将会话记录写入存储（失败仅记录警告，不影响任务执行）。
    fn persist(&self) {
        let record = self.record.lock().expect("会话记录锁中毒").clone();
        if let Err(e) = self.store.save(&record) {
            warn!(session_id = %record.id, error = %e, "coding.session.persist_failed");
        }
    }

    /// 任务收尾：更新状态、写转录、发布完成/失败/取消事件。
    fn finish_task(
        &self,
        text: &str,
        tool_calls: usize,
        duration_ms: u64,
        failure: Option<&str>,
        cancelled: bool,
    ) {
        let status = if cancelled {
            SessionStatus::Cancelled
        } else if failure.is_some() {
            SessionStatus::Failed
        } else {
            SessionStatus::Completed
        };
        {
            let mut record = self.record.lock().expect("会话记录锁中毒");
            record.status = status;
            if !text.is_empty() {
                record.transcript.push(TranscriptEntry::AssistantText {
                    text: text.to_string(),
                });
            }
            if let Some(tracker) = &self.tracker {
                record.changes = tracker.changes();
            }
            record.touch();
        }
        self.persist();

        match (failure, cancelled) {
            (_, true) => {
                info!(tool_calls, duration_ms, "coding.session.task_cancelled");
                self.bus.publish(AgentEvent::Cancelled);
            }
            (None, false) => {
                debug!(tool_calls, duration_ms, "coding.session.task_completed");
                self.bus.publish(AgentEvent::TaskCompleted {
                    text: text.to_string(),
                    tool_calls,
                    duration_ms,
                });
            }
            (Some(message), false) => {
                {
                    let mut record = self.record.lock().expect("会话记录锁中毒");
                    if let Some(obj) = record.metadata.as_object_mut() {
                        obj.insert("last_error".to_string(), serde_json::json!(message));
                    }
                    record.touch();
                }
                self.persist();
                self.bus.publish(AgentEvent::Failed {
                    message: message.to_string(),
                });
            }
        }
    }
}
