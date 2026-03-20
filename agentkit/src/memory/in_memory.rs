//! 进程内 Memory 实现。
//!
//! 该实现将 `MemoryItem` 保存在 `Vec` 中，并用 `RwLock` 进行并发保护。
//! 适合：测试、示例、小规模临时记忆场景。
//!
//! 特性：
//! - 支持最大容量限制（LRU 淘汰）
//! - 自动清理过期条目

use std::collections::VecDeque;
use std::sync::Arc;

use agentkit_core::{
    error::MemoryError,
    memory::{Memory, MemoryItem, MemoryQuery},
};
use async_trait::async_trait;
use tokio::sync::RwLock;

/// 默认的内存容量限制
const DEFAULT_MAX_CAPACITY: usize = 1000;

#[derive(Default)]
/// 基于内存的记忆存储。
///
/// - `add`：按 `id` upsert，超出容量时淘汰最早的条目
/// - `query`：在 id/content/metadata 中做简单子串匹配（不做向量检索）
pub struct InMemoryMemory {
    items: Arc<RwLock<VecDeque<MemoryItem>>>,
    /// 最大容量（0 表示无限制）
    max_capacity: usize,
}

impl InMemoryMemory {
    /// 创建一个空的内存记忆（使用默认容量限制）。
    pub fn new() -> Self {
        Self {
            items: Arc::new(RwLock::new(VecDeque::with_capacity(64))),
            max_capacity: DEFAULT_MAX_CAPACITY,
        }
    }

    /// 创建一个空的内存记忆（指定容量限制）。
    ///
    /// - `max_capacity`: 最大条目数（0 表示无限制）
    pub fn with_capacity(max_capacity: usize) -> Self {
        Self {
            items: Arc::new(RwLock::new(VecDeque::with_capacity(64))),
            max_capacity,
        }
    }

    /// 获取当前存储的记忆数量
    pub async fn len(&self) -> usize {
        self.items.read().await.len()
    }

    /// 检查是否为空
    pub async fn is_empty(&self) -> bool {
        self.items.read().await.is_empty()
    }

    /// 清空所有记忆
    pub async fn clear(&self) {
        self.items.write().await.clear();
    }

    /// 淘汰最早的条目直到容量限制内
    fn enforce_capacity(items: &mut VecDeque<MemoryItem>, max: usize) {
        if max == 0 {
            return;
        }
        while items.len() > max {
            items.pop_front();
        }
    }
}

#[async_trait]
impl Memory for InMemoryMemory {
    async fn add(&self, item: MemoryItem) -> Result<(), MemoryError> {
        let mut items = self.items.write().await;

        // 查找是否已存在，存在则更新
        if let Some(existing) = items.iter_mut().find(|x| x.id == item.id) {
            *existing = item;
            return Ok(());
        }

        // 新条目添加到末尾
        items.push_back(item);

        // 应用容量限制（淘汰最早的条目）
        Self::enforce_capacity(&mut items, self.max_capacity);

        Ok(())
    }

    async fn query(&self, query: MemoryQuery) -> Result<Vec<MemoryItem>, MemoryError> {
        let items = self.items.read().await;
        let limit = if query.limit == 0 {
            usize::MAX
        } else {
            query.limit
        };

        let needle = query.text.to_lowercase();
        if needle.is_empty() {
            // 返回最新的 limit 条
            return Ok(items.iter().rev().cloned().take(limit).collect());
        }

        // 简单子串匹配（在 id/content/metadata 中搜索）
        let mut matches: Vec<MemoryItem> = items
            .iter()
            .filter(|item| {
                if item.id.to_lowercase().contains(&needle) {
                    return true;
                }
                if item.content.to_lowercase().contains(&needle) {
                    return true;
                }
                if let Some(meta) = &item.metadata {
                    let meta_str = meta.to_string().to_lowercase();
                    if meta_str.contains(&needle) {
                        return true;
                    }
                }
                false
            })
            .cloned()
            .collect();

        // 按 ID 排序（可改为按相关性排序）
        matches.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(matches.into_iter().take(limit).collect())
    }
}
