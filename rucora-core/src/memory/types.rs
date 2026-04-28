//! Memory（记忆）相关的类型定义。

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 一条记忆数据。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryItem {
    /// 记忆 id。
    pub id: String,
    /// 记忆内容。
    pub content: String,
    /// 可选元数据。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// 记忆查询。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryQuery {
    /// 查询文本。
    pub text: String,
    /// 结果数量限制。
    #[serde(default)]
    pub limit: usize,
}
