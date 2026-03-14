//! agentkit 聚合入口 crate。
//!
//! 本 crate 作为 agentkit 项目的主要入口点，提供以下功能：
//! - 对外统一导出 core（抽象层）与 runtime（编排层）
//! - 为具体项目提供一个"放自定义 provider/tools/skills 的位置"
//! - 提供便捷的 prelude 模块，简化常用类型的导入
//!
//! 模块结构：
//! - `core`: 重新导出 agentkit-core，提供核心抽象接口
//! - `runtime`: 重新导出 agentkit-runtime，提供运行时实现
//! - `provider`: Provider（模型提供者）实现与示例
//! - `skills`: Skills（技能）实现与示例
//! - `tools`: Tools（工具）实现与示例
//!
//! 使用示例：
//! ```rust
//! use agentkit::prelude::*;
//!
//! // 使用技能
//! let echo_skill = agentkit::skills::EchoSkill::new();
//!
//! // 使用运行时
//! let agent = SimpleAgent::new(provider);
//! ```

/// 导出 core 抽象层（traits + 共享类型）。
pub use agentkit_core as core;
/// 导出 runtime 编排层（agent loop 等实现）。
pub use agentkit_runtime as runtime;

/// 常用 runtime 类型的便捷导出。
pub use agentkit_runtime::{SimpleAgent, SkillRegistry, ToolCallingAgent, ToolRegistry};

/// 常用导入集合（prelude）。
///
/// 使用方式：`use agentkit::prelude::*;`
///
/// 这个模块重新导出了最常用的类型和 trait，避免用户手动导入多个模块。
pub mod prelude {
    /// core 抽象层常用 trait。
    pub use crate::core::{agent::Agent, provider::LlmProvider, tool::Tool};
    /// core 常用类型与错误。
    pub use crate::core::{
        agent::types::*, channel::types::*, error::*, memory::types::*, provider::types::*,
        skill::types::*, tool::types::*,
    };
    /// runtime 常用实现。
    pub use crate::{SimpleAgent, SkillRegistry, ToolCallingAgent, ToolRegistry};
}

/// Provider（模型提供者）实现与示例。
///
/// 本模块包含各种 LLM 提供者的具体实现，如：
/// - OpenAI Provider
/// - Anthropic Provider  
/// - 本地模型 Provider
/// - 自定义 Provider 示例
pub mod provider;

/// Embedding（向量嵌入）实现与示例。
pub mod embed;

/// Retrieval（语义检索）实现与示例。
pub mod retrieval;

/// Skills（技能）实现与示例。
///
/// 本模块包含具体的技能实现，展示如何构建可复用的技能单元。
/// 技能是对 Tool/Provider/Memory 的高级封装，专注于解决特定问题。
///
/// 包含示例：
/// - EchoSkill: 简单的回显技能，作为实现参考
/// - 后续可添加：文件处理、数据分析、网络请求等技能
pub mod skills;

/// Tools（工具）实现与示例。
///
/// 本模块包含各种工具的具体实现，工具是技能的基础构建块。
///
/// 包含示例：
/// - Shell 工具：执行系统命令
/// - HTTP 工具：发送网络请求
/// - 文件工具：读写文件操作
pub mod tools;
