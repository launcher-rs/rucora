//! agentkit-skills - Skills 系统
//!
//! # 概述
//!
//! Skills（技能）是对 Tool/Provider/Memory 的组合封装，提供更高层次的抽象。
//! 每个技能都是独立的可执行单元，具有明确的输入输出。
//!
//! # 模块结构
//!
//! - **rhai_skills**: Rhai 脚本技能实现（需要 `rhai-skills` feature）
//! - **command_skills**: 基于 SKILL.md 模板的命令技能实现（需要 `rhai-skills` feature）
//! - **file_skills**: 文件操作技能实现
//! - **registry**: 技能注册表和加载逻辑
//!
//! # 特性
//!
//! - **Rhai 脚本技能**: 使用 Rhai 脚本语言定义灵活的技能
//! - **命令技能**: 基于 SKILL.md 模板的命令执行技能
//! - **文件操作技能**: 内置文件读取等常用技能
//! - **热加载**: 从目录动态加载技能
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use agentkit_skills::skills::*;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // 从 skills 目录加载
//! let skills = load_skills_from_dir("skills").await?;
//!
//! // 转换为 tools
//! let tools = skills.as_tools();
//! # Ok(())
//! # }
//! ```
//!
//! # Feature 标志
//!
//! | Feature | 说明 |
//! |---------|------|
//! | `rhai-skills` | 启用 Rhai 脚本技能支持 |
//! | `full` | 启用所有功能 |

pub use agentkit_core::skill::Skill;

/// Rhai 脚本技能模块（需要 `rhai-skills` feature）
#[cfg(feature = "rhai-skills")]
pub mod rhai_skills;

/// 命令技能模块（需要 `rhai-skills` feature）
#[cfg(feature = "rhai-skills")]
pub mod command_skills;

/// 文件操作技能模块
pub mod file_skills;

/// 技能注册表和加载模块
pub mod registry;

// 条件导出 Rhai 相关模块
#[cfg(feature = "rhai-skills")]
pub use rhai_skills::*;

#[cfg(feature = "rhai-skills")]
pub use command_skills::*;

// 始终导出 registry
pub use registry::*;

// 始终导出 file_skills
pub use file_skills::*;

/// 技能相关类型重新导出
#[cfg(feature = "rhai-skills")]
pub use rhai::{Engine, Scope};
