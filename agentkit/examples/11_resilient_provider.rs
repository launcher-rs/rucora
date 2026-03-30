//! AgentKit 带重试的 Provider 示例
//!
//! 展示如何使用带重试机制的 Provider。
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example 11_resilient_provider
//! ```

use agentkit::provider::OpenAiProvider;
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

    info!("╔════════════════════════════════════════╗");
    info!("║   AgentKit 带重试 Provider 示例       ║");
    info!("╚════════════════════════════════════════╝\n");

    // 检查配置
    if std::env::var("OPENAI_API_KEY").is_err() {
        info!("⚠ 未设置 API 配置");
        info!("   export OPENAI_API_KEY=sk-your-key");
        return Ok(());
    }

    info!("1. 创建 Provider...\n");
    let _provider = OpenAiProvider::from_env()?;
    info!("✓ Provider 创建成功\n");

    info!("═══════════════════════════════════════");
    info!("重试策略配置:");
    info!("═══════════════════════════════════════");
    info!("• max_retries: 最大重试次数");
    info!("• initial_delay_ms: 初始延迟（毫秒）");
    info!("• max_delay_ms: 最大延迟（毫秒）");
    info!("• exponential_backoff: 指数退避");
    info!("═══════════════════════════════════════\n");

    info!("重试机制的工作原理:");
    info!("1. 首次请求失败后，等待 initial_delay_ms");
    info!("2. 每次重试后，延迟时间翻倍（指数退避）");
    info!("3. 延迟时间不超过 max_delay_ms");
    info!("4. 达到 max_retries 次后，返回最终错误\n");

    info!("示例完成！");
    info!("提示：ResilientProvider 可以自动处理网络波动和临时错误");

    Ok(())
}
