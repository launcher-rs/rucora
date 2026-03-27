//! Resilient Provider 使用示例
//!
//! Resilient Provider 提供自动重试和故障转移功能
//!
//! # 运行方式
//!
//! ```bash
//! export OPENAI_API_KEY=sk-xxx
//! cargo run --example 08_resilient_provider -p agentkit
//! ```

use agentkit::provider::{OpenAiProvider, ResilientProvider};
use agentkit_core::provider::LlmProvider;
use agentkit_core::provider::types::ChatRequest;
use std::sync::Arc;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔════════════════════════════════════════════════════════╗");
    info!("║     AgentKit Resilient Provider 使用示例               ║");
    info!("╚════════════════════════════════════════════════════════╝\n");

    // 检查 API Key
    if std::env::var("OPENAI_API_KEY").is_err() {
        info!("❌ 未找到 OPENAI_API_KEY");
        info!("\n请设置环境变量：");
        info!("  export OPENAI_API_KEY=sk-xxx\n");
        return Ok(());
    }

    // 1. 创建基础 Provider
    info!("=== 1. 创建基础 Provider ===\n");

    let base_provider = OpenAiProvider::from_env()?;
    info!("✓ OpenAI Provider 初始化成功\n");

    // 2. 创建 Resilient Provider
    info!("=== 2. 创建 Resilient Provider ===\n");

    use agentkit::provider::resilient::RetryConfig;

    let retry_config = RetryConfig {
        max_retries: 3,
        base_delay_ms: 100,
        max_delay_ms: 10000,
        timeout_ms: Some(30000),
        retry_non_retriable_once: false,
    };

    info!("✓ Resilient Provider 创建成功");
    info!("  最大重试次数：{}", retry_config.max_retries);
    info!("  基础延迟：{}ms", retry_config.base_delay_ms);
    info!("  最大延迟：{}ms", retry_config.max_delay_ms);
    info!("");

    let resilient = ResilientProvider::new(Arc::new(base_provider)).with_config(retry_config);

    // 3. 测试正常请求
    info!("=== 3. 测试正常请求 ===\n");

    let request =
        ChatRequest::from_user_text("用一句话介绍 Rust 编程语言").with_model("gpt-4o-mini");

    match resilient.chat(request).await {
        Ok(response) => {
            info!("✓ 回复：{}", response.message.content);
        }
        Err(e) => {
            info!("❌ 错误：{}", e);
        }
    }

    // 4. 测试重试机制（模拟）
    info!("\n=== 4. 重试机制说明 ===\n");

    info!("Resilient Provider 的重试机制：");
    info!("1. 首次请求失败后，会自动重试");
    info!("2. 重试延迟采用指数退避策略：");
    info!("   - 第 1 次重试：延迟 100ms");
    info!("   - 第 2 次重试：延迟 200ms");
    info!("   - 第 3 次重试：延迟 400ms");
    info!("   - 以此类推，直到达到最大延迟");
    info!("3. 可配置的参数：");
    info!("   - max_retries: 最大重试次数");
    info!("   - initial_delay: 初始延迟");
    info!("   - max_delay: 最大延迟");
    info!("   - multiplier: 延迟倍数");

    // 5. 使用场景
    info!("\n=== 5. 使用场景 ===\n");

    info!("Resilient Provider 适用于：");
    info!("1. 网络不稳定的环境");
    info!("2. API 偶尔超时的情况");
    info!("3. 需要高可用性的生产环境");
    info!("4. 批量处理任务，避免单个失败影响整体");

    info!("\n=== Resilient Provider 示例完成 ===");

    Ok(())
}
