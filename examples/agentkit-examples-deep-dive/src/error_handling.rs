//! 错误处理示例
//!
//! 本示例展示 AgentKit 中的高级错误处理技巧

use agentkit_core::error::{
    AgentError, DiagnosticError, MemoryError, ProviderError, SkillError, ToolError,
};
use tracing::info;

/// 运行示例
pub async fn run() -> anyhow::Result<()> {
    info!("=== 错误处理示例 ===");

    // 1. 错误诊断
    demo_error_diagnostic().await?;

    // 2. 错误转换
    demo_error_conversion().await?;

    // 3. 错误恢复
    demo_error_recovery().await?;

    // 4. 自定义错误
    demo_custom_error().await?;

    Ok(())
}

/// 错误诊断示例
async fn demo_error_diagnostic() -> anyhow::Result<()> {
    info!("\n--- 错误诊断 ---");

    // Provider 错误
    let provider_error = ProviderError::Message("API 调用失败".to_string());
    let diag = provider_error.diagnostic();
    info!("Provider 错误:");
    info!("  - 类型：{}", diag.kind);
    info!("  - 消息：{}", diag.message);
    info!("  - 可重试：{}", diag.retriable);

    // Tool 错误
    let tool_error = ToolError::Message("工具执行失败".to_string());
    let diag = tool_error.diagnostic();
    info!("\nTool 错误:");
    info!("  - 类型：{}", diag.kind);
    info!("  - 消息：{}", diag.message);
    info!("  - 可重试：{}", diag.retriable);

    // 策略拒绝错误
    let policy_error = ToolError::PolicyDenied {
        rule_id: "dangerous_command".to_string(),
        reason: "命令 'rm -rf /' 被禁止".to_string(),
    };
    let diag = policy_error.diagnostic();
    info!("\n策略拒绝错误:");
    info!("  - 类型：{}", diag.kind);
    info!("  - 消息：{}", diag.message);
    info!("  - 规则 ID: {:?}", diag.source);

    // Agent 错误
    let agent_error = AgentError::Message("运行时错误".to_string());
    let diag = agent_error.diagnostic();
    info!("\nAgent 错误:");
    info!("  - 类型：{}", diag.kind);
    info!("  - 消息：{}", diag.message);
    info!("  - 可重试：{}", diag.retriable);

    Ok(())
}

/// 错误转换示例
async fn demo_error_conversion() -> anyhow::Result<()> {
    info!("\n--- 错误转换 ---");

    // 将 ProviderError 转换为 anyhow::Error
    let provider_error = ProviderError::Message("API 错误".to_string());
    let anyhow_error: anyhow::Error = provider_error.into();
    info!("✓ ProviderError -> anyhow::Error: {}", anyhow_error);

    // 将 ToolError 转换为 String
    let tool_error = ToolError::Message("工具错误".to_string());
    let error_string = tool_error.to_string();
    info!("✓ ToolError -> String: {}", error_string);

    // 使用 ? 操作符自动转换
    let result: Result<(), anyhow::Error> = (|| {
        let _ = some_function_that_returns_provider_error()?;
        Ok(())
    })();

    info!("✓ 自动错误转换：{:?}", result.is_err());

    Ok(())
}

/// 错误恢复示例
async fn demo_error_recovery() -> anyhow::Result<()> {
    info!("\n--- 错误恢复 ---");

    // 重试逻辑
    let mut attempts = 0;
    let max_attempts = 3;

    while attempts < max_attempts {
        attempts += 1;
        info!("尝试 {} / {}", attempts, max_attempts);

        match flaky_operation(attempts).await {
            Ok(result) => {
                info!("✓ 操作成功：{}", result);
                break;
            }
            Err(e) => {
                info!("⚠ 操作失败：{}", e);
                if attempts < max_attempts {
                    let delay = 100 * (attempts as u64);
                    info!("  等待 {}ms 后重试", delay);
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                }
            }
        }
    }

    // 降级处理
    info!("\n--- 降级处理 ---");
    match primary_operation().await {
        Ok(result) => {
            info!("✓ 主操作成功：{}", result);
        }
        Err(_) => {
            info!("⚠ 主操作失败，使用降级方案");
            let fallback_result = fallback_operation().await;
            info!("✓ 降级方案成功：{}", fallback_result);
        }
    }

    Ok(())
}

/// 自定义错误示例
async fn demo_custom_error() -> anyhow::Result<()> {
    info!("\n--- 自定义错误 ---");

    // 定义业务错误
    #[derive(Debug)]
    enum BusinessError {
        NotFound(String),
        InvalidInput(String),
        ExternalServiceError(String),
    }

    impl std::fmt::Display for BusinessError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                BusinessError::NotFound(msg) => write!(f, "未找到：{}", msg),
                BusinessError::InvalidInput(msg) => write!(f, "无效输入：{}", msg),
                BusinessError::ExternalServiceError(msg) => {
                    write!(f, "外部服务错误：{}", msg)
                }
            }
        }
    }

    impl std::error::Error for BusinessError {}

    // 使用业务错误
    let error = BusinessError::NotFound("用户 ID 123".to_string());
    info!("自定义错误：{}", error);

    // 转换为 anyhow::Error
    let anyhow_error: anyhow::Error = error.into();
    info!("转换为 anyhow::Error: {}", anyhow_error);

    Ok(())
}

// 辅助函数

fn some_function_that_returns_provider_error() -> Result<(), ProviderError> {
    Err(ProviderError::Message("测试错误".to_string()))
}

async fn flaky_operation(attempt: usize) -> Result<String, String> {
    if attempt < 2 {
        Err("临时错误".to_string())
    } else {
        Ok("成功".to_string())
    }
}

async fn primary_operation() -> Result<String, String> {
    Err("主操作失败".to_string())
}

async fn fallback_operation() -> String {
    "降级方案结果".to_string()
}
