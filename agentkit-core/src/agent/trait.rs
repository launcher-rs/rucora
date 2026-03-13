use async_trait::async_trait;

use crate::{
    agent::types::{AgentInput, AgentOutput},
    error::AgentError,
};

/// Agent 接口。
///
/// core 层只定义最小运行入口，具体的 loop（ReAct、Planner/Executor、Tool-calling 等）
/// 由 runtime crate 实现。
#[async_trait]
pub trait Agent: Send + Sync {
    /// 执行一次 agent 任务。
    async fn run(&self, input: AgentInput) -> Result<AgentOutput, AgentError>;
}
