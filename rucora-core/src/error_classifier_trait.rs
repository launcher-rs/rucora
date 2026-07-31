//! 结构化错误分类器 trait（纯接口层）
//!
//! 本模块只定义错误分类的接口和类型，不包含具体实现。
//! 实现位于 rucora crate 的 error_classifier_impl 模块。

use serde::{Deserialize, Serialize};

use crate::error::{DiagnosticError, ProviderError};

/// 失败原因分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailoverReason {
    /// 认证失败（可能可恢复，如 Token 过期）
    Auth,
    /// 认证永久失败（凭证无效，无法恢复）
    AuthPermanent,
    /// 计费耗尽（配额用完，需要切换 Provider）
    Billing,
    /// 速率限制（请求过于频繁，需要退避）
    RateLimit,
    /// 服务端过载（临时性问题，可重试）
    Overloaded,
    /// 服务端内部错误（可能需要重试）
    ServerError,
    /// 请求超时（网络问题，可重试）
    Timeout,
    /// 上下文溢出（需要压缩，不应回退）
    ContextOverflow,
    /// 请求体过大（需要压缩或截断）
    PayloadTooLarge,
    /// 模型不存在（可能需要切换模型）
    ModelNotFound,
    /// 请求格式错误（可能需要修复代码）
    FormatError,
    /// Anthropic thinking 签名无效
    ThinkingSignature,
    /// Anthropic 长上下文层级门控
    LongContextTier,
    /// 未知原因（默认按可重试处理）
    Unknown,
}

impl FailoverReason {
    /// 判断是否可重试
    pub fn is_retryable(&self) -> bool {
        !matches!(
            self,
            Self::AuthPermanent | Self::Billing | Self::ModelNotFound | Self::FormatError
        )
    }

    /// 是否应该触发上下文压缩
    pub fn should_compress(&self) -> bool {
        matches!(
            self,
            Self::ContextOverflow | Self::PayloadTooLarge | Self::LongContextTier
        )
    }

    /// 是否应该切换到备用 Provider
    pub fn should_fallback(&self) -> bool {
        matches!(
            self,
            Self::Billing
                | Self::RateLimit
                | Self::Overloaded
                | Self::ModelNotFound
                | Self::AuthPermanent
        )
    }

    /// 是否需要轮换凭证
    pub fn should_rotate_credential(&self) -> bool {
        matches!(self, Self::Auth)
    }

    /// 获取推荐的退避时间（毫秒）
    pub fn recommended_backoff_ms(&self) -> Option<u64> {
        match self {
            Self::RateLimit => Some(5000),   // 5 秒
            Self::Overloaded => Some(3000),  // 3 秒
            Self::Timeout => Some(2000),     // 2 秒
            Self::ServerError => Some(1000), // 1 秒
            _ => None,
        }
    }
}

/// 分类后的错误信息
#[derive(Debug, Clone)]
pub struct ClassifiedError {
    /// 失败原因分类
    pub reason: FailoverReason,
    /// HTTP 状态码（如果有）
    pub status_code: Option<u16>,
    /// 原始错误消息
    pub message: String,
    /// 是否可重试
    pub retryable: bool,
    /// 是否应该触发上下文压缩
    pub should_compress: bool,
    /// 是否应该切换到备用 Provider
    pub should_fallback: bool,
    /// 是否需要轮换凭证
    pub should_rotate_credential: bool,
}

impl ClassifiedError {
    /// 生成人类可读的错误摘要
    pub fn summary(&self) -> String {
        let action = if self.should_compress {
            "需要压缩上下文"
        } else if self.should_fallback {
            "建议切换 Provider"
        } else if self.should_rotate_credential {
            "需要轮换凭证"
        } else if self.retryable {
            "可重试"
        } else {
            "不可恢复"
        };

        format!(
            "错误原因：{:?} | 状态码：{} | 策略：{}",
            self.reason,
            self.status_code
                .map_or_else(|| "N/A".to_string(), |s| s.to_string()),
            action
        )
    }
}

/// 错误上下文信息
#[derive(Debug, Clone, Default)]
pub struct ErrorContext {
    /// HTTP 状态码
    pub status_code: Option<u16>,
    /// Provider 名称
    pub provider: Option<String>,
    /// 模型名称
    pub model: Option<String>,
    /// 请求体大小（字节）
    pub request_size: Option<usize>,
    /// 响应体大小（字节）
    pub response_size: Option<usize>,
    /// 当前上下文 Token 数
    pub context_tokens: Option<usize>,
    /// 模型上下文窗口大小
    pub context_window: Option<usize>,
}

/// 错误分类器 trait（纯接口）
///
/// 具体实现位于 rucora crate。
pub trait ErrorClassifier: Send + Sync {
    /// 分类 API 错误
    fn classify(&self, error: &ProviderError, context: &ErrorContext) -> ClassifiedError {
        default_classify(error, context)
    }
}

/// 基于 [`ProviderError`] 内建分类的默认分类器。
///
/// 通过 [`ErrorCategory`] 与 HTTP 状态码推断失败原因，
/// 无需实现者编写分类逻辑即可获得基础分类能力。
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultErrorClassifier;

impl ErrorClassifier for DefaultErrorClassifier {
    fn classify(&self, error: &ProviderError, context: &ErrorContext) -> ClassifiedError {
        default_classify(error, context)
    }
}

/// 默认分类逻辑：根据错误类别与状态码映射到 [`FailoverReason`]。
fn default_classify(error: &ProviderError, context: &ErrorContext) -> ClassifiedError {
    let diag = error.diagnostic();
    let reason = classify_by_category(diag.category, diag.status_code);
    let status_code = context.status_code.or(diag.status_code);
    let message = diag.message;

    ClassifiedError {
        reason,
        status_code,
        message,
        retryable: reason.is_retryable(),
        should_compress: reason.should_compress(),
        should_fallback: reason.should_fallback(),
        should_rotate_credential: reason.should_rotate_credential(),
    }
}

/// 将错误类别映射到失败原因。
fn classify_by_category(category: crate::error::ErrorCategory, status_code: Option<u16>) -> FailoverReason {
    use crate::error::ErrorCategory;

    match category {
        ErrorCategory::Network => FailoverReason::Timeout,
        ErrorCategory::Api => match status_code {
            Some(401 | 403) => FailoverReason::AuthPermanent,
            Some(402) => FailoverReason::Billing,
            Some(413) => FailoverReason::PayloadTooLarge,
            Some(404) => FailoverReason::ModelNotFound,
            Some(429) => FailoverReason::RateLimit,
            Some(5..=599) => FailoverReason::ServerError,
            _ => FailoverReason::FormatError,
        },
        ErrorCategory::Authentication => match status_code {
            Some(401 | 403) => FailoverReason::AuthPermanent,
            _ => FailoverReason::Auth,
        },
        ErrorCategory::Authorization => FailoverReason::AuthPermanent,
        ErrorCategory::RateLimit => FailoverReason::RateLimit,
        ErrorCategory::Timeout => FailoverReason::Timeout,
        ErrorCategory::Model => FailoverReason::ModelNotFound,
        ErrorCategory::Policy => FailoverReason::FormatError,
        ErrorCategory::Configuration => FailoverReason::FormatError,
        ErrorCategory::Tool => FailoverReason::Unknown,
        ErrorCategory::Other => FailoverReason::Unknown,
    }
}

/// 为 ProviderError 添加分类方法的 trait
///
/// 注意：这是一个扩展 trait，需要在 rucora crate 中实现
pub trait ProviderErrorExt {
    /// 使用默认分类器分类错误
    fn classify(&self, context: &ErrorContext) -> ClassifiedError;
}

impl ProviderErrorExt for ProviderError {
    fn classify(&self, context: &ErrorContext) -> ClassifiedError {
        default_classify(self, context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failover_reason_is_retryable() {
        assert!(FailoverReason::RateLimit.is_retryable());
        assert!(FailoverReason::Timeout.is_retryable());
        assert!(!FailoverReason::Billing.is_retryable());
        assert!(!FailoverReason::AuthPermanent.is_retryable());
    }

    #[test]
    fn test_failover_reason_should_compress() {
        assert!(FailoverReason::ContextOverflow.should_compress());
        assert!(FailoverReason::PayloadTooLarge.should_compress());
        assert!(!FailoverReason::RateLimit.should_compress());
    }

    #[test]
    fn test_failover_reason_should_fallback() {
        assert!(FailoverReason::Billing.should_fallback());
        assert!(FailoverReason::RateLimit.should_fallback());
        assert!(!FailoverReason::ContextOverflow.should_fallback());
    }

    #[test]
    fn test_failover_reason_backoff_ms() {
        assert_eq!(
            FailoverReason::RateLimit.recommended_backoff_ms(),
            Some(5000)
        );
        assert_eq!(FailoverReason::Timeout.recommended_backoff_ms(), Some(2000));
        assert_eq!(FailoverReason::Billing.recommended_backoff_ms(), None);
    }

    #[test]
    fn test_default_classifier_rate_limit() {
        let classifier = DefaultErrorClassifier;
        let error = crate::error::ProviderError::rate_limit("限流", None);
        let classified = classifier.classify(&error, &ErrorContext::default());

        assert_eq!(classified.reason, FailoverReason::RateLimit);
        assert!(classified.retryable);
        assert!(classified.should_fallback);
        assert_eq!(classified.status_code, Some(429));
    }

    #[test]
    fn test_default_classifier_auth() {
        let classifier = DefaultErrorClassifier;
        let error = crate::error::ProviderError::authentication("无效 API Key");
        let classified = classifier.classify(&error, &ErrorContext::default());

        // Auth（如 Token 过期）可重试；401/403 会映射为 AuthPermanent
        assert_eq!(classified.reason, FailoverReason::Auth);
        assert!(classified.retryable);
        assert!(classified.should_rotate_credential);
    }

    #[test]
    fn test_default_classifier_timeout() {
        let classifier = DefaultErrorClassifier;
        let error = crate::error::ProviderError::network("连接超时");
        let classified = classifier.classify(&error, &ErrorContext::default());

        assert_eq!(classified.reason, FailoverReason::Timeout);
        assert!(classified.retryable);
    }

    #[test]
    fn test_provider_error_ext_classify() {
        let error = crate::error::ProviderError::rate_limit("限流", None);
        let classified = error.classify(&ErrorContext::default());
        assert_eq!(classified.reason, FailoverReason::RateLimit);
    }
}
