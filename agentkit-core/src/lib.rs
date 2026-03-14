//! agentkit 的核心抽象层（仅包含 trait + 共享类型）。
//!
//! 设计目标：
//! - 为 LLM Provider、Tool、Skill、Agent、Memory、Channel 提供稳定的抽象接口
//! - 避免把具体实现（例如 OpenAI/Anthropic/本地模型、具体工具实现、具体运行时编排）耦合进来

/// Agent 核心抽象（运行入口）。
pub mod agent;
/// 通信渠道抽象（事件发送与订阅）。
pub mod channel;
/// 向量嵌入抽象（文本向量化）。
pub mod embed;
/// 记忆抽象（添加与检索）。
pub mod memory;
/// LLM 提供者抽象（对话/流式对话等）。
pub mod provider;
/// 技能抽象（更高层的可复用能力单元）。
pub mod skill;
/// 工具抽象（名称、输入 schema、执行）。
pub mod tool;

/// 统一错误类型定义。
pub mod error;
