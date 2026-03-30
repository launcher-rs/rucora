//! Skills（技能）模块
//!
//! # 概述
//!
//! 本模块提供 Skills 的加载、执行和与 Agent 的集成功能。
//!
//! 参考 zeroclaw 项目的设计：
//! - 支持多种配置文件格式（TOML/YAML/JSON）
//! - 支持多种提示词注入模式（Full/Compact）
//! - 提供 read_skill 工具读取 skill 详细信息
//! - 根据 skill 模式构建不同的系统提示词

pub mod config;
pub mod loader;
pub mod integrator;
pub mod tool_adapter;
pub mod cache;

pub use config::{SkillConfig, SkillMeta, SkillToolConfig};
pub use loader::{SkillLoader, SkillExecutor, SkillImplementation};
pub use integrator::SkillsAutoIntegrator;
pub use tool_adapter::{SkillTool, skills_to_tools, skills_to_prompt_with_mode, read_skill, ReadSkillTool};
pub use cache::{SkillCache, CachedSkillLoader};
pub use agentkit_core::skill::{SkillDefinition, SkillResult, SkillContext};

/// Skills 提示词注入模式
///
/// 参考 zeroclaw 的 SkillsPromptInjectionMode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillsPromptMode {
    /// 完整模式：包含所有 skill 的详细说明和工具
    Full,
    /// 简洁模式：只包含 skill 摘要，详细信息通过 read_skill 工具获取
    Compact,
}

impl Default for SkillsPromptMode {
    fn default() -> Self {
        Self::Compact
    }
}
