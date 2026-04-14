//! 结构化错误分类器
//!
//! 参考 Hermes Agent 的错误分类设计，将 API 错误精细分类为不同的失败原因，
//! 以便采取针对性的恢复策略。
//!
//! # 设计目标
//!
//! - **精细分类**: 区分认证失败、计费耗尽、速率限制、上下文溢出等不同情况
//! - **恢复指导**: 根据分类结果决定是重试、回退、压缩还是终止
//! - **优先级管线**: 按优先级从高到低依次匹配，确保分类准确性
//!
//! # 分类管线（优先级排序）
//!
//! 1. Provider 特定模式匹配（thinking 签名、层级门控等）
//! 2. HTTP 状态码 + 消息感知细化
//! 3. 错误码分类
//! 4. 消息模式匹配（billing vs rate_limit vs context vs auth）
//! 5. 传输错误启发式
//! 6. 兜底：`Unknown`（默认可重试）

use serde::{Deserialize, Serialize};

use crate::error::ProviderError;

/// 失败原因分类
///
/// 每种原因对应不同的恢复策略：
/// - `Auth`: 可能需要刷新或轮换凭证
/// - `Billing`: 立即切换到备用 Provider
/// - `RateLimit`: 退避后重试
/// - `ContextOverflow`: 需要压缩上下文，不应回退
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
        match self {
            Self::AuthPermanent | Self::Billing | Self::ModelNotFound | Self::FormatError => false,
            _ => true,
        }
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
            Self::RateLimit => Some(5000),  // 5 秒
            Self::Overloaded => Some(3000), // 3 秒
            Self::Timeout => Some(2000),    // 2 秒
            Self::ServerError => Some(1000), // 1 秒
            _ => None,
        }
    }
}

/// 分类后的错误信息
///
/// 包含原始错误、分类结果和建议的恢复策略。
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
            self.status_code.map(|s| s.to_string()).unwrap_or_else(|| "N/A".to_string()),
            action
        )
    }
}

/// 错误上下文信息
///
/// 用于辅助错误分类，包含请求的相关信息。
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

/// 错误分类器
///
/// 按照优先级排序的管线对 API 错误进行分类。
pub struct ErrorClassifier;

impl ErrorClassifier {
    /// 分类 API 错误
    ///
    /// # 参数
    ///
    /// - `error`: 原始 Provider 错误
    /// - `context`: 错误上下文信息
    ///
    /// # 返回
    ///
    /// 分类后的错误信息
    pub fn classify(error: &ProviderError, context: &ErrorContext) -> ClassifiedError {
        let reason = Self::classify_reason(error, context);

        ClassifiedError {
            reason,
            status_code: context.status_code,
            message: error.to_string(),
            retryable: reason.is_retryable(),
            should_compress: reason.should_compress(),
            should_fallback: reason.should_fallback(),
            should_rotate_credential: reason.should_rotate_credential(),
        }
    }

    /// 内部方法：分类失败原因
    ///
    /// 按照优先级从高到低依次匹配：
    /// 1. Provider 特定模式
    /// 2. HTTP 状态码
    /// 3. 错误码
    /// 4. 消息模式匹配
    /// 5. 传输错误启发式
    /// 6. 兜底：Unknown
    fn classify_reason(error: &ProviderError, context: &ErrorContext) -> FailoverReason {
        let error_message = error.to_string().to_lowercase();

        // ===== 优先级 1: Provider 特定模式匹配 =====

        // Anthropic thinking 签名无效
        if error_message.contains("thinking") && error_message.contains("signature") {
            return FailoverReason::ThinkingSignature;
        }

        // Anthropic 长上下文层级门控
        if error_message.contains("tier")
            || error_message.contains("context window")
                && error_message.contains("upgrade")
        {
            return FailoverReason::LongContextTier;
        }

        // ===== 优先级 2: HTTP 状态码匹配 =====

        if let Some(status) = context.status_code {
            match status {
                // 401 Unauthorized → 认证失败
                401 => {
                    if error_message.contains("invalid") || error_message.contains("expired") {
                        return FailoverReason::Auth;
                    }
                    return FailoverReason::AuthPermanent;
                }
                // 403 Forbidden → 认证永久失败或计费问题
                403 => {
                    if error_message.contains("billing")
                        || error_message.contains("quota")
                        || error_message.contains("payment")
                    {
                        return FailoverReason::Billing;
                    }
                    return FailoverReason::AuthPermanent;
                }
                // 404 Not Found → 模型不存在
                404 => {
                    if error_message.contains("model") || error_message.contains("not found") {
                        return FailoverReason::ModelNotFound;
                    }
                }
                // 413 Payload Too Large → 请求体过大
                413 => return FailoverReason::PayloadTooLarge,
                // 429 Too Many Requests → 速率限制
                429 => {
                    if error_message.contains("billing") || error_message.contains("quota") {
                        return FailoverReason::Billing;
                    }
                    return FailoverReason::RateLimit;
                }
                // 500-599 Server Error → 服务端问题
                500..=599 => {
                    if error_message.contains("overloaded") || error_message.contains("capacity") {
                        return FailoverReason::Overloaded;
                    }
                    return FailoverReason::ServerError;
                }
                _ => {}
            }
        }

        // ===== 优先级 3: 错误码分类 =====

        // 某些 Provider 返回结构化错误码
        if error_message.contains("insufficient_quota")
            || error_message.contains("billing_not_active")
        {
            return FailoverReason::Billing;
        }

        if error_message.contains("rate_limit") {
            return FailoverReason::RateLimit;
        }

        if error_message.contains("model_not_found") {
            return FailoverReason::ModelNotFound;
        }

        // ===== 优先级 4: 消息模式匹配 =====

        // 上下文溢出检测
        if Self::match_patterns(
            &error_message,
            &[
                "context length",
                "context window",
                "maximum context",
                "token limit",
                "prompt is too long",
                "input length",
            ],
        ) {
            return FailoverReason::ContextOverflow;
        }

        // 计费/配额检测
        if Self::match_patterns(
            &error_message,
            &[
                "billing",
                "quota",
                "payment",
                "credit",
                "subscription",
                "insufficient",
                "exceeded",
            ],
        ) {
            return FailoverReason::Billing;
        }

        // 速率限制检测
        if Self::match_patterns(
            &error_message,
            &[
                "rate limit",
                "too many requests",
                "throttl",
                "requests per minute",
                "requests per day",
            ],
        ) {
            return FailoverReason::RateLimit;
        }

        // 认证失败检测
        if Self::match_patterns(
            &error_message,
            &[
                "unauthorized",
                "authentication",
                "api key",
                "access token",
                "invalid credentials",
            ],
        ) {
            return FailoverReason::Auth;
        }

        // 模型不存在检测
        if Self::match_patterns(
            &error_message,
            &["model not found", "invalid model", "no model", "unknown model"],
        ) {
            return FailoverReason::ModelNotFound;
        }

        // ===== 优先级 5: 传输错误启发式 =====

        match error {
            ProviderError::Network { .. } => return FailoverReason::ServerError,
            ProviderError::Timeout { .. } => return FailoverReason::Timeout,
            _ => {}
        }

        // ===== 优先级 6: 兜底 =====

        // 检查是否接近上下文窗口限制
        if let (Some(ctx_tokens), Some(ctx_window)) =
            (context.context_tokens, context.context_window)
        {
            if ctx_tokens >= (ctx_window as f64 * 0.9) as usize {
                return FailoverReason::ContextOverflow;
            }
        }

        // 默认返回 Unknown（按可重试处理）
        FailoverReason::Unknown
    }

    /// 匹配多个模式（任一匹配即返回 true）
    fn match_patterns(message: &str, patterns: &[&str]) -> bool {
        patterns.iter().any(|p| message.contains(p))
    }

    /// 使用正则表达式匹配高级模式
    ///
    /// 用于检测 Prompt 注入等复杂模式。
    #[allow(dead_code)]
    fn match_regex_patterns(message: &str, pattern: &str) -> bool {
        regex::Regex::new(pattern)
            .map(|re: regex::Regex| re.is_match(message))
            .unwrap_or(false)
    }
}

/// 为 ProviderError 添加便捷方法
impl ProviderError {
    /// 分类当前错误
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let error = ProviderError::Message("rate limit exceeded".to_string());
    /// let context = ErrorContext {
    ///     status_code: Some(429),
    ///     ..Default::default()
    /// };
    /// let classified = error.classify(&context);
    /// assert_eq!(classified.reason, FailoverReason::RateLimit);
    /// ```
    pub fn classify(&self, context: &ErrorContext) -> ClassifiedError {
        ErrorClassifier::classify(self, context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_rate_limit() {
        let error = ProviderError::Message("rate limit exceeded".to_string());
        let context = ErrorContext {
            status_code: Some(429),
            ..Default::default()
        };
        let classified = ErrorClassifier::classify(&error, &context);
        assert_eq!(classified.reason, FailoverReason::RateLimit);
        assert!(classified.retryable);
        assert!(classified.should_fallback);
    }

    #[test]
    fn test_classify_billing() {
        let error = ProviderError::Message("insufficient quota".to_string());
        let context = ErrorContext {
            status_code: Some(429),
            ..Default::default()
        };
        let classified = ErrorClassifier::classify(&error, &context);
        assert_eq!(classified.reason, FailoverReason::Billing);
        assert!(!classified.retryable);
        assert!(classified.should_fallback);
    }

    #[test]
    fn test_classify_context_overflow() {
        let error = ProviderError::Message("context length exceeded".to_string());
        let context = ErrorContext {
            status_code: Some(400),
            ..Default::default()
        };
        let classified = ErrorClassifier::classify(&error, &context);
        assert_eq!(classified.reason, FailoverReason::ContextOverflow);
        assert!(classified.should_compress);
        assert!(!classified.should_fallback);
    }

    #[test]
    fn test_classify_auth() {
        let error = ProviderError::Message("invalid api key".to_string());
        let context = ErrorContext {
            status_code: Some(401),
            ..Default::default()
        };
        let classified = ErrorClassifier::classify(&error, &context);
        assert_eq!(classified.reason, FailoverReason::Auth);
        assert!(classified.retryable);
        assert!(classified.should_rotate_credential);
    }

    #[test]
    fn test_classify_model_not_found() {
        let error = ProviderError::Message("model not found".to_string());
        let context = ErrorContext {
            status_code: Some(404),
            ..Default::default()
        };
        let classified = ErrorClassifier::classify(&error, &context);
        assert_eq!(classified.reason, FailoverReason::ModelNotFound);
        assert!(!classified.retryable);
        assert!(classified.should_fallback);
    }

    #[test]
    fn test_failover_reason_backoff_ms() {
        assert_eq!(FailoverReason::RateLimit.recommended_backoff_ms(), Some(5000));
        assert_eq!(FailoverReason::Overloaded.recommended_backoff_ms(), Some(3000));
        assert_eq!(FailoverReason::Timeout.recommended_backoff_ms(), Some(2000));
        assert_eq!(FailoverReason::Billing.recommended_backoff_ms(), None);
    }

    #[test]
    fn test_classified_error_summary() {
        let classified = ClassifiedError {
            reason: FailoverReason::RateLimit,
            status_code: Some(429),
            message: "rate limit".to_string(),
            retryable: true,
            should_compress: false,
            should_fallback: true,
            should_rotate_credential: false,
        };
        let summary = classified.summary();
        assert!(summary.contains("RateLimit"));
        assert!(summary.contains("切换 Provider"));
    }
}
