use async_trait::async_trait;

use crate::{
    agent::types::{AgentInput, AgentOutput},
    error::AgentError,
};

/// runtime 规范（可替换运行时）。
///
/// 设计意图：
/// - `agentkit-core` 仅定义“运行时需要满足的最小能力”，不绑定任何具体实现。
/// - `agentkit-runtime` 提供默认实现。
/// - 业务方也可以按该 trait 自定义 runtime（例如：加入 tracing、限流、并发调度、
///   多 agent 编排、Planner/Executor、不同的 tool loop 策略等）。
///
/// 注意：
/// - 这里刻意复用 `AgentInput/AgentOutput` 作为统一输入输出，以保持 core 层类型稳定。
/// - runtime 的实现内部可以自由决定如何组织 agent loop（ReAct、tool-calling 等）。
#[async_trait]
pub trait Runtime: Send + Sync {
    /// 执行一次任务。
    ///
    /// 典型实现可能会：
    /// - 构造消息历史
    /// - 调用 provider
    /// - 解析工具调用并执行工具
    /// - 循环直到得到最终回答
    async fn run(&self, input: AgentInput) -> Result<AgentOutput, AgentError>;
}
