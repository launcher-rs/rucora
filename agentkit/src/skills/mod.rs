//! Skills（技能）模块
//!
//! # 概述
//!
//! 本模块提供 Skills 的加载、执行和与 Agent 的集成功能。

pub mod loader;
pub mod integrator;
pub mod tool_adapter;

pub use loader::{SkillLoader, SkillExecutor, SkillImplementation};
pub use integrator::SkillsAutoIntegrator;
pub use tool_adapter::{SkillTool, skills_to_tools};
pub use agentkit_core::skill::{SkillDefinition, SkillResult, SkillContext};
