//! agentkit 的最小运行时（runtime）示例。
//!
//! 该 crate 的职责是提供“编排层”的实现（如何调用 provider、如何循环、如何调用工具等）。
//! 目前仅提供一个最小的 `SimpleAgent`，用于演示如何基于 `agentkit-core` 的 trait 进行组装。

/// 轨迹持久化与回放（trace/replay）。
pub mod trace;

mod policy;
mod default_runtime;
mod tool_execution;
mod tool_registry;
mod utils;

pub use agentkit_core::{
    agent::types::{AgentInput, AgentOutput},
    channel::types::{ChannelEvent, DebugEvent, ErrorEvent, TokenDeltaEvent},
    error::{AgentError, ToolError},
    provider::LlmProvider,
    runtime::{NoopRuntimeObserver, Runtime, RuntimeObserver},
    tool::Tool,
};
pub use policy::{
    AllowAllToolPolicy, CommandPolicyConfig, DefaultToolPolicy, ToolCallContext, ToolPolicy,
};
pub use default_runtime::DefaultRuntime;
pub use tool_registry::{SkillRegistry, ToolRegistry};
