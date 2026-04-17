//! AgentKit 结构化错误分类器示例
//!
//! 展示如何使用错误分类器对 Provider 错误进行精细分类，
//! 并根据分类结果采取针对性的恢复策略。
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example 22_error_classification
//! ```
//!
//! ## 功能演示
//!
//! 1. **错误分类** - 将 API 错误分类为 14 种原因
//! 2. **恢复策略** - 根据分类决定重试/回退/压缩
//! 3. **实际应用** - 与 Provider 集成使用

use agentkit::provider::OpenAiProvider;
use agentkit_core::error::ProviderError;
// 允许使用已弃用的旧实现，直到新实现完成
#[allow(deprecated)]
use agentkit_core::error_classifier::{ErrorClassifier, ErrorContext, FailoverReason};
use agentkit_core::provider::LlmProvider;
use agentkit_core::provider::types::ChatRequest;
use std::time::Duration;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔════════════════════════════════════════╗");
    info!("║   AgentKit 结构化错误分类器示例       ║");
    info!("╚════════════════════════════════════════╝\n");

    // ═══════════════════════════════════════════════════════════
    // 示例 1: 错误分类器基础使用
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 1: 错误分类器基础使用");
    info!("═══════════════════════════════════════\n");

    info!("错误分类器可以识别以下错误类型:");
    info!("  - Auth: 认证失败");
    info!("  - AuthPermanent: 认证永久失败");
    info!("  - Billing: 计费耗尽");
    info!("  - RateLimit: 速率限制");
    info!("  - Overloaded: 服务端过载");
    info!("  - ServerError: 服务端内部错误");
    info!("  - Timeout: 请求超时");
    info!("  - ContextOverflow: 上下文溢出");
    info!("  - PayloadTooLarge: 请求体过大");
    info!("  - ModelNotFound: 模型不存在");
    info!("  - FormatError: 请求格式错误");
    info!("  - ThinkingSignature: Anthropic thinking 签名无效");
    info!("  - LongContextTier: Anthropic 长上下文层级门控");
    info!("  - Unknown: 未知原因\n");

    // 测试不同的错误场景
    let test_cases = vec![
        (
            ProviderError::Message("rate limit exceeded".to_string()),
            ErrorContext {
                status_code: Some(429),
                provider: Some("openai".to_string()),
                ..Default::default()
            },
            "速率限制错误",
        ),
        (
            ProviderError::Message("context length exceeded".to_string()),
            ErrorContext {
                status_code: Some(400),
                provider: Some("openai".to_string()),
                ..Default::default()
            },
            "上下文溢出",
        ),
        (
            ProviderError::Message("invalid api key".to_string()),
            ErrorContext {
                status_code: Some(401),
                provider: Some("openai".to_string()),
                ..Default::default()
            },
            "认证失败",
        ),
        (
            ProviderError::Message("model not found".to_string()),
            ErrorContext {
                status_code: Some(404),
                provider: Some("openai".to_string()),
                ..Default::default()
            },
            "模型不存在",
        ),
        (
            ProviderError::Message("insufficient quota".to_string()),
            ErrorContext {
                status_code: Some(429),
                provider: Some("openai".to_string()),
                ..Default::default()
            },
            "计费耗尽",
        ),
        (
            ProviderError::Timeout {
                message: "request timeout".to_string(),
                elapsed: Duration::from_secs(30),
            },
            ErrorContext {
                status_code: None,
                provider: Some("openai".to_string()),
                ..Default::default()
            },
            "请求超时",
        ),
    ];

    for (error, context, description) in &test_cases {
        #[allow(deprecated)]
        let classified = ErrorClassifier::classify(error, context);
        info!("场景: {}", description);
        info!("  错误: {}", error);
        info!("  分类: {:?}", classified.reason);
        info!("  可重试: {}", classified.retryable);
        info!("  应压缩: {}", classified.should_compress);
        info!("  应回退: {}", classified.should_fallback);
        info!("  轮换凭证: {}", classified.should_rotate_credential);
        info!("  推荐退避: {:?}ms", classified.reason.recommended_backoff_ms());
        info!("");
    }

    // ═══════════════════════════════════════════════════════════
    // 示例 2: 便捷方法使用
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 2: 便捷方法使用");
    info!("═══════════════════════════════════════\n");

    let error = ProviderError::Message("rate limit exceeded".to_string());
    let context = ErrorContext {
        status_code: Some(429),
        ..Default::default()
    };

    // ProviderError 的 classify 便捷方法（使用旧的实现）
    #[allow(deprecated)]
    let classified = error.classify(&context);
    info!("使用便捷方法 classify():");
    info!("  错误: {}", classified.message);
    info!("  分类: {:?}", classified.reason);
    info!("  摘要: {}", classified.summary());
    info!("");

    // FailoverReason 的判断方法
    info!("FailoverReason 判断方法:");
    let reasons: Vec<FailoverReason> = vec![
        FailoverReason::RateLimit,
        FailoverReason::Billing,
        FailoverReason::ContextOverflow,
        FailoverReason::Auth,
        FailoverReason::ModelNotFound,
    ];

    for reason in &reasons {
        info!(
            "  {:?}: 可重试={}, 应压缩={}, 应回退={}, 轮换凭证={}",
            reason,
            reason.is_retryable(),
            reason.should_compress(),
            reason.should_fallback(),
            reason.should_rotate_credential()
        );
    }
    info!("");

    // ═══════════════════════════════════════════════════════════
    // 示例 3: 实际 Provider 调用中的错误分类
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 3: 实际 Provider 调用中的错误分类");
    info!("═══════════════════════════════════════\n");

    if std::env::var("OPENAI_API_KEY").is_ok() {
        info!("3.1 使用 OpenAI Provider...");
        let provider = OpenAiProvider::from_env()?;
        info!("✓ Provider 创建成功\n");

        // 正常请求
        info!("3.2 发送正常请求...");
        let request = ChatRequest::from_user_text("你好");
        match provider.chat(request).await {
            Ok(response) => {
                info!("✓ 请求成功");
                info!(
                    "  响应: {}",
                    &response.message.content.chars().take(50).collect::<String>()
                );
                if let Some(usage) = response.usage {
                    info!(
                        "  Token 使用: 输入={}, 输出={}, 总计={}",
                        usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
                    );
                }
            }
            Err(e) => {
                info!("✗ 请求失败");
                // 分类错误
                let context = ErrorContext {
                    provider: Some("openai".to_string()),
                    ..Default::default()
                };
                #[allow(deprecated)]
                let classified = e.classify(&context);
                info!("  错误分类: {:?}", classified.reason);
                info!("  恢复策略: {}", classified.summary());

                // 根据分类采取恢复策略
                if classified.should_compress {
                    info!("  → 建议: 压缩上下文");
                }
                if classified.should_fallback {
                    info!("  → 建议: 切换到备用 Provider");
                }
                if classified.should_rotate_credential {
                    info!("  → 建议: 轮换 API Key");
                }
                if classified.retryable {
                    info!("  → 建议: 稍后重试");
                }
            }
        }
    } else {
        info!("⚠ 跳过实际 API 测试（未设置 API Key）\n");
    }

    // ═══════════════════════════════════════════════════════════
    // 示例 4: 恢复策略决策树
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 4: 恢复策略决策树");
    info!("═══════════════════════════════════════\n");

    info!("根据错误分类结果，推荐以下恢复策略:\n");

    info!("1. Auth (认证失败):");
    info!("   ├─ 可重试: 是 (可能 Token 过期)");
    info!("   ├─ 策略: 刷新/轮换凭证");
    info!("   └─ 退避: 无\n");

    info!("2. Billing (计费耗尽):");
    info!("   ├─ 可重试: 否");
    info!("   ├─ 策略: 立即切换 Provider");
    info!("   └─ 退避: 无\n");

    info!("3. RateLimit (速率限制):");
    info!("   ├─ 可重试: 是");
    info!("   ├─ 策略: 退避后重试 + 切换 Provider");
    info!("   └─ 退避: 5000ms\n");

    info!("4. ContextOverflow (上下文溢出):");
    info!("   ├─ 可重试: 是");
    info!("   ├─ 策略: 压缩上下文 (不应回退)");
    info!("   └─ 退避: 无\n");

    info!("5. ModelNotFound (模型不存在):");
    info!("   ├─ 可重试: 否");
    info!("   ├─ 策略: 切换模型或 Provider");
    info!("   └─ 退避: 无\n");

    info!("6. Timeout (超时):");
    info!("   ├─ 可重试: 是");
    info!("   ├─ 策略: 重建客户端 + 重试");
    info!("   └─ 退避: 2000ms\n");

    // ═══════════════════════════════════════════════════════════
    // 总结
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例完成！");
    info!("═══════════════════════════════════════\n");

    info!("📝 错误分类器总结:\n");

    info!("1. 优势:");
    info!("   - 精细分类 14 种错误原因");
    info!("   - 优先级排序的分类管线");
    info!("   - 针对性的恢复策略建议");
    info!("   - 自动判断是否可重试\n");

    info!("2. 使用场景:");
    info!("   - Provider 调用失败时的错误处理");
    info!("   - 自动重试决策");
    info!("   - 多 Provider 失败链切换");
    info!("   - 上下文压缩触发\n");

    info!("3. 最佳实践:");
    info!("   - 捕获所有 Provider 错误并分类");
    info!("   - 根据分类结果采取恢复策略");
    info!("   - 记录分类结果用于监控告警");
    info!("   - 结合退避算法使用\n");

    info!("4. 与 ResilientProvider 的区别:");
    info!("   - ResilientProvider: 自动重试，适用于简单场景");
    info!("   - 错误分类器: 手动决策，适用于复杂恢复策略\n");

    Ok(())
}
