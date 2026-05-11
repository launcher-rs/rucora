//! Deep Research（深度研究）实现
//!
//! 提供多种研究策略的实现。

pub mod engine;
pub mod strategies;

pub use engine::DefaultResearchEngine;
pub use strategies::{FastStrategy, StandardStrategy};