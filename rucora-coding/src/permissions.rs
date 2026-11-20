//! 工具调用权限与交互式审批。
//!
//! 基于 rucora 现有的 [`ToolPolicy`](rucora::agent::ToolPolicy) 钩子实现：
//! 在工具执行前按策略决定 **放行 / 询问 / 拒绝**，其中"询问"会发布
//! [`AgentEvent::ApprovalRequired`] 事件并挂起等待人工裁决
//! （通过 [`ApprovalToolPolicy::resolve`]）。
//!
//! 拒绝以 `ToolError::PolicyDenied` 返回——执行层会将其转换为结构化的
//! `"ok": false` 工具结果反馈给模型，Agent 可以据此调整策略继续工作，
//! 而不是让整个任务中断。
//!
//! # 示例
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use rucora_coding::events::EventBus;
//! use rucora_coding::permissions::{ApprovalToolPolicy, PermissionConfig, PermissionAction};
//!
//! # fn example() {
//! let bus = EventBus::new();
//! let policy = ApprovalToolPolicy::new(
//!     PermissionConfig {
//!         default_action: PermissionAction::Ask,
//!         auto_allow_safe: true,
//!         ..PermissionConfig::default()
//!     },
//!     "session-1".to_string(),
//!     bus,
//! );
//! // 消费事件流的 GUI 在收到 ApprovalRequired 后调用：
//! // policy.resolve("req-1", ApprovalDecision::Allow);
//! # let _ = Arc::new(policy);
//! # }
//! ```

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use rucora::agent::policy::{ToolCallContext, ToolPolicy};
use rucora_core::error::ToolError;
use rucora_core::tool::types::ToolRiskLevel;
use serde_json::Value;
use tokio::sync::oneshot;
use tracing::{debug, warn};

use crate::events::{AgentEvent, EventBus};

/// 默认询问超时（秒）：超时按拒绝处理，避免任务无限挂起。
pub const DEFAULT_ASK_TIMEOUT_SECS: u64 = 300;

/// 单个工具的权限动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PermissionAction {
    /// 直接放行，不询问。
    Allow,
    /// 询问用户（人工审批）。
    Ask,
    /// 直接拒绝。
    Deny,
}

/// 权限配置。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PermissionConfig {
    /// 未命中规则时的默认动作。
    pub default_action: PermissionAction,
    /// 按工具名精确匹配的规则（优先级高于默认动作）。
    pub rules: Vec<ToolPermissionRule>,
    /// 是否自动放行 `Safe` 风险等级的工具（只读/无副作用类）。
    pub auto_allow_safe: bool,
    /// 询问超时（秒），超时按拒绝处理。
    pub ask_timeout_secs: u64,
}

impl Default for PermissionConfig {
    fn default() -> Self {
        Self {
            default_action: PermissionAction::Ask,
            rules: Vec::new(),
            auto_allow_safe: true,
            ask_timeout_secs: DEFAULT_ASK_TIMEOUT_SECS,
        }
    }
}

impl PermissionConfig {
    /// 添加按工具名的规则（builder 风格）。
    pub fn with_rule(mut self, tool_name: impl Into<String>, action: PermissionAction) -> Self {
        self.rules.push(ToolPermissionRule {
            tool_name: tool_name.into(),
            action,
        });
        self
    }
}

/// 按工具名的权限规则。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ToolPermissionRule {
    /// 工具名称（精确匹配）。
    pub tool_name: String,
    /// 命中后的动作。
    pub action: PermissionAction,
}

/// 审批裁决结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ApprovalDecision {
    /// 本次放行。
    Allow,
    /// 本次放行且记住选择（会话内后续同名工具自动放行）。
    AllowAlways,
    /// 拒绝（以结构化的 policy_denied 结果反馈给模型）。
    Deny,
}

impl ApprovalDecision {
    /// 决策的事件字符串表示。
    pub fn as_str(&self) -> &'static str {
        match self {
            ApprovalDecision::Allow => "allow",
            ApprovalDecision::AllowAlways => "allow_always",
            ApprovalDecision::Deny => "deny",
        }
    }
}

/// 待裁决的审批请求（供 UI 展示）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ApprovalRequest {
    /// 审批请求 ID（`resolve` 时使用）。
    pub request_id: String,
    /// 所属会话 ID。
    pub session_id: String,
    /// 待审批的工具名称。
    pub tool_name: String,
    /// 工具风险等级（"safe" / "caution" / "dangerous"）。
    pub risk: String,
    /// 工具输入参数。
    pub input: Value,
    /// 请求审批的原因。
    pub reason: String,
}

/// 交互式审批策略。
///
/// 实现 rucora 的 `ToolPolicy`，可直接传给
/// `ToolAgent::builder(...).with_policy(...)`。
pub struct ApprovalToolPolicy {
    session_id: String,
    config: PermissionConfig,
    bus: Arc<EventBus>,
    next_request_id: AtomicU64,
    /// 待裁决请求：request_id → (请求信息, 裁决通道)。
    pending: Mutex<HashMap<String, PendingApproval>>,
    /// 会话内被"总是允许"的工具名。
    always_allow: Mutex<HashSet<String>>,
}

struct PendingApproval {
    request: ApprovalRequest,
    tx: oneshot::Sender<ApprovalDecision>,
}

impl ApprovalToolPolicy {
    /// 创建审批策略。
    pub fn new(config: PermissionConfig, session_id: String, bus: Arc<EventBus>) -> Self {
        Self {
            session_id,
            config,
            bus,
            next_request_id: AtomicU64::new(1),
            pending: Mutex::new(HashMap::new()),
            always_allow: Mutex::new(HashSet::new()),
        }
    }

    /// 裁决一个待审批请求。
    ///
    /// # 返回
    ///
    /// 请求存在且已送达裁决时返回 `true`；请求不存在（已超时/已裁决）返回 `false`。
    pub fn resolve(&self, request_id: &str, decision: ApprovalDecision) -> bool {
        let removed = {
            let mut pending = self.pending.lock().expect("审批注册表锁中毒");
            pending.remove(request_id)
        };
        match removed {
            Some(entry) => {
                if decision == ApprovalDecision::AllowAlways {
                    self.always_allow
                        .lock()
                        .expect("always_allow 锁中毒")
                        .insert(entry.request.tool_name.clone());
                }
                self.bus.publish(AgentEvent::ApprovalResolved {
                    request_id: request_id.to_string(),
                    decision: decision.as_str().to_string(),
                });
                debug!(request_id, decision = decision.as_str(), "coding.approval.resolved");
                // 忽略接收端已丢弃的情况（任务取消等）
                let _ = entry.tx.send(decision);
                true
            }
            None => {
                warn!(request_id, "coding.approval.resolve_miss");
                false
            }
        }
    }

    /// 当前待裁决的请求快照（供 UI 恢复显示）。
    pub fn pending_requests(&self) -> Vec<ApprovalRequest> {
        self.pending
            .lock()
            .expect("审批注册表锁中毒")
            .values()
            .map(|entry| entry.request.clone())
            .collect()
    }

    /// 会话内已被"总是允许"的工具名列表。
    pub fn always_allowed_tools(&self) -> Vec<String> {
        self.always_allow
            .lock()
            .expect("always_allow 锁中毒")
            .iter()
            .cloned()
            .collect()
    }

    /// 决定一个工具调用的动作。
    fn decide(&self, tool_name: &str, risk: ToolRiskLevel) -> PermissionAction {
        for rule in &self.config.rules {
            if rule.tool_name == tool_name {
                return rule.action;
            }
        }
        if self.config.auto_allow_safe && risk == ToolRiskLevel::Safe {
            return PermissionAction::Allow;
        }
        self.config.default_action
    }
}

#[async_trait]
impl ToolPolicy for ApprovalToolPolicy {
    async fn check(&self, ctx: &ToolCallContext) -> Result<(), ToolError> {
        let tool_name = ctx.tool_call.name.clone();
        let risk = ctx
            .identity
            .as_ref()
            .map_or(ToolRiskLevel::Caution, |identity| identity.risk_level);

        match self.decide(&tool_name, risk) {
            PermissionAction::Allow => Ok(()),
            PermissionAction::Deny => Err(ToolError::PolicyDenied {
                rule_id: "coding.permission".to_string(),
                reason: format!("工具 '{tool_name}' 被权限策略拒绝"),
            }),
            PermissionAction::Ask => {
                if self
                    .always_allow
                    .lock()
                    .expect("always_allow 锁中毒")
                    .contains(&tool_name)
                {
                    return Ok(());
                }

                let request_id = format!(
                    "req-{}-{}",
                    self.session_id,
                    self.next_request_id.fetch_add(1, Ordering::Relaxed)
                );
                let request = ApprovalRequest {
                    request_id: request_id.clone(),
                    session_id: self.session_id.clone(),
                    tool_name: tool_name.clone(),
                    risk: risk.as_str().to_string(),
                    input: ctx.tool_call.input.clone(),
                    reason: format!("策略要求人工确认 '{tool_name}'（风险：{}）", risk.as_str()),
                };

                let (tx, rx) = oneshot::channel();
                self.pending
                    .lock()
                    .expect("审批注册表锁中毒")
                    .insert(request_id.clone(), PendingApproval { request: request.clone(), tx });

                self.bus.publish(AgentEvent::ApprovalRequired {
                    request_id: request_id.clone(),
                    tool_name,
                    input: request.input.clone(),
                    reason: request.reason.clone(),
                });

                let timeout = Duration::from_secs(self.config.ask_timeout_secs);
                let decision = match tokio::time::timeout(timeout, rx).await {
                    Ok(Ok(decision)) => Some(decision),
                    Ok(Err(_recv_dropped)) => {
                        // 裁决通道被丢弃（会话销毁），按拒绝处理
                        None
                    }
                    Err(_elapsed) => {
                        warn!(request_id = %request_id, "coding.approval.timeout");
                        None
                    }
                };

                // 从注册表移除（超时/通道关闭时 resolve 不再可用）
                self.pending
                    .lock()
                    .expect("审批注册表锁中毒")
                    .remove(&request_id);

                match decision {
                    Some(ApprovalDecision::Allow) => Ok(()),
                    Some(ApprovalDecision::AllowAlways) => {
                        self.always_allow
                            .lock()
                            .expect("always_allow 锁中毒")
                            .insert(request.tool_name.clone());
                        Ok(())
                    }
                    // Deny 的 ApprovalResolved 事件已由 resolve() 发布，此处不再重复
                    Some(ApprovalDecision::Deny) => Err(ToolError::PolicyDenied {
                        rule_id: "coding.permission".to_string(),
                        reason: format!("用户拒绝了工具 '{0}' 的调用", request.tool_name),
                    }),
                    // 超时/裁决通道关闭：resolve 未发生，这里补发事件并按拒绝处理
                    None => {
                        self.bus.publish(AgentEvent::ApprovalResolved {
                            request_id,
                            decision: ApprovalDecision::Deny.as_str().to_string(),
                        });
                        Err(ToolError::PolicyDenied {
                            rule_id: "coding.permission".to_string(),
                            reason: format!(
                                "工具 '{0}' 审批超时或已失效，已按拒绝处理",
                                request.tool_name
                            ),
                        })
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rucora_core::tool::types::{ToolCall, ToolSource};

    fn ctx(name: &str, risk: ToolRiskLevel) -> ToolCallContext {
        ToolCallContext {
            tool_call: ToolCall {
                id: "call-1".to_string(),
                name: name.to_string(),
                input: serde_json::json!({"path": "src/main.rs"}),
            },
            identity: Some(rucora_core::tool::types::ToolIdentity::new(
                name,
                ToolSource::Custom,
                vec![],
                risk,
            )),
        }
    }

    fn policy_with(config: PermissionConfig) -> (Arc<ApprovalToolPolicy>, Arc<EventBus>) {
        let bus = EventBus::new();
        let policy = Arc::new(ApprovalToolPolicy::new(
            config,
            "test-session".to_string(),
            bus.clone(),
        ));
        (policy, bus)
    }

    #[tokio::test]
    async fn test_allow_rule_passes() {
        let (policy, _bus) = policy_with(
            PermissionConfig::default().with_rule("file_read", PermissionAction::Allow),
        );
        assert!(policy.check(&ctx("file_read", ToolRiskLevel::Safe)).await.is_ok());
    }

    #[tokio::test]
    async fn test_deny_rule_returns_policy_denied() {
        let (policy, _bus) = policy_with(
            PermissionConfig::default().with_rule("shell", PermissionAction::Deny),
        );
        let err = policy.check(&ctx("shell", ToolRiskLevel::Dangerous)).await;
        assert!(matches!(err, Err(ToolError::PolicyDenied { .. })));
    }

    #[tokio::test]
    async fn test_safe_auto_allowed() {
        let (policy, _bus) = policy_with(PermissionConfig::default());
        assert!(policy.check(&ctx("file_read", ToolRiskLevel::Safe)).await.is_ok());
    }

    #[tokio::test]
    async fn test_ask_emits_event_and_resolves() {
        let (policy, bus) = policy_with(PermissionConfig::default());

        // 后台任务在收到 ApprovalRequired 事件后放行
        let waiter = {
            let policy = policy.clone();
            let mut sub = bus.subscribe();
            tokio::spawn(async move {
                // 跳过历史（可能为空），等待 ApprovalRequired
                loop {
                    match sub.receiver.recv().await {
                        Ok((_, AgentEvent::ApprovalRequired { request_id, .. })) => {
                            assert!(policy.resolve(&request_id, ApprovalDecision::Allow));
                            return request_id;
                        }
                        Ok(_) => continue,
                        Err(_) => panic!("事件通道关闭"),
                    }
                }
            })
        };

        let result = policy.check(&ctx("file_edit", ToolRiskLevel::Caution)).await;
        assert!(result.is_ok());
        let request_id = waiter.await.expect("审批任务不应 panic");
        assert!(request_id.starts_with("req-test-session-"));
    }

    #[tokio::test]
    async fn test_allow_always_remembers() {
        let (policy, bus) = policy_with(PermissionConfig::default());

        let waiter = {
            let policy = policy.clone();
            let mut sub = bus.subscribe();
            tokio::spawn(async move {
                loop {
                    match sub.receiver.recv().await {
                        Ok((_, AgentEvent::ApprovalRequired { request_id, .. })) => {
                            assert!(policy.resolve(&request_id, ApprovalDecision::AllowAlways));
                            return;
                        }
                        Ok(_) => continue,
                        Err(_) => panic!("事件通道关闭"),
                    }
                }
            })
        };

        assert!(policy.check(&ctx("file_edit", ToolRiskLevel::Caution)).await.is_ok());
        waiter.await.expect("审批任务不应 panic");

        // 第二次调用直接放行（AllowAlways 生效）
        assert!(policy.check(&ctx("file_edit", ToolRiskLevel::Caution)).await.is_ok());
        assert_eq!(policy.always_allowed_tools(), vec!["file_edit"]);
    }

    #[tokio::test]
    async fn test_deny_via_resolve() {
        let (policy, bus) = policy_with(PermissionConfig::default());

        let waiter = {
            let policy = policy.clone();
            let mut sub = bus.subscribe();
            tokio::spawn(async move {
                loop {
                    match sub.receiver.recv().await {
                        Ok((_, AgentEvent::ApprovalRequired { request_id, .. })) => {
                            assert!(policy.resolve(&request_id, ApprovalDecision::Deny));
                            return;
                        }
                        Ok(_) => continue,
                        Err(_) => panic!("事件通道关闭"),
                    }
                }
            })
        };

        let err = policy.check(&ctx("file_edit", ToolRiskLevel::Caution)).await;
        assert!(matches!(err, Err(ToolError::PolicyDenied { reason, .. }) if reason.contains("用户拒绝")));
        waiter.await.expect("审批任务不应 panic");
    }

    #[tokio::test]
    async fn test_ask_timeout_denies() {
        let (policy, _bus) = policy_with(PermissionConfig {
            ask_timeout_secs: 1,
            ..PermissionConfig::default()
        });
        let err = policy.check(&ctx("file_edit", ToolRiskLevel::Caution)).await;
        assert!(matches!(err, Err(ToolError::PolicyDenied { reason, .. }) if reason.contains("超时")));
        // 超时后无遗留待审批请求
        assert!(policy.pending_requests().is_empty());
    }
}
