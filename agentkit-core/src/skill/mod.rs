//! Skill（技能）抽象模块。
//!
//! Skill 通常比 Tool 更高层：它可能会组合多个 Tool/Provider/Memory 来完成一个完整的任务流程。
//! 在 core 层，我们只定义技能接口，具体编排在 runtime 或具体项目中实现。

pub mod types;
pub mod r#trait;

/// 重新导出 skill 相关 trait，方便 `agentkit_core::skill::*` 使用。
pub use r#trait::*;

/// 重新导出 skill 相关类型，方便使用。
pub use types::*;
