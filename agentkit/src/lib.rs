//! agentkit 聚合入口 crate。
//!
//! 该 crate 的定位：
//! - 对外统一导出 core（抽象层）与 runtime（编排层）
//! - 为具体项目提供一个“放自定义 provider/tools/skills 的位置”

/// 导出 core 抽象层（traits + shared types）。
pub use agentkit_core as core;
/// 导出 runtime 编排层（agent loop 等实现）。
pub use agentkit_runtime as runtime;

/// 常用 runtime 类型的便捷导出。
pub use agentkit_runtime::{SimpleAgent, ToolCallingAgent, ToolRegistry};

/// 常用导入集合（prelude）。
///
/// 使用方式：`use agentkit::prelude::*;`
pub mod prelude {
    /// core 抽象层常用 trait。
    pub use crate::core::{agent::Agent, provider::LlmProvider, tool::Tool};
    /// core 常用类型与错误。
    pub use crate::core::{
        agent::types::*,
        channel::types::*,
        error::*,
        memory::types::*,
        provider::types::*,
        skill::types::*,
        tool::types::*,
    };
    /// runtime 常用实现。
    pub use crate::{SimpleAgent, ToolCallingAgent, ToolRegistry};
}

/// Provider（模型提供者）实现与示例。
pub mod provider;
/// Tools（工具）实现与示例。
pub mod tools;
/// Skills（技能）实现与示例。
pub mod skills;
