//! Deep Research（深度研究）核心抽象
//!
//! 提供深度研究功能的核心类型和 trait 定义。
//!
//! # 模块结构
//!
//! - [`types`] - 研究相关类型定义
//! - [`strategies`] - 核心 trait 定义

pub mod types;
mod strategies;

pub use types::*;
// 导出 strategies 中的类型和 trait
pub use strategies::{
    CitationHandler, DeepResearchEngine, ResearchContext, ResearchError, ResearchLibrary,
    ResearchQualityAssessor, StrategyFactory, StrategyResult, StrategyTrait,
};