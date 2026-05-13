//! Deep Research（深度研究）核心抽象
//!
//! 提供深度研究功能的核心类型和 trait 定义。
//!
//! # 模块结构
//!
//! - [`types`] - 研究相关类型定义（InfoPiece、Citation、ResearchReport 等）
//! - [`strategies`] - 核心 trait 定义（DeepResearchEngine、StrategyTrait、ResearchQualityAssessor 等）
//!
//! # 职责划分
//!
//! 本模块（`rucora-core::research`）只定义**抽象接口和核心类型**，不包含具体实现。
//!
//! 具体实现位于 `rucora` crate 的 `deep_research` 模块中：
//! - [`rucora::deep_research::strategies`] - 策略的具体实现（StandardStrategy、FastStrategy、AgenticStrategy）
//! - [`rucora::deep_research::engine`] - 默认引擎实现（DefaultResearchEngine）
//! - [`rucora::deep_research::library`] - 研究库存储实现
//!
//! 这种分离允许第三方 crate 只依赖 `rucora-core` 来实现自定义的研究引擎和策略。

pub mod types;
mod strategies;

pub use types::*;
pub use strategies::{
    CitationHandler, DeepResearchEngine, ResearchContext, ResearchError, ResearchLibrary,
    ResearchQualityAssessor, StrategyFactory, StrategyResult, StrategyTrait,
};