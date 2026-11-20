//! 子 Agent 编排。
//!
//! [`SubAgentTool`] 把"派生子任务"实现为一个普通工具（与 Claude Code 的
//! Task 工具同构）：主 Agent 的工具循环零改动即可派生一个拥有独立上下文
//! 窗口的子 Agent，子任务跑完后以结构化结果回传主循环。
//!
//! 关键行为：
//! - **上下文隔离**：子 Agent 只看到自己的系统提示词与子任务描述，
//!   不继承主会话历史
//! - **事件上报**：子 Agent 的流式输出经 [`SubAgentObserver`] 桥接到
//!   父会话事件总线（`SubAgentDelta` / `SubAgentToolStarted` /
//!   `SubAgentToolCompleted`），UI 可实时缩进渲染子任务进度
//! - **变更追踪**：工厂闭包内复用父会话的 `ChangeTracker` 构建工具包装，
//!   子任务编辑的文件同样可回滚
//! - **超时保护**：可配置执行超时（默认 600 秒），超时以 `ok:false`
//!   的结构化结果返回主 Agent，不中断主循环
//!
//! # 示例
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use rucora::agent::ToolAgent;
//! use rucora::tools::FileReadTool;
//! use rucora_coding::subagent::{SubAgentObserver, SubAgentTool};
//!
//! # fn example(bus: Arc<rucora_coding::events::EventBus>,
//! #            policy: Arc<rucora_coding::permissions::ApprovalToolPolicy>,
//! #            workspace: std::path::PathBuf) {
//! let workspace_for_sub = workspace.clone();
//! let subagent = SubAgentTool::new(
//!     move |_task| {
//!         let provider = Arc::new(rucora::provider::OpenAiProvider::from_env()
//!             .map_err(|e| rucora_core::error::ToolError::message(e.to_string()))?);
//!         let agent = ToolAgent::builder(provider)
//!             .model("gpt-5-mini")
//!             .system_prompt("你是子任务执行者：完成单一目标后立即汇报结果")
//!             .tool(FileReadTool::new().with_allowed_dirs(vec![workspace_for_sub.clone()]))
//!             .with_policy(policy.clone())
//!             .with_observer(SubAgentObserver::new(bus.clone()))
//!             .build();
//!         Ok(Arc::new(agent))
//!     },
//!     workspace.clone(),
//! );
//! # let _ = subagent;
//! # }
//! ```

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures_util::StreamExt;
use rucora_core::channel::types::ChannelEvent;
use rucora_core::channel::ChannelObserver;
use rucora_core::error::ToolError;
use rucora_core::tool::types::{ToolContext, ToolRiskLevel};
use rucora_core::tool::ToolCategory;
use rucora_core::tool::Tool;
use serde_json::{Value, json};
use tracing::{debug, warn};

use crate::events::{AgentEvent, EventBus};

/// 默认子任务执行超时（秒）。
pub const DEFAULT_SUBAGENT_TIMEOUT_SECS: u64 = 600;

/// 子 Agent 执行观察器：把子 Agent 的 ChannelEvent 桥接为父会话的
/// 嵌套事件（`SubAgentDelta` 等），UI 可实时渲染子任务进度。
pub struct SubAgentObserver {
    bus: Arc<EventBus>,
    /// call_id → 工具名（ToolResult 事件只带 call_id）。
    call_names: Mutex<HashMap<String, String>>,
}

impl SubAgentObserver {
    /// 创建子 Agent 观察器。
    pub fn new(bus: Arc<EventBus>) -> Arc<Self> {
        Arc::new(Self {
            bus,
            call_names: Mutex::new(HashMap::new()),
        })
    }
}

impl ChannelObserver for SubAgentObserver {
    fn on_event(&self, event: ChannelEvent) {
        match event {
            ChannelEvent::TokenDelta(delta) => {
                self.bus.publish(AgentEvent::SubAgentDelta {
                    delta: delta.delta,
                });
            }
            ChannelEvent::ToolCall(call) => {
                self.call_names
                    .lock()
                    .expect("调用名映射锁中毒")
                    .insert(call.id.clone(), call.name.clone());
                self.bus.publish(AgentEvent::SubAgentToolStarted {
                    name: call.name,
                    input: call.input,
                });
            }
            ChannelEvent::ToolResult(result) => {
                let name = self
                    .call_names
                    .lock()
                    .expect("调用名映射锁中毒")
                    .remove(&result.tool_call_id)
                    .unwrap_or_default();
                self.bus.publish(AgentEvent::SubAgentToolCompleted {
                    name,
                    success: result.success,
                });
            }
            _ => {}
        }
    }
}

/// 子任务派生工具。
///
/// 每次调用通过工厂闭包构建一个全新的 Agent（独立上下文），
/// 以 `run_stream` 执行到结束，最终文本作为工具结果回传主循环。
/// 工厂闭包负责把子 Agent 的观测器（[`SubAgentObserver`]）、审批策略与
/// 变更追踪器装配进子 Agent（通常与父会话共享同一套）。
pub struct SubAgentTool<F>
where
    F: Fn(&str) -> Result<Arc<dyn rucora_core::agent::Agent>, ToolError> + Send + Sync,
{
    factory: F,
    workspace: PathBuf,
    timeout: Duration,
}

impl<F> SubAgentTool<F>
where
    F: Fn(&str) -> Result<Arc<dyn rucora_core::agent::Agent>, ToolError> + Send + Sync,
{
    /// 创建子任务工具。
    ///
    /// # 参数
    ///
    /// - `factory`: 工厂闭包，入参为子任务文本，返回装配完成的子 Agent。
    ///   工厂内应通过 `with_observer(SubAgentObserver::new(bus))` 把子任务
    ///   进度桥接到父会话事件总线，并复用父策略与追踪器
    /// - `workspace`: 工作目录（写入结果元数据）
    pub fn new(factory: F, workspace: impl Into<PathBuf>) -> Self {
        Self {
            factory,
            workspace: workspace.into(),
            timeout: Duration::from_secs(DEFAULT_SUBAGENT_TIMEOUT_SECS),
        }
    }

    /// 设置子任务执行超时。
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

#[async_trait]
impl<F> Tool for SubAgentTool<F>
where
    F: Fn(&str) -> Result<Arc<dyn rucora_core::agent::Agent>, ToolError> + Send + Sync,
{
    fn name(&self) -> &str {
        "subagent"
    }

    fn description(&self) -> Option<&str> {
        Some(
            "派生一个拥有独立上下文的子 Agent 执行子任务，完成后以摘要回传。\
             适合拆解探索/调研类子目标；子任务无法再派生新的子任务",
        )
    }

    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::External]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task": {
                    "type": "string",
                    "description": "子任务描述（需自包含，子 Agent 无法看到当前对话历史）"
                }
            },
            "required": ["task"]
        })
    }

    fn risk_level(&self) -> ToolRiskLevel {
        // 派生本身无副作用；子 Agent 内的文件操作仍受其策略约束
        ToolRiskLevel::Safe
    }

    async fn call(&self, input: Value, _context: &ToolContext) -> Result<Value, ToolError> {
        let task = input
            .get("task")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::message("缺少 'task' 参数".to_string()))?
            .to_string();
        let started = Instant::now();

        // 构建子 Agent（独立上下文）
        let agent = (self.factory)(&task)?;

        let input = rucora_core::agent::AgentInput::new(task.clone())
            .map_err(|e| ToolError::message(format!("子任务输入构造失败：{e}")))?;
        let mut stream = agent.run_stream(input);

        // 消费子流：累积最终文本，统计工具调用
        let mut final_text = String::new();
        let mut tool_calls = 0usize;
        let mut failure: Option<String> = None;
        let consume = async {
            while let Some(item) = stream.next().await {
                match item {
                    Ok(ChannelEvent::TokenDelta(delta)) => {
                        final_text.push_str(&delta.delta);
                    }
                    Ok(ChannelEvent::ToolCall(call)) => {
                        tool_calls += 1;
                        debug!(tool = %call.name, "coding.subagent.tool_call");
                    }
                    Ok(ChannelEvent::Error(err)) => {
                        failure = Some(err.message);
                        break;
                    }
                    Ok(_) => {}
                    Err(err) => {
                        failure = Some(err.to_string());
                        break;
                    }
                }
            }
        };
        match tokio::time::timeout(self.timeout, consume).await {
            Ok(()) => {}
            Err(_elapsed) => {
                warn!(timeout_secs = self.timeout.as_secs(), "coding.subagent.timeout");
                return Ok(json!({
                    "ok": false,
                    "error": {
                        "kind": "subagent_timeout",
                        "message": format!("子任务超时（{} 秒），已中止", self.timeout.as_secs())
                    }
                }));
            }
        }

        if let Some(message) = failure {
            return Ok(json!({
                "ok": false,
                "error": {"kind": "subagent_failed", "message": message}
            }));
        }

        debug!(
            task_chars = task.len(),
            summary_chars = final_text.len(),
            tool_calls,
            "coding.subagent.completed"
        );

        Ok(json!({
            "ok": true,
            "summary": final_text,
            "tool_calls": tool_calls,
            "workspace": self.workspace.display().to_string(),
            "duration_ms": started.elapsed().as_millis() as u64,
        }))
    }
}
