//! 多会话管理器。
//!
//! [`SessionManager`] 管理多个并发 [`CodingSession`]：
//! 启动/恢复/查询/取消，并把各会话的待审批请求聚合成统一路由表
//! （UI 只需订阅一个管理器视角即可看到全部会话的审批需求）。
//!
//! 装配采用**蓝图反转**：管理器生成会话 ID 与事件总线，通过
//! [`SessionBlueprint`] 交给调用方的工厂闭包去构建 Agent 与审批策略
//! （策略必须在 Agent 构建期通过 `with_policy` 接入），随后管理器完成
//! 会话启动与注册。
//!
//! # 示例
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use rucora::agent::ToolAgent;
//! use rucora_coding::events::EventBus;
//! use rucora_coding::manager::SessionManager;
//! use rucora_coding::permissions::{ApprovalToolPolicy, PermissionConfig};
//! use rucora_coding::session::FileSessionStore;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let manager = SessionManager::new(FileSessionStore::new("./sessions")?);
//!
//! let session = manager.launch(std::path::Path::new("."), |blueprint| {
//!     let policy = Arc::new(ApprovalToolPolicy::new(
//!         PermissionConfig::default(),
//!         blueprint.session_id.clone(),
//!         blueprint.bus.clone(),
//!     ));
//!     let provider = Arc::new(rucora::provider::OpenAiProvider::from_env()?);
//!     let agent = ToolAgent::builder(provider)
//!         .model("gpt-5-mini")
//!         .with_policy(policy.clone())
//!         .build();
//!     Ok((Arc::new(agent), policy, None))
//! })?;
//!
//! // 聚合全部会话的待审批请求
//! for route in manager.pending_approvals() {
//!     manager.resolve_approval(&route.session_id, &route.request.request_id,
//!         rucora_coding::permissions::ApprovalDecision::Allow);
//! }
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rucora_core::agent::AgentError;
use tracing::{debug, info, warn};

use crate::diff::ChangeTracker;
use crate::events::EventBus;
use crate::permissions::{ApprovalDecision, ApprovalRequest, ApprovalToolPolicy};
use crate::runner::CodingSession;
use crate::session::{FileSessionStore, SessionRecord};

/// 会话装配蓝图：管理器生成，交给工厂闭包构建 Agent 与审批策略。
#[derive(Clone)]
pub struct SessionBlueprint {
    /// 本次会话的稳定 ID（与 SessionRecord.id 一致）。
    pub session_id: String,
    /// 会话专属事件总线。
    pub bus: Arc<EventBus>,
    /// 工作目录。
    pub workspace: PathBuf,
    /// 恢复模式下为源记录 ID；新启动为 `None`。
    pub resume_from: Option<String>,
}

impl std::fmt::Debug for SessionBlueprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionBlueprint")
            .field("session_id", &self.session_id)
            .field("workspace", &self.workspace)
            .field("resume_from", &self.resume_from)
            .finish()
    }
}

/// 工厂闭包的返回值：装配完成的 Agent、审批策略与可选变更追踪器。
pub type SessionParts = (
    Arc<dyn rucora_core::agent::Agent>,
    Arc<ApprovalToolPolicy>,
    Option<Arc<ChangeTracker>>,
);

/// 待审批请求的会话路由。
#[derive(Debug, Clone, PartialEq)]
pub struct ApprovalRoute {
    /// 所属会话 ID。
    pub session_id: String,
    /// 待审批请求。
    pub request: ApprovalRequest,
}

/// 多会话管理器。
///
/// 持有活动会话表与持久化存储；活动会话与持久化记录是两个维度：
/// `detach` 只移除活动表（任务继续后台执行），`delete_record` 删除磁盘记录
/// （仅对未注册的活动会话允许）。
pub struct SessionManager {
    store: FileSessionStore,
    sessions: Mutex<HashMap<String, Arc<CodingSession>>>,
}

impl SessionManager {
    /// 创建管理器。
    pub fn new(store: FileSessionStore) -> Arc<Self> {
        Arc::new(Self {
            store,
            sessions: Mutex::new(HashMap::new()),
        })
    }

    /// 持久化存储句柄。
    pub fn store(&self) -> &FileSessionStore {
        &self.store
    }

    /// 启动新会话。
    ///
    /// `setup` 接收装配蓝图，返回装配完成的 Agent、审批策略与可选追踪器。
    /// 会话启动后自动注册进活动表。
    ///
    /// # Errors
    ///
    /// 蓝图装配失败或会话启动失败时返回 [`AgentError`]。
    pub fn launch<F>(&self, workspace: &Path, setup: F) -> Result<Arc<CodingSession>, AgentError>
    where
        F: FnOnce(&SessionBlueprint) -> Result<SessionParts, AgentError>,
    {
        let session_id = uuid::Uuid::new_v4().to_string();
        let bus = EventBus::new();
        let blueprint = SessionBlueprint {
            session_id: session_id.clone(),
            bus: bus.clone(),
            workspace: workspace.to_path_buf(),
            resume_from: None,
        };

        let (agent, policy, tracker) = setup(&blueprint)?;
        let session = CodingSession::start(
            agent,
            workspace,
            self.store.clone(),
            bus,
            policy,
            tracker,
            Some(&session_id),
        )?;

        self.register(&session_id, session.clone());
        Ok(session)
    }

    /// 从持久化记录恢复会话并注册。
    ///
    /// # Errors
    ///
    /// 记录不存在或会话恢复失败时返回 [`AgentError`]。
    pub fn resume_session<F>(
        &self,
        workspace: &Path,
        record_id: &str,
        setup: F,
    ) -> Result<Arc<CodingSession>, AgentError>
    where
        F: FnOnce(&SessionBlueprint) -> Result<SessionParts, AgentError>,
    {
        let bus = EventBus::new();
        let blueprint = SessionBlueprint {
            session_id: record_id.to_string(),
            bus: bus.clone(),
            workspace: workspace.to_path_buf(),
            resume_from: Some(record_id.to_string()),
        };

        let (agent, policy, tracker) = setup(&blueprint)?;
        let session = CodingSession::resume(
            agent,
            workspace,
            self.store.clone(),
            bus,
            policy,
            tracker,
            record_id,
        )?;

        self.register(record_id, session.clone());
        Ok(session)
    }

    /// 注册活动会话。
    fn register(&self, session_id: &str, session: Arc<CodingSession>) {
        self.sessions
            .lock()
            .expect("活动会话表锁中毒")
            .insert(session_id.to_string(), session);
        info!(session_id = %session_id, active = self.active_count(), "coding.manager.registered");
    }

    /// 活动会话数。
    pub fn active_count(&self) -> usize {
        self.sessions.lock().expect("活动会话表锁中毒").len()
    }

    /// 活动会话 ID 列表。
    pub fn active_ids(&self) -> Vec<String> {
        self.sessions
            .lock()
            .expect("活动会话表锁中毒")
            .keys()
            .cloned()
            .collect()
    }

    /// 按 ID 获取活动会话。
    pub fn get(&self, session_id: &str) -> Option<Arc<CodingSession>> {
        self.sessions
            .lock()
            .expect("活动会话表锁中毒")
            .get(session_id)
            .cloned()
    }

    /// 全部持久化会话记录（按更新时间降序，含活动与已结束）。
    pub fn all_records(&self) -> Vec<SessionRecord> {
        self.store.list()
    }

    /// 聚合全部活动会话的待审批请求。
    pub fn pending_approvals(&self) -> Vec<ApprovalRoute> {
        let sessions = self.sessions.lock().expect("活动会话表锁中毒");
        sessions
            .iter()
            .flat_map(|(session_id, session)| {
                session
                    .pending_approvals()
                    .into_iter()
                    .map(move |request| ApprovalRoute {
                        session_id: session_id.clone(),
                        request,
                    })
            })
            .collect()
    }

    /// 通过管理器裁决某会话的审批请求。
    ///
    /// # 返回
    ///
    /// 会话存在且裁决送达时返回 `true`。
    pub fn resolve_approval(
        &self,
        session_id: &str,
        request_id: &str,
        decision: ApprovalDecision,
    ) -> bool {
        match self.get(session_id) {
            Some(session) => session.resolve_approval(request_id, decision),
            None => {
                warn!(session_id, request_id, "coding.manager.resolve_miss");
                false
            }
        }
    }

    /// 取消全部活动会话的当前任务。
    ///
    /// # 返回
    ///
    /// 被取消的会话数。
    pub fn cancel_all(&self) -> usize {
        let sessions: Vec<Arc<CodingSession>> = self
            .sessions
            .lock()
            .expect("活动会话表锁中毒")
            .values()
            .cloned()
            .collect();
        for session in &sessions {
            session.cancel();
        }
        debug!(count = sessions.len(), "coding.manager.cancel_all");
        sessions.len()
    }

    /// 从活动表移除会话（不影响其后台执行与持久化记录）。
    ///
    /// # 返回
    ///
    /// 会话存在且已移除时返回 `true`。
    pub fn detach(&self, session_id: &str) -> bool {
        let removed = self
            .sessions
            .lock()
            .expect("活动会话表锁中毒")
            .remove(session_id)
            .is_some();
        if removed {
            debug!(session_id, "coding.manager.detached");
        }
        removed
    }

    /// 删除持久化会话记录。
    ///
    /// 会话仍在活动表注册时拒绝删除（先 `detach`）。
    ///
    /// # 返回
    ///
    /// 记录存在且已删除时返回 `true`。
    ///
    /// # Errors
    ///
    /// 磁盘删除失败时返回 IO 错误。
    pub fn delete_record(&self, session_id: &str) -> Result<bool, AgentError> {
        if self.get(session_id).is_some() {
            return Err(AgentError::Message(format!(
                "会话 {session_id} 仍在活动中，请先 detach 再删除"
            )));
        }
        let existed = self.store.load(session_id).is_ok();
        if !existed {
            return Ok(false);
        }
        self.store
            .delete(session_id)
            .map(|_| {
                debug!(session_id, "coding.manager.record_deleted");
                true
            })
            .map_err(|e| AgentError::Message(format!("会话记录删除失败：{e}")))
    }
}
