//! Memory 使用示例
//!
//! 展示如何使用记忆系统存储和检索信息
//!
//! # 运行方式
//!
//! ```bash
//! cargo run --example 04_memory -p agentkit
//! ```

use agentkit::memory::{FileMemory, InMemoryMemory};
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

    info!("╔════════════════════════════════════════════════════════╗");
    info!("║         AgentKit Memory 使用示例                       ║");
    info!("╚════════════════════════════════════════════════════════╝\n");

    // 示例 1: InMemory Memory
    info!("=== InMemory Memory ===");
    test_in_memory().await?;

    // 示例 2: File Memory
    info!("\n=== File Memory ===");
    test_file_memory().await?;

    info!("\n=== 所有 Memory 测试完成 ===");

    Ok(())
}

/// 测试进程内记忆
async fn test_in_memory() -> anyhow::Result<()> {
    let memory = InMemoryMemory::new();

    // 添加记忆
    info!("添加记忆...");
    memory
        .add(MemoryItem {
            id: "user:name".to_string(),
            content: "张三".to_string(),
            metadata: Some(serde_json::json!({"category": "personal"})),
        })
        .await?;
    info!("✓ 已添加：user:name = 张三");

    memory
        .add(MemoryItem {
            id: "user:lang".to_string(),
            content: "Rust".to_string(),
            metadata: Some(serde_json::json!({"category": "technical"})),
        })
        .await?;
    info!("✓ 已添加：user:lang = Rust");

    memory
        .add(MemoryItem {
            id: "project:name".to_string(),
            content: "AgentKit".to_string(),
            metadata: Some(serde_json::json!({"category": "project"})),
        })
        .await?;
    info!("✓ 已添加：project:name = AgentKit\n");

    // 检索记忆
    info!("检索记忆...");
    let results = memory
        .query(MemoryQuery {
            text: "user".to_string(),
            limit: 10,
        })
        .await?;
    info!("✓ 找到 {} 条匹配'user'的记忆", results.len());
    for item in &results {
        info!("  - {}: {}", item.id, item.content);
    }

    // 按类别检索
    info!("\n按类别检索...");
    let results = memory
        .query(MemoryQuery {
            text: "technical".to_string(),
            limit: 10,
        })
        .await?;
    info!("✓ 找到 {} 条匹配'technical'的记忆", results.len());
    for item in &results {
        info!("  - {}: {}", item.id, item.content);
    }

    Ok(())
}

/// 测试文件记忆
async fn test_file_memory() -> anyhow::Result<()> {
    let memory = FileMemory::new("memory_test.json");

    // 添加记忆
    info!("添加记忆到文件...");
    memory
        .add(MemoryItem {
            id: "config:theme".to_string(),
            content: "dark".to_string(),
            metadata: Some(serde_json::json!({"category": "config"})),
        })
        .await?;
    info!("✓ 已添加：config:theme = dark");

    memory
        .add(MemoryItem {
            id: "config:lang".to_string(),
            content: "zh-CN".to_string(),
            metadata: Some(serde_json::json!({"category": "config"})),
        })
        .await?;
    info!("✓ 已添加：config:lang = zh-CN\n");

    // 检索记忆
    info!("检索记忆...");
    let results = memory
        .query(MemoryQuery {
            text: "config".to_string(),
            limit: 10,
        })
        .await?;
    info!("✓ 找到 {} 条匹配'config'的记忆", results.len());
    for item in &results {
        info!("  - {}: {}", item.id, item.content);
    }

    // 清理测试文件
    let _ = std::fs::remove_file("memory_test.json");
    info!("\n✓ 已清理测试文件");

    Ok(())
}
