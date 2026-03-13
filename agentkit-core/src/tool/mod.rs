//! Tool（工具）抽象模块。
//!
//! Tool 通常是可以被 Agent 调用的“可执行能力”，例如：读取文件、访问网页、查询数据库等。
//! 在 core 层，我们只定义工具的接口与 schema，不关心具体执行实现。

pub mod r#trait;
pub mod types;

/// 重新导出 tool 相关 trait，方便 `agentkit_core::tool::*` 使用。
pub use r#trait::*;

/// 重新导出 tool 相关类型，方便使用。
pub use types::*;
