//! rucora-coding — Coding Agent 运行时
//!
//! 在 rucora 通用 LLM 框架之上，为 AI 编程工作台补齐运行时层：
//!
//! - [`events`]：统一 [`AgentEvent`] 事件流（序号、订阅、历史回放），
//!   GUI/CLI 只消费事件，不感知 Provider 差异
//! - [`permissions`]：工具调用权限与交互式审批
//!   （`ApprovalRequired` → 人工裁决 → 继续/以结构化结果拒绝）
//! - [`session`]：会话记录与 JSON 持久化
//! - [`runner`]：[`CodingSession`] 把 Agent、事件、审批、持久化粘合成
//!   "接收任务 → 理解项目 → 调用工具 → 审批 → 完成"的完整闭环
//!
//! # 与其他 crate 的关系
//!
//! - `rucora` 提供 Agent 执行引擎、工具、Provider（本 crate 的直接依赖）
//! - `rgpui`（工作台 GUI）只依赖本 crate 暴露的事件与 API，不感知 LLM 细节
//!
//! # 快速开始
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use rucora::agent::ToolAgent;
//! use rucora::prelude::Agent;
//! use rucora::tools::{FileEditTool, FileReadTool};
//! use rucora_coding::events::EventBus;
//! use rucora_coding::permissions::{ApprovalToolPolicy, PermissionConfig};
//! use rucora_coding::session::FileSessionStore;
//! use rucora_coding::runner::CodingSession;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // 1. 组装 Agent（审批策略 + 事件观测）
//! let bus = EventBus::new();
//! let policy = Arc::new(ApprovalToolPolicy::new(
//!     PermissionConfig::default(),
//!     "demo".to_string(),
//!     bus.clone(),
//! ));
//! let provider = Arc::new(rucora::provider::OpenAiProvider::from_env()?);
//! let agent = ToolAgent::builder(provider.clone())
//!     .model("gpt-5-mini")
//!     .system_prompt("你是有用的编程助手")
//!     .tool(FileReadTool::new())
//!     .tool(FileEditTool::new())
//!     .with_policy(policy.clone())
//!     .build();
//!
//! // 2. 启动会话（事件流 + 审批 + 持久化）
//! let store = FileSessionStore::new("./sessions")?;
//! let session = CodingSession::start(
//!     Arc::new(agent),
//!     std::path::Path::new("."),
//!     store,
//!     bus.clone(),
//!     policy,
//!     None,
//! )?;
//!
//! // 3. 后台消费事件（GUI 渲染用）
//! let mut subscription = session.events().subscribe();
//! tokio::spawn(async move {
//!     // 先渲染 subscription.history，再消费 subscription.receiver ...
//! });
//!
//! // 4. 运行任务（中途 ApprovalRequired 事件可调用
//! //    session.resolve_approval(request_id, ApprovalDecision::Allow)）
//! let outcome = session.run_task("给 utils.rs 补充单元测试并运行").await?;
//! println!("完成：{}", outcome.text);
//! # Ok(())
//! # }
//! ```

pub mod context;
pub mod diff;
pub mod events;
pub mod permissions;
pub mod runner;
pub mod session;

pub use context::{
    ContextAssembler, ContextConfig, ProjectInstructions, RepoMap, estimate_tokens,
};
pub use diff::{ChangeRecorder, ChangeTracker, FileChange, FileDiff, FILE_MUTATING_TOOLS, git_diff};
pub use events::{AgentEvent, EventSubscription, EventBus, SequencedEvent};
pub use permissions::{
    ApprovalDecision, ApprovalRequest, ApprovalToolPolicy, PermissionAction, PermissionConfig,
    ToolPermissionRule,
};
pub use runner::{CodingSession, TaskOutcome};
pub use session::{FileSessionStore, SessionRecord, SessionStatus, SharedSessionStore, TranscriptEntry};
