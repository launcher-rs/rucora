//! AgentKit 带重试的 Provider 示例
//!
//! 展示如何使用带重试机制的 Provider。
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example 11_resilient_provider
//! ```
//!
//! ## 功能演示
//!
//! 1. **重试策略配置** - 配置重试参数
//! 2. **指数退避** - 自动延迟计算
//! 3. **错误恢复** - 处理临时错误
//! 4. **实际使用** - 与 OpenAI Provider 集成

use agentkit::agent::SimpleAgent;
use agentkit::prelude::Agent;
use agentkit::provider::OpenAiProvider;
use agentkit::provider::resilient::ResilientProvider;
use std::sync::Arc;
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
    info!("║   AgentKit 带重试 Provider 示例       ║");
    info!("╚════════════════════════════════════════╝\n");

    // 检查配置
    if std::env::var("OPENAI_API_KEY").is_err() {
        info!("⚠ 未设置 API 配置");
        info!("   export OPENAI_API_KEY=sk-your-key");
        info!("\n注意：以下演示将跳过实际 API 调用\n");
    }

    // ═══════════════════════════════════════════════════════════
    // 重试策略说明
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("重试策略配置:");
    info!("═══════════════════════════════════════");
    info!("• max_retries: 最大重试次数");
    info!("• base_delay_ms: 基础延迟（毫秒）");
    info!("• max_delay_ms: 最大延迟（毫秒）");
    info!("• exponential_backoff: 指数退避");
    info!("═══════════════════════════════════════\n");

    info!("重试机制的工作原理:");
    info!("1. 首次请求失败后，等待 base_delay_ms");
    info!("2. 每次重试后，延迟时间翻倍（指数退避）");
    info!("3. 延迟时间不超过 max_delay_ms");
    info!("4. 达到 max_retries 次后，返回最终错误\n");

    // ═══════════════════════════════════════════════════════════
    // 示例 1: 创建 ResilientProvider
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 1: 创建 ResilientProvider");
    info!("═══════════════════════════════════════\n");

    if std::env::var("OPENAI_API_KEY").is_ok() {
        info!("1.1 创建 OpenAI Provider...");
        let openai_provider = OpenAiProvider::from_env()?;
        info!("✓ OpenAI Provider 创建成功\n");

        info!("1.2 包装为 ResilientProvider...");
        let resilient_provider = ResilientProvider::new(Arc::new(openai_provider)).with_config(
            agentkit::provider::resilient::RetryConfig {
                max_retries: 3,
                base_delay_ms: 500,
                max_delay_ms: 10000,
                timeout_ms: None,
                retry_non_retriable_once: false,
            },
        );
        info!("✓ ResilientProvider 创建成功\n");

        info!("1.3 测试 ResilientProvider...");
        let model_name = std::env::var("MODEL_NAME").unwrap_or_else(|_| "gpt-4o-mini".to_string());

        use agentkit_core::provider::{LlmProvider, types::ChatRequest};

        let request =
            ChatRequest::from_user_text("你好，请简单介绍一下自己。").with_model(&model_name);

        match resilient_provider.chat(request).await {
            Ok(response) => {
                info!("✓ 请求成功");
                info!(
                    "  响应：{}",
                    response
                        .message
                        .content
                        .chars()
                        .take(50)
                        .collect::<String>()
                );
                if let Some(usage) = response.usage {
                    info!(
                        "  Token 使用：输入={}, 输出={}, 总计={}",
                        usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
                    );
                }
            }
            Err(e) => {
                info!("✗ 请求失败：{}\n", e);
            }
        }
        info!("");
    } else {
        info!("⚠ 跳过实际 API 测试（未设置 API Key）\n");
    }

    // ═══════════════════════════════════════════════════════════
    // 示例 2: 与 Agent 集成
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 2: 与 Agent 集成");
    info!("═══════════════════════════════════════\n");

    if std::env::var("OPENAI_API_KEY").is_ok() {
        info!("2.1 创建带重试的 Agent...");

        let openai_provider = OpenAiProvider::from_env()?;
        let resilient_provider = ResilientProvider::new(Arc::new(openai_provider))
            .with_config(agentkit::provider::resilient::RetryConfig::default());

        let model_name = std::env::var("MODEL_NAME").unwrap_or_else(|_| "gpt-4o-mini".to_string());

        let agent = SimpleAgent::builder()
            .provider(resilient_provider)
            .model(&model_name)
            .system_prompt("你是一个友好的助手。")
            .build();

        info!("✓ 带重试的 Agent 创建成功\n");

        info!("2.2 测试 Agent...");
        let task = "请用一句话解释什么是人工智能。";
        info!("任务：\"{}\"\n", task);

        match agent.run(task.into()).await {
            Ok(output) => {
                if let Some(text) = output.text() {
                    info!("回答：\n{}\n", text);
                }
            }
            Err(e) => {
                info!("Agent 运行出错：{}\n", e);
            }
        }
    } else {
        info!("⚠ 跳过 Agent 测试（未设置 API Key）\n");
    }

    // ═══════════════════════════════════════════════════════════
    // 示例 3: 重试场景说明
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 3: 重试场景说明");
    info!("═══════════════════════════════════════\n");

    info!("3.1 会触发重试的错误类型:");
    info!("  - 网络超时");
    info!("  - 5xx 服务器错误");
    info!("  - 429 请求限流");
    info!("  - 连接中断\n");

    info!("3.2 不会重试的错误类型:");
    info!("  - 4xx 客户端错误（如认证失败）");
    info!("  - 无效请求参数");
    info!("  - 业务逻辑错误\n");

    info!("3.3 重试延迟计算示例:");
    info!("  假设 base_delay_ms = 500ms, exponential_backoff = true");
    info!("  第 1 次重试：等待 500ms");
    info!("  第 2 次重试：等待 1000ms (500 × 2)");
    info!("  第 3 次重试：等待 2000ms (500 × 4)");
    info!("  第 4 次重试：等待 4000ms (500 × 8)");
    info!("  第 5 次重试：等待 8000ms (500 × 16)，但不超过 max_delay_ms\n");

    // ═══════════════════════════════════════════════════════════
    // 总结
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例完成！");
    info!("═══════════════════════════════════════\n");

    info!("📝 ResilientProvider 总结：\n");

    info!("1. 重试策略配置:");
    info!("   - max_retries: 最大重试次数（默认 3 次）");
    info!("   - base_delay_ms: 基础延迟（默认 1000ms）");
    info!("   - max_delay_ms: 最大延迟（默认 30000ms）");
    info!("   - retry_non_retriable_once: 是否重试不可重试错误一次\n");

    info!("2. 使用场景:");
    info!("   - 网络不稳定的环境");
    info!("   - 调用第三方 API");
    info!("   - 需要高可用性的生产环境\n");

    info!("3. 最佳实践:");
    info!("   - 生产环境建议启用重试");
    info!("   - 设置合理的 max_retries（3-5 次）");
    info!("   - 配合日志监控重试情况");
    info!("   - 对于幂等操作可以设置更多重试次数\n");

    info!("4. 注意事项:");
    info!("   - 重试会增加总延迟");
    info!("   - 非幂等操作慎用重试");
    info!("   - 监控重试率，过高可能说明有问题\n");

    Ok(())
}
