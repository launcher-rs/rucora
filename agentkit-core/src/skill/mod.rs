//! Skill（技能）抽象模块
//!
//! # 概述
//!
//! Skill（技能）是对 Tool/Provider/Memory 的组合封装，提供更高层次的抽象。
//! 与 Tool 相比，Skill 通常更复杂，可能会组合多个 Tool/Provider/Memory 来完成一个完整的任务流程。
//!
//! # Skill 与 Tool 的区别
//!
//! | 特性 | Tool | Skill |
//! |------|------|-------|
//! | 粒度 | 细粒度（单一功能） | 粗粒度（完整任务） |
//! | 组合 | 独立执行 | 可组合多个 Tool/Provider |
//! | 输入输出 | JSON Value | JSON Value |
//! | 示例 | 读取文件、HTTP 请求 | 天气查询、新闻摘要 |
//!
//! # 核心类型
//!
//! ## Skill trait
//!
//! [`Skill`] trait 是所有技能必须实现的接口：
//!
//! ```rust,no_run
//! use agentkit_core::skill::Skill;
//! use agentkit_core::skill::types::{SkillContext, SkillOutput};
//! use agentkit_core::error::SkillError;
//! use async_trait::async_trait;
//!
//! struct WeatherSkill;
//!
//! #[async_trait]
//! impl Skill for WeatherSkill {
//!     fn name(&self) -> &str {
//!         "weather"
//!     }
//!
//!     fn description(&self) -> Option<&str> {
//!         Some("查询天气信息")
//!     }
//!
//!     async fn run_value(&self, input: serde_json::Value) -> Result<serde_json::Value, SkillError> {
//!         // 实现技能逻辑
//!         unimplemented!()
//!     }
//! }
//! ```
//!
//! ## SkillContext
//!
//! 技能执行的上下文，包含：
//! - `user_input`: 用户原始输入
//! - `input`: 技能的输入参数
//!
//! ## SkillOutput
//!
//! 技能执行的输出，包含：
//! - `output`: 技能的输出结果
//! - `metadata`: 可选的元数据
//!
//! # 使用示例
//!
//! ## 实现天气查询技能
//!
//! ```rust,no_run
//! use agentkit_core::skill::{Skill, SkillContext, SkillOutput};
//! use agentkit_core::error::SkillError;
//! use async_trait::async_trait;
//! use serde_json::{Value, json};
//!
//! struct WeatherSkill;
//!
//! #[async_trait]
//! impl Skill for WeatherSkill {
//!     fn name(&self) -> &str {
//!         "weather"
//!     }
//!
//!     fn description(&self) -> Option<&str> {
//!         Some("查询指定城市的天气信息")
//!     }
//!
//!     async fn run_value(&self, input: Value) -> Result<Value, SkillError> {
//!         let location = input.get("location")
//!             .and_then(|v| v.as_str())
//!             .ok_or_else(|| SkillError::Message("缺少 location 参数".to_string()))?;
//!
//!         // 调用 HTTP 工具获取天气数据
//!         // ...
//!
//!         Ok(json!({
//!             "location": location,
//!             "temperature": 25,
//!             "condition": "晴朗"
//!         }))
//!     }
//! }
//! ```
//!
//! ## 实现新闻摘要技能
//!
//! ```rust,no_run
//! use agentkit_core::skill::{Skill, SkillContext, SkillOutput};
//! use agentkit_core::error::SkillError;
//! use async_trait::async_trait;
//! use serde_json::{Value, json};
//!
//! struct NewsSummarySkill;
//!
//! #[async_trait]
//! impl Skill for NewsSummarySkill {
//!     fn name(&self) -> &str {
//!         "news_summary"
//!     }
//!
//!     fn description(&self) -> Option<&str> {
//!         Some("获取并摘要新闻")
//!     }
//!
//!     async fn run_value(&self, input: Value) -> Result<Value, SkillError> {
//!         // 1. 获取新闻
//!         // 2. 调用 LLM 摘要
//!         // 3. 返回结果
//!         unimplemented!()
//!     }
//! }
//! ```
//!
//! # 技能的生命周期
//!
//! ```text
//! 1. 接收 SkillContext
//!    │
//!    ▼
//! 2. 解析输入参数
//!    │
//!    ▼
//! 3. 执行技能逻辑（可能调用多个 Tool/Provider）
//!    │
//!    ▼
//! 4. 返回 SkillOutput
//! ```
//!
//! # 最佳实践
//!
//! ## 1. 提供清晰的描述
//!
//! ```rust
//! fn description(&self) -> Option<&str> {
//!     Some("查询指定城市的天气信息，支持当前天气和 3 天预报")
//! }
//! ```
//!
//! ## 2. 完善的错误处理
//!
//! ```rust
//! async fn run_value(&self, input: Value) -> Result<Value, SkillError> {
//!     let location = input.get("location")
//!         .and_then(|v| v.as_str())
//!         .ok_or_else(|| SkillError::Message("缺少必需的 'location' 参数".to_string()))?;
//!     
//!     // 执行逻辑...
//!     Ok(json!({"result": "success"}))
//! }
//! ```
//!
//! ## 3. 合理的输入输出格式
//!
//! ```rust
//! // 输入
//! json!({
//!     "location": "Beijing",
//!     "days": 3
//! })
//!
//! // 输出
//! json!({
//!     "location": "Beijing",
//!     "forecast": [
//!         {"day": "今天", "temp": 25, "condition": "晴朗"},
//!         {"day": "明天", "temp": 23, "condition": "多云"},
//!         {"day": "后天", "temp": 20, "condition": "小雨"}
//!     ]
//! })
//! ```

pub mod skill_trait;
pub mod types;

/// 重新导出 skill 相关 trait，方便使用
pub use skill_trait::*;

/// 重新导出 skill 相关类型，方便使用
pub use types::*;
