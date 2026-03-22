//! 性能优化示例
//!
//! 本示例展示 AgentKit 中的性能优化技巧

use agentkit::core::memory::{Memory, MemoryItem};
use agentkit::memory::InMemoryMemory;
use futures_util::future::join_all;
use std::sync::Arc;
use std::time::Instant;
use tracing::info;

/// 运行示例
pub async fn run() -> anyhow::Result<()> {
    info!("=== 性能优化示例 ===");

    // 1. 批量操作
    demo_batch_operations().await?;

    // 2. 并发控制
    demo_concurrency_control().await?;

    // 3. 内存管理
    demo_memory_management().await?;

    Ok(())
}

/// 批量操作示例
async fn demo_batch_operations() -> anyhow::Result<()> {
    info!("\n--- 批量操作 ---");

    let memory = InMemoryMemory::new();

    // 批量添加记忆
    let start = Instant::now();

    // 方式 1：逐个添加
    for i in 0..10 {
        memory
            .add(MemoryItem {
                id: format!("item_{}", i),
                content: format!("内容 {}", i),
                metadata: None,
            })
            .await?;
    }
    let sequential_time = start.elapsed();
    info!("逐个添加 10 项：{:?}", sequential_time);

    // 方式 2：并发添加
    let start = Instant::now();
    let items: Vec<MemoryItem> = (10..20)
        .map(|i| MemoryItem {
            id: format!("batch_{}", i),
            content: format!("批量内容 {}", i),
            metadata: None,
        })
        .collect();

    let futures: Vec<_> = items.iter().map(|item| memory.add(item.clone())).collect();

    join_all(futures).await;
    let batch_time = start.elapsed();
    info!("并发添加 10 项：{:?}", batch_time);

    info!("✓ 批量操作演示完成");

    Ok(())
}

/// 并发控制示例
async fn demo_concurrency_control() -> anyhow::Result<()> {
    info!("\n--- 并发控制 ---");

    let memory = Arc::new(InMemoryMemory::new());

    // 限制并发数
    let max_concurrency = 5;
    let total_tasks = 20;

    let start = Instant::now();

    // 使用信号量限制并发
    let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrency));
    let mut handles = vec![];

    for i in 0..total_tasks {
        let memory = memory.clone();
        let semaphore = semaphore.clone();

        let handle = tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();

            // 模拟工作
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

            memory
                .add(MemoryItem {
                    id: format!("task_{}", i),
                    content: format!("任务 {}", i),
                    metadata: None,
                })
                .await
        });

        handles.push(handle);
    }

    // 等待所有任务完成
    let results = join_all(handles).await;
    let success_count = results.iter().filter(|r| r.is_ok()).count();

    let elapsed = start.elapsed();
    info!(
        "并发控制：{} 任务，最大并发 {}，耗时：{:?}",
        success_count, max_concurrency, elapsed
    );

    Ok(())
}

/// 内存管理示例
async fn demo_memory_management() -> anyhow::Result<()> {
    info!("\n--- 内存管理 ---");

    // 使用带容量限制的 Memory
    let memory = InMemoryMemory::with_capacity(100);

    // 添加大量数据
    for i in 0..150 {
        memory
            .add(MemoryItem {
                id: format!("item_{}", i),
                content: format!("内容 {}", i),
                metadata: None,
            })
            .await?;
    }

    let count = memory.len().await;
    info!("添加 150 项后，实际存储：{} 项（限制 100）", count);

    // 清空内存
    memory.clear().await;
    let count = memory.len().await;
    info!("清空后：{} 项", count);

    info!("✓ 内存管理演示完成");

    Ok(())
}
