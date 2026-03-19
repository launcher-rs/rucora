use std::sync::Arc;

use agentkit_core::{
    error::MemoryError,
    memory::{Memory, MemoryItem, MemoryQuery},
};
use async_trait::async_trait;
use tokio::sync::RwLock;

#[derive(Default)]
pub struct InMemoryMemory {
    items: Arc<RwLock<Vec<MemoryItem>>>,
}

impl InMemoryMemory {
    pub fn new() -> Self {
        Self {
            items: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

#[async_trait]
impl Memory for InMemoryMemory {
    async fn add(&self, item: MemoryItem) -> Result<(), MemoryError> {
        let mut items = self.items.write().await;
        if let Some(existing) = items.iter_mut().find(|x| x.id == item.id) {
            *existing = item;
            return Ok(());
        }
        items.push(item);
        Ok(())
    }

    async fn query(&self, query: MemoryQuery) -> Result<Vec<MemoryItem>, MemoryError> {
        let items = self.items.read().await;
        let limit = if query.limit == 0 { usize::MAX } else { query.limit };

        let needle = query.text.to_lowercase();
        if needle.is_empty() {
            return Ok(items.iter().cloned().take(limit).collect());
        }

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

        matches.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(matches.into_iter().take(limit).collect())
    }
}
