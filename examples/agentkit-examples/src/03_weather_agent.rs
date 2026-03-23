//! 天气查询 Agent 示例
//!
//! 展示如何使用 Agent API
//!
//! # 运行方式
//!
//! ```bash
//! cargo run --bin weather-agent
//! ```

mod utils;

use agentkit::agent::DefaultAgent;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use utils::MockProvider;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("=== 天气查询 Agent 示例 ===\n");
    info!("展示 Agent API 的使用\n");

    // 创建 Mock Provider
    let provider = MockProvider::with_response("北京今天晴朗，气温 25°C。");

    // 创建 Agent
    info!("创建 Agent:");
    let agent = DefaultAgent::builder()
        .provider(provider)
        .system_prompt("你是一个天气查询助手")
        .build();

    info!("✓ Agent 创建成功\n");

    // 测试对话
    let input = agentkit::prelude::AgentInput::new("北京今天天气怎么样？");

    match agent.run(input).await {
        Ok(output) => {
            if let Some(content) = output.text() {
                info!("✓ Agent 回复：{}", content);
            }
        }
        Err(e) => {
            info!("❌ 错误：{}", e);
        }
    }

    info!("\n=== 示例完成 ===");

    Ok(())
}
