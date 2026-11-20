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
use rucora_core::provider::types::Role;
use serde_json::Value;
use tracing::{debug, info, warn};

use crate::events::{AgentEvent, EventBus, SequencedEvent};
use crate::permissions::{ApprovalDecision, ApprovalRequest, ApprovalToolPolicy};
use crate::session::{FileSessionStore, SessionRecord, SessionStatus, TranscriptEntry};

/// 一次任务的执行结果。
#[derive(Debug, Clone)]
pub struct TaskOutcome {
    /// 最终回复文本（全部增量的累积）。
    pub text: String,
    /// 工具调用次数。
    pub tool_calls: usize,
    /// 任务耗时（毫秒）。
    pub duration_ms: u64,
    /// 任务是否成功完成（失败时为 `false`）。
    pub success: bool,
}

/// Coding 会话。
///
/// 聚合 Agent、事件总线、审批策略与会话存储。
/// 会话运行不依赖任何 UI——即使消费者全部退出，`run_task` 也会继续执行。
pub struct CodingSession {
    agent: Arc<dyn Agent>,
    bus: Arc<EventBus>,
    policy: Arc<ApprovalToolPolicy>,
    store: FileSessionStore,
    record: Mutex<SessionRecord>,
    workspace: PathBuf,
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
    ) -> Result<Arc<Self>, AgentError> {
        let record = SessionRecord::new(workspace);
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
        }))
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

    /// 执行一个任务（完整闭环：流式事件 → 翻译 → 持久化）。
    ///
    /// # Errors
    ///
    /// Agent 输入构造失败或 Agent 运行出错时返回 [`AgentError`]。
    /// 注意：工具层面的失败（含审批拒绝）不会中断任务——它们以结构化
    /// 结果回传给模型，由模型决定后续动作。
    pub async fn run_task(&self, task: &str) -> Result<TaskOutcome, AgentError> {
        let started_at = Instant::now();
        let task_text = task.to_string();
        self.publish_and_record(
            AgentEvent::TaskStarted { task: task_text.clone() },
            |record| {
                record.tasks.push(task_text.clone());
                record.transcript.push(TranscriptEntry::Task { task: task_text });
            },
        );

        let input = rucora_core::agent::AgentInput::new(task.to_string())?;
        let stream = self.agent.run_stream(input);
        let (text, tool_calls, failure) = self.consume_stream(stream).await;

        let duration_ms = started_at.elapsed().as_millis() as u64;
        self.finish_task(&text, tool_calls, duration_ms, failure.as_deref());

        match failure {
            None => Ok(TaskOutcome {
                text,
                tool_calls,
                duration_ms,
                success: true,
            }),
            Some(message) => Err(AgentError::Message(message)),
        }
    }

    /// 消费 Agent 的 ChannelEvent 流，翻译并发布 AgentEvent。
    ///
    /// 返回（累积文本，工具调用数，失败原因）。
    async fn consume_stream(
        &self,
        mut stream: BoxStream<'static, Result<ChannelEvent, AgentError>>,
    ) -> (String, usize, Option<String>) {
        let mut final_text = String::new();
        let mut tool_calls = 0usize;
        let mut failure: Option<String> = None;
        // call_id → 调用信息（ToolResult 只带 call_id，名字/输入由配对的 ToolCall 提供）
        let mut call_infos: HashMap<String, ToolCallInfo> = HashMap::new();
        // 当前步骤累积的文本段（Message 事件用于切分步骤）
        let mut step_text = String::new();

        while let Some(item) = stream.next().await {
            match item {
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
                    call_infos.insert(
                        call.id.clone(),
                        ToolCallInfo {
                            name: call.name.clone(),
                            input: call.input.clone(),
                        },
                    );
                    self.bus.publish(AgentEvent::ToolStarted {
                        call_id: call.id,
                        name: call.name,
                        input: call.input,
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
                // Skill / Memory / Debug / Raw 事件对 UI 非必需，M0 忽略
                Ok(_) => {}
                Err(err) => {
                    failure = Some(err.to_string());
                    break;
                }
            }
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

    /// 任务收尾：更新状态、写转录、发布完成/失败事件。
    fn finish_task(&self, text: &str, tool_calls: usize, duration_ms: u64, failure: Option<&str>) {
        let status = if failure.is_some() {
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
            record.touch();
        }
        self.persist();

        match failure {
            None => {
                debug!(tool_calls, duration_ms, "coding.session.task_completed");
                self.bus.publish(AgentEvent::TaskCompleted {
                    text: text.to_string(),
                    tool_calls,
                    duration_ms,
                });
            }
            Some(message) => {
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
