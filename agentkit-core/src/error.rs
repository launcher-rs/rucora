//! agentkit-core 的统一错误类型定义（增强版）
//!
//! # 概述
//!
//! 本模块提供细粒度的错误分类，支持：
//! - 错误类型识别
//! - 可重试性判断
//! - 结构化诊断信息
//! - 错误来源追踪
//!
//! # 使用示例
//!
//! ```rust
//! use agentkit_core::error::{ProviderError, ErrorCategory};
//!
//! let error = ProviderError::RateLimit {
//!     retry_after: Some(std::time::Duration::from_secs(60)),
//!     message: "API 限流".to_string(),
//! };
//!
//! // 判断是否可重试
//! if error.is_retriable() {
//!     println!("可以重试");
//! }
//!
//! // 获取错误类别
//! match error.category() {
//!     ErrorCategory::RateLimit => println!("限流错误"),
//!     _ => println!("其他错误"),
//! }
//! ```

/// 错误类别枚举
///
/// 用于识别错误的根本原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// 网络错误（连接超时、DNS 解析失败等）
    Network,
    /// API 错误（HTTP 状态码错误）
    Api,
    /// 认证错误（API Key 无效、令牌过期等）
    Authentication,
    /// 授权错误（权限不足）
    Authorization,
    /// 限流错误（请求频率过高）
    RateLimit,
    /// 超时错误（请求超时）
    Timeout,
    /// 模型错误（模型不存在、模型过载等）
    Model,
    /// 工具错误（工具执行失败）
    Tool,
    /// 策略错误（违反安全策略）
    Policy,
    /// 配置错误（配置无效）
    Configuration,
    /// 其他错误
    Other,
}

impl ErrorCategory {
    /// 判断是否可重试
    pub fn is_retriable(self) -> bool {
        matches!(
            self,
            ErrorCategory::Network | ErrorCategory::Timeout | ErrorCategory::RateLimit
        )
    }

    /// 判断是否认证相关
    pub fn is_authentication_error(self) -> bool {
        matches!(
            self,
            ErrorCategory::Authentication | ErrorCategory::Authorization
        )
    }

    /// 判断是否客户端错误
    pub fn is_client_error(self) -> bool {
        matches!(
            self,
            ErrorCategory::Authentication
                | ErrorCategory::Authorization
                | ErrorCategory::Configuration
                | ErrorCategory::Policy
        )
    }
}

/// 统一的错误诊断信息
///
/// 设计目标：
/// - 不破坏现有错误枚举与调用点
/// - 让上层能拿到结构化的诊断字段
///
/// # 字段说明
///
/// - `kind`: 错误大类（provider/tool/runtime/skill/memory/channel）
/// - `message`: 人类可读信息
/// - `retriable`: 是否建议重试
/// - `source`: 可选的错误来源字符串
/// - `category`: 错误详细类别
/// - `status_code`: 可选的 HTTP 状态码
/// - `retry_after`: 可选的重试等待时间
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
    /// 错误详细类别
    pub category: ErrorCategory,
    /// HTTP 状态码（如果有）
    pub status_code: Option<u16>,
    /// 建议重试等待时间
    pub retry_after: Option<std::time::Duration>,
}

impl Default for ErrorDiagnostic {
    fn default() -> Self {
        Self {
            kind: "unknown",
            message: String::new(),
            retriable: false,
            source: None,
            category: ErrorCategory::Other,
            status_code: None,
            retry_after: None,
        }
    }
}

/// 为 core 层错误提供统一诊断能力
pub trait DiagnosticError {
    /// 获取错误的诊断信息
    fn diagnostic(&self) -> ErrorDiagnostic;

    /// 判断是否可重试
    fn is_retriable(&self) -> bool {
        self.diagnostic().retriable
    }

    /// 获取错误类别
    fn category(&self) -> ErrorCategory {
        self.diagnostic().category
    }
}

/// Provider 错误（增强版）
///
/// # 变体说明
///
/// - `Network`: 网络错误（可重试）
/// - `Api`: API 错误（根据状态码判断）
/// - `Authentication`: 认证错误（不可重试）
/// - `RateLimit`: 限流错误（可重试，带等待时间）
/// - `Timeout`: 超时错误（可重试）
/// - `Model`: 模型错误
/// - `Message`: 通用错误（向后兼容）
#[derive(thiserror::Error, Debug)]
pub enum ProviderError {
    /// 网络错误
    #[error("网络错误：{message}")]
    Network {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        retriable: bool,
    },

    /// API 错误
    #[error("API 错误 ({status}): {message}")]
    Api {
        status: u16,
        message: String,
        code: Option<String>,
    },

    /// 认证错误
    #[error("认证失败：{message}")]
    Authentication { message: String },

    /// 限流错误
    #[error("请求频率过高：{message}")]
    RateLimit {
        message: String,
        retry_after: Option<std::time::Duration>,
    },

    /// 超时错误
    #[error("请求超时：{message}")]
    Timeout {
        message: String,
        elapsed: std::time::Duration,
    },

    /// 模型错误
    #[error("模型错误：{message}")]
    Model { message: String },

    /// 通用错误（向后兼容）
    #[error("provider error: {0}")]
    Message(String),
}

impl ProviderError {
    /// 创建网络错误
    pub fn network(message: impl Into<String>) -> Self {
        Self::Network {
            message: message.into(),
            source: None,
            retriable: true,
        }
    }

    /// 创建认证错误
    pub fn authentication(message: impl Into<String>) -> Self {
        Self::Authentication {
            message: message.into(),
        }
    }

    /// 创建限流错误
    pub fn rate_limit(
        message: impl Into<String>,
        retry_after: Option<std::time::Duration>,
    ) -> Self {
        Self::RateLimit {
            message: message.into(),
            retry_after,
        }
    }

    /// 判断是否可重试
    pub fn is_retriable(&self) -> bool {
        match self {
            ProviderError::Network { retriable, .. } => *retriable,
            ProviderError::RateLimit { .. } => true,
            ProviderError::Timeout { .. } => true,
            ProviderError::Model { .. } => true,
            ProviderError::Api { status, .. } => {
                // 5xx 错误可重试
                *status >= 500 && *status < 600
            }
            _ => false,
        }
    }

    /// 获取错误类别
    pub fn category(&self) -> ErrorCategory {
        match self {
            ProviderError::Network { .. } => ErrorCategory::Network,
            ProviderError::Api { .. } => ErrorCategory::Api,
            ProviderError::Authentication { .. } => ErrorCategory::Authentication,
            ProviderError::RateLimit { .. } => ErrorCategory::RateLimit,
            ProviderError::Timeout { .. } => ErrorCategory::Timeout,
            ProviderError::Model { .. } => ErrorCategory::Model,
            ProviderError::Message(_) => ErrorCategory::Other,
        }
    }
}

impl DiagnosticError for ProviderError {
    fn diagnostic(&self) -> ErrorDiagnostic {
        match self {
            ProviderError::Network {
                message, retriable, ..
            } => ErrorDiagnostic {
                kind: "provider",
                message: message.clone(),
                retriable: *retriable,
                source: None,
                category: ErrorCategory::Network,
                status_code: None,
                retry_after: None,
            },
            ProviderError::Api {
                status,
                message,
                code,
            } => ErrorDiagnostic {
                kind: "provider",
                message: message.clone(),
                retriable: *status >= 500,
                source: code.clone(),
                category: ErrorCategory::Api,
                status_code: Some(*status),
                retry_after: None,
            },
            ProviderError::Authentication { message } => ErrorDiagnostic {
                kind: "provider",
                message: message.clone(),
                retriable: false,
                source: None,
                category: ErrorCategory::Authentication,
                status_code: None,
                retry_after: None,
            },
            ProviderError::RateLimit {
                message,
                retry_after,
            } => ErrorDiagnostic {
                kind: "provider",
                message: message.clone(),
                retriable: true,
                source: None,
                category: ErrorCategory::RateLimit,
                status_code: Some(429),
                retry_after: *retry_after,
            },
            ProviderError::Timeout {
                message,
                elapsed: _,
            } => ErrorDiagnostic {
                kind: "provider",
                message: message.clone(),
                retriable: true,
                source: None,
                category: ErrorCategory::Timeout,
                status_code: None,
                retry_after: None, // elapsed 是已消耗时间，不是建议等待时间
            },
            ProviderError::Model { message } => ErrorDiagnostic {
                kind: "provider",
                message: message.clone(),
                retriable: false, // 模型错误通常是永久性错误（如模型不存在）
                source: None,
                category: ErrorCategory::Model,
                status_code: None,
                retry_after: None,
            },
            ProviderError::Message(msg) => ErrorDiagnostic {
                kind: "provider",
                message: msg.clone(),
                retriable: true,
                source: None,
                category: ErrorCategory::Other,
                status_code: None,
                retry_after: None,
            },
        }
    }
}

/// Tool 错误（增强版）
#[derive(thiserror::Error, Debug)]
pub enum ToolError {
    /// 通用错误
    #[error("tool error: {0}")]
    Message(String),

    /// 策略拒绝
    #[error("tool policy denied (rule_id={rule_id}): {reason}")]
    PolicyDenied { rule_id: String, reason: String },

    /// 工具不存在
    #[error("工具不存在：{name}")]
    NotFound { name: String },

    /// 输入验证失败
    #[error("输入验证失败：{message}")]
    ValidationError { message: String },

    /// 执行超时
    #[error("工具执行超时：{message}")]
    Timeout { message: String },
}

impl DiagnosticError for ToolError {
    fn diagnostic(&self) -> ErrorDiagnostic {
        match self {
            ToolError::Message(msg) => ErrorDiagnostic {
                kind: "tool",
                message: msg.clone(),
                retriable: false,
                source: None,
                category: ErrorCategory::Tool,
                status_code: None,
                retry_after: None,
            },
            ToolError::PolicyDenied { rule_id, reason } => ErrorDiagnostic {
                kind: "tool",
                message: format!("policy denied (rule_id={}): {}", rule_id, reason),
                retriable: false,
                source: Some(rule_id.clone()),
                category: ErrorCategory::Policy,
                status_code: None,
                retry_after: None,
            },
            ToolError::NotFound { name } => ErrorDiagnostic {
                kind: "tool",
                message: format!("工具不存在：{}", name),
                retriable: false,
                source: None,
                category: ErrorCategory::Configuration,
                status_code: None,
                retry_after: None,
            },
            ToolError::ValidationError { message } => ErrorDiagnostic {
                kind: "tool",
                message: format!("输入验证失败：{}", message),
                retriable: false,
                source: None,
                category: ErrorCategory::Configuration,
                status_code: None,
                retry_after: None,
            },
            ToolError::Timeout { message } => ErrorDiagnostic {
                kind: "tool",
                message: format!("工具执行超时：{}", message),
                retriable: false, // 工具超时通常不应该重试
                source: None,
                category: ErrorCategory::Timeout,
                status_code: None,
                retry_after: None,
            },
        }
    }
}

/// Skill 错误
#[derive(thiserror::Error, Debug)]
pub enum SkillError {
    #[error("skill error: {0}")]
    Message(String),

    #[error("技能不存在：{name}")]
    NotFound { name: String },

    #[error("技能执行超时：{message}")]
    Timeout { message: String },
}

impl DiagnosticError for SkillError {
    fn diagnostic(&self) -> ErrorDiagnostic {
        match self {
            SkillError::Message(msg) => ErrorDiagnostic {
                kind: "skill",
                message: msg.clone(),
                retriable: false,
                source: None,
                category: ErrorCategory::Other,
                status_code: None,
                retry_after: None,
            },
            SkillError::NotFound { name } => ErrorDiagnostic {
                kind: "skill",
                message: format!("技能不存在：{}", name),
                retriable: false,
                source: None,
                category: ErrorCategory::Configuration,
                status_code: None,
                retry_after: None,
            },
            SkillError::Timeout { message } => ErrorDiagnostic {
                kind: "skill",
                message: format!("技能执行超时：{}", message),
                retriable: true,
                source: None,
                category: ErrorCategory::Timeout,
                status_code: None,
                retry_after: None,
            },
        }
    }
}

/// Agent 错误
#[derive(thiserror::Error, Debug)]
pub enum AgentError {
    /// 通用错误消息
    #[error("agent error: {0}")]
    Message(String),

    /// 超过最大步数限制
    #[error("超过最大步数限制：{max_steps}")]
    MaxStepsExceeded { max_steps: usize },

    /// Provider 错误
    #[error("Provider 错误：{source}")]
    ProviderError {
        #[source]
        source: ProviderError,
    },

    /// 需要 Runtime 支持（Agent 返回了需要 Runtime 执行的决策）
    #[error("此决策需要 Runtime 支持，请使用 Runtime 模式运行")]
    RequiresRuntime,
}

impl DiagnosticError for AgentError {
    fn diagnostic(&self) -> ErrorDiagnostic {
        match self {
            AgentError::Message(msg) => ErrorDiagnostic {
                kind: "runtime",
                message: msg.clone(),
                retriable: false,
                source: None,
                category: ErrorCategory::Other,
                status_code: None,
                retry_after: None,
            },
            AgentError::MaxStepsExceeded { max_steps } => ErrorDiagnostic {
                kind: "runtime",
                message: format!("超过最大步数限制：{}", max_steps),
                retriable: false,
                source: None,
                category: ErrorCategory::Configuration,
                status_code: None,
                retry_after: None,
            },
            AgentError::ProviderError { source } => {
                let mut diag = source.diagnostic();
                diag.kind = "runtime";
                diag
            }
            AgentError::RequiresRuntime => ErrorDiagnostic {
                kind: "runtime",
                message: "此决策需要 Runtime 支持，请使用 Runtime 模式运行".to_string(),
                retriable: false,
                source: None,
                category: ErrorCategory::Configuration,
                status_code: None,
                retry_after: None,
            },
        }
    }
}

/// Memory 错误
#[derive(thiserror::Error, Debug)]
pub enum MemoryError {
    #[error("memory error: {0}")]
    Message(String),

    #[error("记忆不存在：{id}")]
    NotFound { id: String },
}

impl DiagnosticError for MemoryError {
    fn diagnostic(&self) -> ErrorDiagnostic {
        match self {
            MemoryError::Message(msg) => ErrorDiagnostic {
                kind: "memory",
                message: msg.clone(),
                retriable: false,
                source: None,
                category: ErrorCategory::Other,
                status_code: None,
                retry_after: None,
            },
            MemoryError::NotFound { id } => ErrorDiagnostic {
                kind: "memory",
                message: format!("记忆不存在：{}", id),
                retriable: false,
                source: None,
                category: ErrorCategory::Configuration,
                status_code: None,
                retry_after: None,
            },
        }
    }
}

/// Channel 错误
#[derive(thiserror::Error, Debug)]
pub enum ChannelError {
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
                category: ErrorCategory::Other,
                status_code: None,
                retry_after: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_error_retriable() {
        let network = ProviderError::network("连接失败");
        assert!(network.is_retriable());
        assert_eq!(network.category(), ErrorCategory::Network);

        let auth = ProviderError::authentication("API Key 无效");
        assert!(!auth.is_retriable());
        assert_eq!(auth.category(), ErrorCategory::Authentication);

        let rate_limit =
            ProviderError::rate_limit("限流", Some(std::time::Duration::from_secs(60)));
        assert!(rate_limit.is_retriable());
        assert_eq!(rate_limit.category(), ErrorCategory::RateLimit);
    }

    #[test]
    fn test_error_category() {
        assert!(ErrorCategory::Network.is_retriable());
        assert!(ErrorCategory::Timeout.is_retriable());
        assert!(ErrorCategory::RateLimit.is_retriable());
        assert!(!ErrorCategory::Authentication.is_retriable());
        assert!(!ErrorCategory::Policy.is_retriable());
    }

    #[test]
    fn test_diagnostic() {
        let error = ProviderError::Api {
            status: 503,
            message: "服务不可用".to_string(),
            code: None,
        };

        let diag = error.diagnostic();
        assert_eq!(diag.kind, "provider");
        assert_eq!(diag.status_code, Some(503));
        assert!(diag.retriable);
        assert_eq!(diag.category, ErrorCategory::Api);
    }
}
