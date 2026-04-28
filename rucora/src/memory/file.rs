//! 文件持久化 Memory 实现。
//!
//! 该实现把 `MemoryItem` 以 JSON 数组形式存储到磁盘文件：
//! - `add`：读入 -> upsert -> 写回（使用进程内互斥锁避免并发写冲突）
//! - `query`：读入 -> 简单子串匹配过滤
//!
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use rucora_core::{
    error::MemoryError,
    memory::{Memory, MemoryItem, MemoryQuery},
};
use tokio::sync::Mutex;

/// 基于 JSON 文件的记忆存储。
///
/// 适合本地、小规模场景；不适合高并发/大数据量（每次操作都需要读写整个文件）。
pub struct FileMemory {
    path: PathBuf,
    lock: Arc<Mutex<()>>,
}

impl FileMemory {
    /// 创建一个文件记忆，`path` 指向存储文件位置。
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            lock: Arc::new(Mutex::new(())),
        }
    }

    /// 从文件读取并解析全部记忆条目。
    async fn load_items(&self) -> Result<Vec<MemoryItem>, MemoryError> {
        let bytes = match tokio::fs::read(&self.path).await {
            Ok(b) => b,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => {
                return Err(MemoryError::Message(format!(
                    "failed to read memory file: {err}"
                )));
            }
        };

        if bytes.is_empty() {
            return Ok(Vec::new());
        }

        serde_json::from_slice::<Vec<MemoryItem>>(&bytes)
            .map_err(|e| MemoryError::Message(format!("failed to parse memory file: {e}")))
    }

    /// 将全部记忆条目序列化并写回文件。
    async fn save_items(&self, items: &[MemoryItem]) -> Result<(), MemoryError> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| MemoryError::Message(format!("failed to create dir: {e}")))?;
        }

        let data = serde_json::to_vec_pretty(items)
            .map_err(|e| MemoryError::Message(format!("failed to serialize memory: {e}")))?;

        tokio::fs::write(&self.path, data)
            .await
            .map_err(|e| MemoryError::Message(format!("failed to write memory file: {e}")))
    }
}

#[async_trait]
impl Memory for FileMemory {
    async fn add(&self, item: MemoryItem) -> Result<(), MemoryError> {
        let _g = self.lock.lock().await;
        let mut items = self.load_items().await?;

        if let Some(existing) = items.iter_mut().find(|x| x.id == item.id) {
            *existing = item;
        } else {
            items.push(item);
        }

        self.save_items(&items).await
    }

    async fn query(&self, query: MemoryQuery) -> Result<Vec<MemoryItem>, MemoryError> {
        let _g = self.lock.lock().await;
        let items = self.load_items().await?;

        let limit = if query.limit == 0 {
            usize::MAX
        } else {
            query.limit
        };
        let needle = query.text.to_lowercase();

        if needle.is_empty() {
            return Ok(items.into_iter().take(limit).collect());
        }

        let mut matches: Vec<MemoryItem> = items
            .into_iter()
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
            .collect();

        matches.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(matches.into_iter().take(limit).collect())
    }
}
