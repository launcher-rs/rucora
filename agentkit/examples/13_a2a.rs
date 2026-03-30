//! AgentKit A2A（Agent-to-Agent）示例
//!
//! 展示 A2A 协议集成，实现 Agent 间通信。
//!
//! ## 运行方法
//! ```bash
//! cargo run --example 13_a2a --features a2a
//! ```

#[cfg(feature = "a2a")]
use tracing::{Level, info};
#[cfg(feature = "a2a")]
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
#[cfg(feature = "a2a")]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔════════════════════════════════════════╗");
    info!("║   AgentKit A2A 示例                   ║");
    info!("╚════════════════════════════════════════╝\n");

    info!("A2A（Agent-to-Agent）协议用于 Agent 之间的通信和协作。\n");

    info!("═══════════════════════════════════════");
    info!("A2A 主要功能:");
    info!("═══════════════════════════════════════");
    info!("1. Agent 发现 - 发现网络中的其他 Agent");
    info!("2. 任务委托 - 将任务委托给专业 Agent");
    info!("3. 结果聚合 - 聚合多个 Agent 的结果");
    info!("4. 协作对话 - 多 Agent 协作对话");
    info!("═══════════════════════════════════════\n");

    info!("使用场景:");
    info!("• 多专家系统 - 不同 Agent 擅长不同领域");
    info!("• 任务分解 - 复杂任务分解给多个 Agent");
    info!("• 负载均衡 - 分散请求到多个 Agent");
    info!("• 冗余备份 - 多个 Agent 提供相同服务\n");

    info!("提示：A2A 功能需要启用 a2a feature");
    info!("运行：cargo run --example 13_a2a --features a2a\n");

    info!("示例完成！");

    Ok(())
}

#[cfg(not(feature = "a2a"))]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("A2A feature 未启用");
    println!("请使用以下命令运行:");
    println!("  cargo run --example 13_a2a --features a2a");
    Ok(())
}
