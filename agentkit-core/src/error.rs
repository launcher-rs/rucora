//! agentkit-core 的统一错误类型定义。
//!
//! 该文件只描述错误“类别”和最小信息，不绑定具体实现细节。

/// 统一的错误诊断信息。
///
/// 设计目标：
/// - 不破坏现有错误枚举（`Message(String)` 等）与调用点。
/// - 让上层（runtime / UI / trace / policy）能拿到结构化的诊断字段。
///
/// 字段说明：
/// - `kind`：错误大类（provider/tool/runtime/skill/memory/channel/...）
/// - `message`：人类可读信息（可直接打印）
/// - `retriable`：是否建议重试（最佳努力）
/// - `source`：可选的错误来源字符串（例如底层库名、provider 名称等）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorDiagnostic {
    pub kind: &'static str,
    pub message: String,
    pub retriable: bool,
    pub source: Option<String>,
}

/// 为 core 层错误提供统一诊断能力。
pub trait DiagnosticError {
    fn diagnostic(&self) -> ErrorDiagnostic;
}

/// Provider 错误。
#[derive(thiserror::Error, Debug)]
pub enum ProviderError {
    /// 通用错误信息。
    #[error("provider error: {0}")]
    Message(String),
}

impl DiagnosticError for ProviderError {
    fn diagnostic(&self) -> ErrorDiagnostic {
        match self {
            ProviderError::Message(msg) => ErrorDiagnostic {
                kind: "provider",
                message: msg.clone(),
                // provider 错误在实践中经常可重试（网络抖动/限流/暂时不可用）。
                retriable: true,
                source: None,
            },
        }
    }
}

/// Tool 错误。
#[derive(thiserror::Error, Debug)]
pub enum ToolError {
    /// 通用错误信息。
    #[error("tool error: {0}")]
    Message(String),

    #[error("tool policy denied (rule_id={rule_id}): {reason}")]
    PolicyDenied { rule_id: String, reason: String },
}

impl DiagnosticError for ToolError {
    fn diagnostic(&self) -> ErrorDiagnostic {
        match self {
            ToolError::Message(msg) => ErrorDiagnostic {
                kind: "tool",
                message: msg.clone(),
                retriable: false,
                source: None,
            },
            ToolError::PolicyDenied { rule_id, reason } => ErrorDiagnostic {
                kind: "tool",
                message: format!("policy denied (rule_id={}): {}", rule_id, reason),
                // 策略拒绝通常不可重试（除非用户改变输入/策略）。
                retriable: false,
                source: Some(rule_id.clone()),
            },
        }
    }
}

/// Skill 错误。
#[derive(thiserror::Error, Debug)]
pub enum SkillError {
    /// 通用错误信息。
    #[error("skill error: {0}")]
    Message(String),
}

impl DiagnosticError for SkillError {
    fn diagnostic(&self) -> ErrorDiagnostic {
        match self {
            SkillError::Message(msg) => ErrorDiagnostic {
                kind: "skill",
                message: msg.clone(),
                retriable: false,
                source: None,
            },
        }
    }
}

/// Agent 错误。
#[derive(thiserror::Error, Debug)]
pub enum AgentError {
    /// 通用错误信息。
    #[error("agent error: {0}")]
    Message(String),
}

impl DiagnosticError for AgentError {
    fn diagnostic(&self) -> ErrorDiagnostic {
        match self {
            AgentError::Message(msg) => ErrorDiagnostic {
                // 这里用 runtime 作为 kind，便于与 provider/tool/skill 区分。
                kind: "runtime",
                message: msg.clone(),
                retriable: false,
                source: None,
            },
        }
    }
}

/// Memory 错误。
#[derive(thiserror::Error, Debug)]
pub enum MemoryError {
    /// 通用错误信息。
    #[error("memory error: {0}")]
    Message(String),
}

impl DiagnosticError for MemoryError {
    fn diagnostic(&self) -> ErrorDiagnostic {
        match self {
            MemoryError::Message(msg) => ErrorDiagnostic {
                kind: "memory",
                message: msg.clone(),
                retriable: false,
                source: None,
            },
        }
    }
}

/// Channel 错误。
#[derive(thiserror::Error, Debug)]
pub enum ChannelError {
    /// 通用错误信息。
    #[error("channel error: {0}")]
    Message(String),
}

impl DiagnosticError for ChannelError {
    fn diagnostic(&self) -> ErrorDiagnostic {
        match self {
            ChannelError::Message(msg) => ErrorDiagnostic {
                kind: "channel",
                message: msg.clone(),
                retriable: false,
                source: None,
            },
        }
    }
}
