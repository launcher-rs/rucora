//! agentkit-core 的统一错误类型定义
//!
//! # 概述
//!
//! 本模块定义了 Agentkit 框架中所有组件的统一错误类型。
//! 设计目标是提供结构化的错误信息，便于上层进行错误处理、日志记录和用户提示。
//!
//! # 错误类型
//!
//! ## ProviderError
//!
//! LLM Provider 相关错误，通常来自网络请求、API 调用等。
//!
//! ```rust
//! use agentkit_core::error::ProviderError;
//!
//! let err = ProviderError::Message("API 调用失败".to_string());
//! ```
//!
//! ## ToolError
//!
//! 工具执行相关错误，包括策略拒绝等。
//!
//! ```rust
//! use agentkit_core::error::ToolError;
//!
//! // 通用错误
//! let err1 = ToolError::Message("工具执行失败".to_string());
//!
//! // 策略拒绝
//! let err2 = ToolError::PolicyDenied {
//!     rule_id: "dangerous_command".to_string(),
//!     reason: "命令 'rm -rf /' 被禁止".to_string(),
//! };
//! ```
//!
//! ## SkillError
//!
//! 技能执行相关错误。
//!
//! ```rust
//! use agentkit_core::error::SkillError;
//!
//! let err = SkillError::Message("技能执行失败".to_string());
//! ```
//!
//! ## AgentError
//!
//! Agent/运行时相关错误。
//!
//! ```rust
//! use agentkit_core::error::AgentError;
//!
//! let err = AgentError::Message("运行时错误".to_string());
//! ```
//!
//! ## MemoryError
//!
//! 记忆系统相关错误。
//!
//! ```rust
//! use agentkit_core::error::MemoryError;
//!
//! let err = MemoryError::Message("记忆存储失败".to_string());
//! ```
//!
//! ## ChannelError
//!
//! 通信渠道相关错误。
//!
//! ```rust
//! use agentkit_core::error::ChannelError;
//!
//! let err = ChannelError::Message("事件发送失败".to_string());
//! ```
//!
//! # 错误诊断
//!
//! 所有错误类型都实现了 [`DiagnosticError`] trait，提供结构化的诊断信息：
//!
//! ```rust
//! use agentkit_core::error::{DiagnosticError, ToolError};
//!
//! let err = ToolError::Message("工具执行失败".to_string());
//! let diag = err.diagnostic();
//!
//! println!("错误类型：{}", diag.kind);        // "tool"
//! println!("错误信息：{}", diag.message);      // "工具执行失败"
//! println!("是否可重试：{}", diag.retriable);  // false
//! ```
//!
//! ## ErrorDiagnostic 字段说明
//!
//! - `kind`: 错误大类（provider/tool/runtime/skill/memory/channel）
//! - `message`: 人类可读的错误信息
//! - `retriable`: 是否建议重试（网络错误通常可重试，策略拒绝通常不可重试）
//! - `source`: 可选的错误来源（如底层库名、provider 名称等）

/// 统一的错误诊断信息
///
/// 设计目标：
/// - 不破坏现有错误枚举（`Message(String)` 等）与调用点
/// - 让上层（runtime / UI / trace / policy）能拿到结构化的诊断字段
///
/// # 字段说明
///
/// - `kind`: 错误大类（provider/tool/runtime/skill/memory/channel/...）
/// - `message`: 人类可读信息（可直接打印）
/// - `retriable`: 是否建议重试（最佳努力）
/// - `source`: 可选的错误来源字符串（例如底层库名、provider 名称等）
///
/// # 示例
///
/// ```rust
/// use agentkit_core::error::ErrorDiagnostic;
///
/// let diag = ErrorDiagnostic {
///     kind: "tool",
///     message: "工具执行失败".to_string(),
///     retriable: false,
///     source: Some("shell_tool".to_string()),
/// };
///
/// assert_eq!(diag.kind, "tool");
/// assert_eq!(diag.message, "工具执行失败");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorDiagnostic {
    /// 错误类型
    pub kind: &'static str,
    /// 错误消息
    pub message: String,
    /// 是否可重试
    pub retriable: bool,
    /// 错误来源
    pub source: Option<String>,
}

/// 为 core 层错误提供统一诊断能力
///
/// 实现此 trait 的错误类型可以提供结构化的诊断信息。
///
/// # 示例
///
/// ```rust
/// use agentkit_core::error::{DiagnosticError, ToolError};
///
/// let err = ToolError::Message("工具执行失败".to_string());
/// let diag = err.diagnostic();
///
/// assert_eq!(diag.kind, "tool");
/// assert_eq!(diag.message, "工具执行失败");
/// assert!(!diag.retriable);
/// ```
pub trait DiagnosticError {
    /// 获取错误的诊断信息
    fn diagnostic(&self) -> ErrorDiagnostic;
}

/// Provider 错误
///
/// LLM Provider 相关错误，通常来自网络请求、API 调用等。
///
/// # 变体
///
/// - `Message(String)`: 通用错误信息
///
/// # 示例
///
/// ```rust
/// use agentkit_core::error::ProviderError;
///
/// let err = ProviderError::Message("API 调用失败：连接超时".to_string());
/// println!("错误：{}", err);
/// ```
#[derive(thiserror::Error, Debug)]
pub enum ProviderError {
    /// 通用错误信息
    #[error("provider error: {0}")]
    Message(String),
}

impl DiagnosticError for ProviderError {
    fn diagnostic(&self) -> ErrorDiagnostic {
        match self {
            ProviderError::Message(msg) => ErrorDiagnostic {
                kind: "provider",
                message: msg.clone(),
                // provider 错误在实践中经常可重试（网络抖动/限流/暂时不可用）
                retriable: true,
                source: None,
            },
        }
    }
}

/// Tool 错误
///
/// 工具执行相关错误。
///
/// # 变体
///
/// - `Message(String)`: 通用错误信息
/// - `PolicyDenied`: 策略拒绝（如危险命令被拦截）
///
/// # 示例
///
/// ```rust
/// use agentkit_core::error::ToolError;
///
/// // 通用错误
/// let err1 = ToolError::Message("工具执行失败".to_string());
///
/// // 策略拒绝
/// let err2 = ToolError::PolicyDenied {
///     rule_id: "dangerous_command".to_string(),
///     reason: "命令 'rm -rf /' 被禁止".to_string(),
/// };
///
/// println!("错误 1: {}", err1);
/// println!("错误 2: {}", err2);
/// ```
#[derive(thiserror::Error, Debug)]
pub enum ToolError {
    /// 通用错误信息
    #[error("tool error: {0}")]
    Message(String),

    /// 策略拒绝
    #[error("tool policy denied (rule_id={rule_id}): {reason}")]
    PolicyDenied {
        /// 规则 ID
        rule_id: String,
        /// 拒绝原因
        reason: String,
    },
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
                // 策略拒绝通常不可重试（除非用户改变输入/策略）
                retriable: false,
                source: Some(rule_id.clone()),
            },
        }
    }
}

/// Skill 错误
///
/// 技能执行相关错误。
///
/// # 变体
///
/// - `Message(String)`: 通用错误信息
///
/// # 示例
///
/// ```rust
/// use agentkit_core::error::SkillError;
///
/// let err = SkillError::Message("技能执行失败：缺少依赖".to_string());
/// println!("错误：{}", err);
/// ```
#[derive(thiserror::Error, Debug)]
pub enum SkillError {
    /// 通用错误信息
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

/// Agent 错误
///
/// Agent/运行时相关错误。
///
/// # 变体
///
/// - `Message(String)`: 通用错误信息
///
/// # 示例
///
/// ```rust
/// use agentkit_core::error::AgentError;
///
/// let err = AgentError::Message("超过最大步数限制".to_string());
/// println!("错误：{}", err);
/// ```
#[derive(thiserror::Error, Debug)]
pub enum AgentError {
    /// 通用错误信息
    #[error("agent error: {0}")]
    Message(String),
}

impl DiagnosticError for AgentError {
    fn diagnostic(&self) -> ErrorDiagnostic {
        match self {
            AgentError::Message(msg) => ErrorDiagnostic {
                // 这里用 runtime 作为 kind，便于与 provider/tool/skill 区分
                kind: "runtime",
                message: msg.clone(),
                retriable: false,
                source: None,
            },
        }
    }
}

/// Memory 错误
///
/// 记忆系统相关错误。
///
/// # 变体
///
/// - `Message(String)`: 通用错误信息
///
/// # 示例
///
/// ```rust
/// use agentkit_core::error::MemoryError;
///
/// let err = MemoryError::Message("记忆存储失败".to_string());
/// println!("错误：{}", err);
/// ```
#[derive(thiserror::Error, Debug)]
pub enum MemoryError {
    /// 通用错误信息
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

/// Channel 错误
///
/// 通信渠道相关错误。
///
/// # 变体
///
/// - `Message(String)`: 通用错误信息
///
/// # 示例
///
/// ```rust
/// use agentkit_core::error::ChannelError;
///
/// let err = ChannelError::Message("事件发送失败".to_string());
/// println!("错误：{}", err);
/// ```
#[derive(thiserror::Error, Debug)]
pub enum ChannelError {
    /// 通用错误信息
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
