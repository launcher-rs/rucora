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
//!
//! # 核心组件
//!
//! ## SkillLoader（技能加载器）
//!
//! 用于从目录加载技能定义：
//!
//! ```rust,no_run
//! use agentkit_skills::SkillLoader;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut loader = SkillLoader::new("skills/");
//! let skills = loader.load_from_dir().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## SkillExecutor（技能执行器）
//!
//! 用于执行技能：
//!
//! ```rust,no_run
//! use agentkit_skills::{SkillExecutor, SkillContext};
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let executor = Arc::new(SkillExecutor::new());
//! let context = SkillContext::new();
//! // 执行技能...
//! # Ok(())
//! # }
//! ```
//!
//! ## SkillTool（技能工具）
//!
//! 将技能转换为 Agent 可使用的工具：
//!
//! ```rust,no_run
//! use agentkit_skills::{SkillTool, SkillExecutor, SkillDefinition};
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let executor = Arc::new(SkillExecutor::new());
//! let skill = SkillDefinition {
//!     name: "my_skill".to_string(),
//!     description: "My custom skill".to_string(),
//!     ..Default::default()
//! };
//! let tool = SkillTool::new(skill, executor, "skills/");
//! # Ok(())
//! # }
//! ```
//!
//! ## SkillsAutoIntegrator（技能自动集成器）
//!
//! 自动将技能集成到 Agent 中：
//!
//! ```rust,no_run
//! use agentkit_skills::SkillsAutoIntegrator;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let integrator = SkillsAutoIntegrator::new();
//! // 集成技能...
//! # Ok(())
//! # }
//! ```
//!
//! # 子模块
//!
//! - [`cache`][]: 技能缓存，用于缓存加载的技能定义
//! - [`config`][]: 技能配置解析，支持 TOML/YAML/JSON 格式
//! - [`integrator`][]: 技能自动集成器
//! - [`loader`][]: 技能加载器和执行器
//! - [`tool_adapter`][]: 技能到工具的适配器

pub mod cache;
pub mod config;
pub mod integrator;
pub mod loader;
pub mod tool_adapter;

pub use agentkit_core::skill::{SkillContext, SkillDefinition, SkillResult};
pub use cache::{CachedSkillLoader, SkillCache};
pub use config::{SkillConfig, SkillMeta};
pub use integrator::SkillsAutoIntegrator;
pub use loader::{SkillExecutor, SkillImplementation, SkillLoader};
pub use tool_adapter::{
    ReadSkillTool, SkillTool, read_skill, skills_to_prompt_with_mode, skills_to_tools,
};

/// Skills 提示词注入模式
///
/// 参考 zeroclaw 的 SkillsPromptInjectionMode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum SkillsPromptMode {
    /// 完整模式：包含所有 skill 的详细说明和工具
    Full,
    /// 简洁模式：只包含 skill 摘要，详细信息通过 read_skill 工具获取
    #[default]
    Compact,
}


