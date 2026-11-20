//! 统一 Agent 事件流。
//!
//! [`AgentEvent`] 是 GUI / CLI / 日志等前端消费的唯一事件源；[`EventBus`]
//! 提供带序号的事件发布、订阅与历史回放：
//!
//! - 每个事件附带单调递增的序号（`u64`），消费者可据此做断线重连与去重
//! - 新订阅者会先拿到完整历史再接收实时事件（"快照 + 增量"语义）
//! - 事件同时在内存中保留全量历史，供 `history()` 快照查询

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::Value;
use tokio::sync::broadcast;
use tracing::debug;

/// 广播通道容量：订阅者落后超过该数量时需依赖历史回放补齐。
const BROADCAST_CAPACITY: usize = 1024;

/// 历史缓冲上限：超出后丢弃最旧事件（防长会话内存膨胀）。
const HISTORY_CAPACITY: usize = 10_000;

/// Agent 运行过程中的统一事件。
///
/// 设计原则：对 UI 足够（渲染对话/工具/审批/结果），对运行时无侵入
/// （不携带不可序列化的对象，全部字段可 JSON 化，便于落盘与跨进程）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AgentEvent {
    /// 会话已创建。
    SessionStarted {
        /// 会话 ID（对应持久化的 SessionRecord.id）。
        session_id: String,
    },

    /// 用户提交了一个任务，Agent 开始处理。
    TaskStarted {
        /// 用户任务文本。
        task: String,
    },

    /// 助手流式文本增量（可能包含思考/中间说明，跨步骤累积）。
    MessageDelta {
        /// 增量文本。
        delta: String,
    },

    /// 一步推理完成（不含工具调用的纯文本回复，或中间步骤的文本段）。
    AssistantMessage {
        /// 该步骤的完整文本。
        text: String,
        /// 该步骤是否伴随工具调用。
        has_tool_calls: bool,
    },

    /// 工具调用开始。
    ToolStarted {
        /// 工具调用 ID（与 ToolCompleted 关联）。
        call_id: String,
        /// 工具名称。
        name: String,
        /// 工具输入参数。
        input: Value,
    },

    /// 工具调用完成。
    ToolCompleted {
        /// 工具调用 ID。
        call_id: String,
        /// 工具名称。
        name: String,
        /// 工具输出（已由执行层限幅/脱敏）。
        output: Value,
        /// 是否执行成功。
        success: bool,
    },

    /// 需要用户审批（由权限策略发出，等待裁决）。
    ApprovalRequired {
        /// 审批请求 ID（调用 `resolve` 时使用）。
        request_id: String,
        /// 待审批的工具名称。
        tool_name: String,
        /// 工具输入参数。
        input: Value,
        /// 请求审批的原因（如"策略要求人工确认"）。
        reason: String,
    },

    /// 审批已裁决。
    ApprovalResolved {
        /// 审批请求 ID。
        request_id: String,
        /// 裁决结果：`allow` / `allow_always` / `deny`。
        decision: String,
    },

    /// 任务完成。
    TaskCompleted {
        /// 最终回复文本（全部增量的累积）。
        text: String,
        /// 执行的工具调用次数。
        tool_calls: usize,
        /// 任务总耗时（毫秒）。
        duration_ms: u64,
    },

    /// 任务失败。
    Failed {
        /// 错误信息。
        message: String,
    },

    /// 任务被用户取消。
    Cancelled,
}

/// 带序号的事件信封。
pub type SequencedEvent = (u64, AgentEvent);

/// 事件总线：发布、订阅与历史回放。
///
/// # 线程安全
///
/// 内部全部使用锁/原子量，`Arc<EventBus>` 可在多任务间共享。
///
/// # 示例
///
/// ```rust
/// use rucora_coding::events::{AgentEvent, EventBus};
///
/// let bus = EventBus::new();
/// let seq = bus.publish(AgentEvent::MessageDelta { delta: "你好".to_string() });
/// assert_eq!(seq, 1);
///
/// let subscription = bus.subscribe();
/// // 新订阅者先收到完整历史，再接收实时事件
/// assert_eq!(subscription.history.len(), 1);
/// ```
pub struct EventBus {
    seq: AtomicU64,
    history: Mutex<VecDeque<SequencedEvent>>,
    tx: broadcast::Sender<SequencedEvent>,
}

impl EventBus {
    /// 创建事件总线。
    pub fn new() -> Arc<Self> {
        let (tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        Arc::new(Self {
            seq: AtomicU64::new(0),
            history: Mutex::new(VecDeque::with_capacity(HISTORY_CAPACITY)),
            tx,
        })
    }

    /// 发布事件，返回分配的序号。
    pub fn publish(&self, event: AgentEvent) -> u64 {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed) + 1;
        let sequenced = (seq, event);
        {
            let mut history = self.history.lock().expect("事件历史锁中毒");
            if history.len() >= HISTORY_CAPACITY {
                history.pop_front();
            }
            history.push_back(sequenced.clone());
        }
        // 无订阅者时 send 返回 Err，属正常情况，忽略即可
        let _ = self.tx.send(sequenced);
        debug!(seq, "coding.event.published");
        seq
    }

    /// 订阅事件：返回当前全部历史 + 实时接收器。
    ///
    /// 实时接收器可能在订阅与开始消费之间出现滞后（broadcast 语义），
    /// 消费者应依据序号去重：跳过 `seq <= 历史最大序号` 的实时事件。
    pub fn subscribe(&self) -> EventSubscription {
        let history: Vec<SequencedEvent> = {
            let history = self.history.lock().expect("事件历史锁中毒");
            history.iter().cloned().collect()
        };
        EventSubscription {
            history,
            receiver: self.tx.subscribe(),
        }
    }

    /// 当前事件历史快照（不带实时接收器）。
    pub fn history(&self) -> Vec<SequencedEvent> {
        self.history
            .lock()
            .expect("事件历史锁中毒")
            .iter()
            .cloned()
            .collect()
    }
}

/// 订阅结果：历史快照 + 实时通道。
pub struct EventSubscription {
    /// 订阅时刻的全部历史事件（含序号）。
    pub history: Vec<SequencedEvent>,
    /// 实时事件接收器。
    pub receiver: broadcast::Receiver<SequencedEvent>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn delta(text: &str) -> AgentEvent {
        AgentEvent::MessageDelta {
            delta: text.to_string(),
        }
    }

    #[tokio::test]
    async fn test_publish_assigns_monotonic_seq() {
        let bus = EventBus::new();
        let s1 = bus.publish(delta("a"));
        let s2 = bus.publish(delta("b"));
        let s3 = bus.publish(AgentEvent::Failed {
            message: "x".to_string(),
        });
        assert_eq!((s1, s2, s3), (1, 2, 3));
    }

    #[tokio::test]
    async fn test_subscribe_replays_history_and_receives_live() {
        let bus = EventBus::new();
        bus.publish(delta("a"));
        bus.publish(delta("b"));

        let mut sub = bus.subscribe();
        assert_eq!(sub.history.len(), 2);
        assert_eq!(sub.history[0].0, 1);

        // 订阅后发布的新事件应实时到达
        bus.publish(delta("c"));
        let (seq, ev) = sub.receiver.recv().await.expect("实时事件应可达");
        assert_eq!(seq, 3);
        assert_eq!(ev, delta("c"));

        // 历史快照不受订阅影响
        assert_eq!(bus.history().len(), 3);
    }

    #[tokio::test]
    async fn test_history_capacity_bound() {
        let bus = EventBus::new();
        for i in 0..(HISTORY_CAPACITY + 100) {
            bus.publish(delta(&i.to_string()));
        }
        let history = bus.history();
        assert_eq!(history.len(), HISTORY_CAPACITY);
        // 最旧的事件被丢弃，序号仍然连续
        assert_eq!(history[0].0, (HISTORY_CAPACITY + 100 - HISTORY_CAPACITY + 1) as u64);
    }
}
