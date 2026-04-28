use async_trait::async_trait;

use crate::{
    error::MemoryError,
    memory::types::{MemoryItem, MemoryQuery},
};

/// Memory（记忆）接口。
///
/// 该接口刻意保持最小化：
/// - `add` 写入一条记忆
/// - `query` 按查询条件检索记忆
#[async_trait]
pub trait Memory: Send + Sync {
    /// 写入一条记忆。
    async fn add(&self, item: MemoryItem) -> Result<(), MemoryError>;

    /// 检索记忆。
    async fn query(&self, query: MemoryQuery) -> Result<Vec<MemoryItem>, MemoryError>;
}
