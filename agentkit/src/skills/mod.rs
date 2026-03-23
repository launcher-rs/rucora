//! agentkit-skills - Skills 系统
//!
//! # 概述
//!
//! Skills 是对 Tool/Provider/Memory 的高级封装，提供更高层次的抽象：
//! - 每个技能都是独立的可执行单元，具有明确的输入输出
//! - 技能应该专注于解决特定领域的问题
//! - 支持 Rhai 脚本技能和命令模板技能
//!
//! # 使用示例
//!
//! ```rust
//! use agentkit::skills::EchoSkill;
//!
//! let skill = EchoSkill::new();
//! let ctx = SkillContext { input: json!("hello") };
//! let result = skill.run(ctx).await?;
//! ```

pub use agentkit_core::skill::Skill;

/// Rhai 脚本技能（需要 `rhai-skills` feature）
#[cfg(feature = "rhai-skills")]
pub mod rhai_skills;

/// 命令技能（需要 `rhai-skills` feature）
#[cfg(feature = "rhai-skills")]
pub mod command_skills;

/// 文件操作技能
pub mod file_skills;

/// 技能注册表
pub mod registry;

// 重新导出 Rhai 相关类型（需要 `rhai-skills` feature）
#[cfg(feature = "rhai-skills")]
pub use rhai_skills::*;

#[cfg(feature = "rhai-skills")]
pub use command_skills::*;

// 重新导出 registry
pub use registry::*;

// 重新导出 file_skills
pub use file_skills::*;

/// Rhai 引擎类型（需要 `rhai-skills` feature）
#[cfg(feature = "rhai-skills")]
pub use rhai::{Engine, Scope};

/// Rhai 工具调用器类型（需要 `rhai-skills` feature）
#[cfg(feature = "rhai-skills")]
pub use rhai_skills::RhaiToolInvoker;

/// Rhai 引擎注册器类型（需要 `rhai-skills` feature）
#[cfg(feature = "rhai-skills")]
pub use rhai_skills::RhaiEngineRegistrar;

/// 从目录加载 skills（带 Rhai 注册器，需要 `rhai-skills` feature）
#[cfg(feature = "rhai-skills")]
pub use registry::load_skills_from_dir_with_rhai;

/// 测试工具包（仅在测试时启用）
#[cfg(test)]
pub mod testkit;
