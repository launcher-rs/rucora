//! Agent 核心抽象模块。
//!
//! Agent 通常是“运行时编排”的入口：
//! - 接收输入（消息、任务、上下文）
//! - 调用 Provider/Tool/Memory/Skill
//! - 输出最终结果或事件流

pub mod r#trait;
pub mod types;

/// 重新导出 agent 相关 trait，方便 `agentkit_core::agent::*` 使用。
pub use r#trait::*;

/// 重新导出 agent 相关类型，方便使用。
pub use types::*;
