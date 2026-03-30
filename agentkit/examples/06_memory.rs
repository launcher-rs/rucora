//! AgentKit 记忆系统示例
//!
//! 展示如何使用记忆系统存储和检索信息。
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example 06_memory
//! ```

use agentkit::memory::InMemoryMemory;
use agentkit_core::memory::{Memory, MemoryItem, MemoryQuery};
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
    info!("║   AgentKit 记忆系统示例               ║");
    info!("╚════════════════════════════════════════╝\n");

    // 创建记忆系统
    info!("1. 创建记忆系统...");
    let memory = InMemoryMemory::new();
    info!("✓ 记忆系统创建成功\n");

    // 添加记忆
    info!("2. 添加记忆...\n");

    memory
        .add(MemoryItem {
            id: "1".to_string(),
            content: "用户喜欢 Python 编程".to_string(),
            metadata: None,
        })
        .await?;

    memory
        .add(MemoryItem {
            id: "2".to_string(),
            content: "用户在北京工作".to_string(),
            metadata: None,
        })
        .await?;

    memory
        .add(MemoryItem {
            id: "3".to_string(),
            content: "用户有一只叫咪咪的猫".to_string(),
            metadata: None,
        })
        .await?;

    info!("✓ 已添加 3 条记忆\n");

    // 查询记忆
    info!("═══════════════════════════════════════");
    info!("3. 查询记忆");
    info!("═══════════════════════════════════════\n");

    let queries = vec!["用户的喜好", "用户的工作"];

    for query in queries {
        info!("查询：\"{}\"", query);

        let results = memory
            .query(MemoryQuery {
                text: query.to_string(),
                limit: 5,
            })
            .await?;

        info!("找到 {} 条相关记忆:", results.len());
        for (i, item) in results.iter().enumerate() {
            info!("  {}. {}", i + 1, item.content);
        }
        info!("");
    }

    // 删除记忆
    info!("═══════════════════════════════════════");
    info!("4. 删除记忆");
    info!("═══════════════════════════════════════\n");

    memory.clear().await;
    info!("✓ 已清空所有记忆\n");

    info!("\n═══════════════════════════════════════");
    info!("示例完成！");
    info!("═══════════════════════════════════════");

    Ok(())
}
