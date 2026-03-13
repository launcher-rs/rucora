//! Memory（记忆）抽象模块。
//!
//! 记忆用于保存与检索信息：
//! - 短期上下文（对话历史）
//! - 长期知识（向量库/数据库）
//! 在 core 层，我们只定义最小接口，具体存储策略交给实现层。

pub mod r#trait;
pub mod types;

/// 重新导出 memory 相关 trait，方便 `agentkit_core::memory::*` 使用。
pub use r#trait::*;

/// 重新导出 memory 相关类型，方便使用。
pub use types::*;
