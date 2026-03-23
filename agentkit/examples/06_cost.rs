//! Cost 使用示例
//!
//! 展示如何使用 Token 计数和成本管理
//!
//! # 运行方式
//!
//! ```bash
//! cargo run --example 06_cost -p agentkit
//! ```

use agentkit::cost::TokenCounter;
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
    info!("║         AgentKit Cost 使用示例                         ║");
    info!("╚════════════════════════════════════════════════════════╝\n");

    // Token Counter 示例
    info!("=== Token Counter ===");
    test_token_counter()?;

    info!("\n=== Cost 测试完成 ===");

    Ok(())
}

/// 测试 Token 计数器
fn test_token_counter() -> anyhow::Result<()> {
    // 创建不同模型的计数器
    let gpt4_counter = TokenCounter::new("gpt-4");
    let gpt35_counter = TokenCounter::new("gpt-3.5-turbo");

    let text = "Hello, World! 这是一个测试文本。";

    // 计算不同模型的 token 数
    let gpt4_tokens = gpt4_counter.count_text(text);
    let gpt35_tokens = gpt35_counter.count_text(text);

    info!("文本：\"{}\"", text);
    info!("GPT-4 Token 数：{}", gpt4_tokens);
    info!("GPT-3.5 Token 数：{}", gpt35_tokens);

    Ok(())
}
