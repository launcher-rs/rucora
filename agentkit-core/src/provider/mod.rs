//! LLM Provider 抽象模块。
//!
//! 该模块只定义接口（trait），不包含任何具体模型/平台的实现。

pub mod r#trait;
pub mod types;

/// 重新导出 provider 相关 trait，方便 `agentkit_core::provider::*` 使用。
pub use r#trait::*;

/// 重新导出 provider 相关类型，方便使用。
pub use types::*;
