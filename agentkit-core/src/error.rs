//! agentkit-core 的统一错误类型定义。
//!
//! 该文件只描述错误“类别”和最小信息，不绑定具体实现细节。

/// Provider 错误。
#[derive(thiserror::Error, Debug)]
pub enum ProviderError {
    /// 通用错误信息。
    #[error("provider error: {0}")]
    Message(String),
}

/// Tool 错误。
#[derive(thiserror::Error, Debug)]
pub enum ToolError {
    /// 通用错误信息。
    #[error("tool error: {0}")]
    Message(String),
}

/// Skill 错误。
#[derive(thiserror::Error, Debug)]
pub enum SkillError {
    /// 通用错误信息。
    #[error("skill error: {0}")]
    Message(String),
}

/// Agent 错误。
#[derive(thiserror::Error, Debug)]
pub enum AgentError {
    /// 通用错误信息。
    #[error("agent error: {0}")]
    Message(String),
}

/// Memory 错误。
#[derive(thiserror::Error, Debug)]
pub enum MemoryError {
    /// 通用错误信息。
    #[error("memory error: {0}")]
    Message(String),
}

/// Channel 错误。
#[derive(thiserror::Error, Debug)]
pub enum ChannelError {
    /// 通用错误信息。
    #[error("channel error: {0}")]
    Message(String),
}
