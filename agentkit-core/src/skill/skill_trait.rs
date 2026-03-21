//! Skill（技能）trait 定义
//!
//! # 概述
//!
//! [`Skill`] trait 定义了技能的标准接口。
//! 技能是对 Tool/Provider/Memory 的组合封装，提供更高层次的抽象。
//!
//! # 设计要求
//!
//! - **自描述**: 提供名称、描述和输入 schema
//! - **统一输入输出**: 使用 JSON Value 作为输入输出类型
//! - **异步执行**: 支持 IO 密集型操作
//! - **线程安全**: 实现 `Send + Sync`
//!
//! # 示例
//!
//! ## 简单技能
//!
//! ```rust
//! use agentkit_core::skill::{Skill, SkillContext, SkillOutput};
//! use agentkit_core::error::SkillError;
//! use async_trait::async_trait;
//! use serde_json::{Value, json};
//!
//! struct EchoSkill;
//!
//! #[async_trait]
//! impl Skill for EchoSkill {
//!     fn name(&self) -> &str {
//!         "echo"
//!     }
//!
//!     fn description(&self) -> Option<&str> {
//!         Some("回显输入内容")
//!     }
//!
//!     fn input_schema(&self) -> Value {
//!         json!({
//!             "type": "object",
//!             "properties": {
//!                 "text": {"type": "string", "description": "要回显的文本"}
//!             },
//!             "required": ["text"]
//!         })
//!     }
//!
//!     async fn run_value(&self, input: Value) -> Result<Value, SkillError> {
//!         Ok(input)
//!     }
//! }
//! ```
//!
//! ## 复杂技能（组合多个 Tool）
//!
//! ```rust,no_run
//! use agentkit_core::skill::Skill;
//! use agentkit_core::error::SkillError;
//! use async_trait::async_trait;
//! use serde_json::{Value, json};
//!
//! struct WeatherSummarySkill {
//!     // http_tool: Arc<HttpRequestTool>,
//!     // llm_provider: Arc<dyn LlmProvider>,
//! };
//!
//! #[async_trait]
//! impl Skill for WeatherSummarySkill {
//!     fn name(&self) -> &str {
//!         "weather_summary"
//!     }
//!
//!     fn description(&self) -> Option<&str> {
//!         Some("查询天气并生成摘要")
//!     }
//!
//!     fn input_schema(&self) -> Value {
//!         json!({
//!             "type": "object",
//!             "properties": {
//!                 "location": {"type": "string", "description": "城市名称"},
//!                 "days": {"type": "integer", "description": "预报天数", "default": 1}
//!             },
//!             "required": ["location"]
//!         })
//!     }
//!
//!     async fn run_value(&self, input: Value) -> Result<Value, SkillError> {
//!         // 1. 调用 HTTP 工具获取天气数据
//!         // 2. 调用 LLM 生成摘要
//!         // 3. 返回结果
//!         unimplemented!()
//!     }
//! }
//! ```

use async_trait::async_trait;
use serde_json::Value;

use crate::{error::SkillError, tool::ToolCategory};

/// Skill（技能）trait
///
/// 定义技能的标准接口。
///
/// # 说明
///
/// - core 层只定义抽象接口（Skill 是什么）
/// - 具体 skill 的实现与注册/编排应放在上层 crate（例如 agentkit / runtime）
///
/// # 设计要求
///
/// - **自描述**: 提供名称、描述和输入 schema
/// - **统一输入输出**: 使用 JSON Value 作为输入输出类型
/// - **异步执行**: 支持 IO 密集型操作
/// - **线程安全**: 实现 `Send + Sync`
///
/// # 字段说明
///
/// - `name()`: 技能名称（必须唯一）
/// - `description()`: 技能描述（帮助理解用途）
/// - `categories()`: 技能分类（支持多标签）
/// - `input_schema()`: 输入参数的 JSON Schema
/// - `run_value()`: 执行技能的异步方法
///
/// # 示例
///
/// ```rust,no_run
/// use agentkit_core::skill::Skill;
/// use agentkit_core::error::SkillError;
/// use async_trait::async_trait;
/// use serde_json::{Value, json};
///
/// struct MySkill;
///
/// #[async_trait]
/// impl Skill for MySkill {
///     fn name(&self) -> &str {
///         "my_skill"
///     }
///
///     fn description(&self) -> Option<&str> {
///         Some("我的自定义技能")
///     }
///
///     fn categories(&self) -> &'static [ToolCategory] {
///         &[ToolCategory::Basic]
///     }
///
///     fn input_schema(&self) -> Value {
///         json!({"type": "object"})
///     }
///
///     async fn run_value(&self, input: Value) -> Result<Value, SkillError> {
///         // 实现技能逻辑
///         Ok(json!({"result": "success"}))
///     }
/// }
/// ```
#[async_trait]
pub trait Skill: Send + Sync {
    /// 技能名称（必须唯一）
    ///
    /// # 说明
    ///
    /// 技能名称用于：
    /// - 在运行时查找和调用技能
    /// - 作为暴露给 LLM 的 tool 名称
    ///
    /// 命名约定：
    /// - 使用小写字母和下划线
    /// - 具有描述性
    /// - 避免与其他技能冲突
    ///
    /// # 示例
    ///
    /// ```rust
    /// // 好的命名
    /// "weather_query"
    /// "news_summary"
    /// "file_analysis"
    ///
    /// // 避免的命名
    /// "skill1"  // 不具描述性
    /// "MySkill" // 包含大写字母
    /// ```
    fn name(&self) -> &str;

    /// 技能描述（可选）
    ///
    /// # 说明
    ///
    /// 技能描述帮助理解技能的用途和使用场景。
    /// 描述应该：
    /// - 简洁明了
    /// - 说明技能的功能
    /// - 包含使用示例（可选）
    ///
    /// # 示例
    ///
    /// ```rust
    /// fn description(&self) -> Option<&str> {
    ///     Some("查询指定城市的天气信息，支持当前天气和 3 天预报")
    /// }
    /// ```
    fn description(&self) -> Option<&str> {
        None
    }

    /// 技能分类
    ///
    /// # 说明
    ///
    /// 返回技能所属的所有分类，支持多标签分类。
    /// 调用方可根据分类进行技能筛选、加载或禁用。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit_core::skill::Skill;
    /// use agentkit_core::tool::ToolCategory;
    ///
    /// fn categories(&self) -> &'static [ToolCategory] {
    ///     &[ToolCategory::Network, ToolCategory::System]
    /// }
    /// ```
    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::Basic]
    }

    /// 输入参数 schema
    ///
    /// # 说明
    ///
    /// 定义技能接受的输入参数的 JSON Schema。
    /// 上层 runtime 可以基于该 schema 做参数验证。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use serde_json::json;
    ///
    /// fn input_schema(&self) -> Value {
    ///     json!({
    ///         "type": "object",
    ///         "properties": {
    ///             "location": {
    ///                 "type": "string",
    ///                 "description": "城市名称"
    ///             },
    ///             "days": {
    ///                 "type": "integer",
    ///                 "description": "预报天数",
    ///                 "default": 1
    ///             }
    ///         },
    ///         "required": ["location"]
    ///     })
    /// }
    /// ```
    fn input_schema(&self) -> Value;

    /// 执行技能
    ///
    /// # 参数
    ///
    /// - `input`: 输入参数，应符合 `input_schema()` 定义的 schema
    ///
    /// # 返回值
    ///
    /// - `Ok(Value)`: 执行成功，返回结果
    /// - `Err(SkillError)`: 执行失败，返回错误
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use agentkit_core::skill::Skill;
    /// use agentkit_core::error::SkillError;
    /// use async_trait::async_trait;
    /// use serde_json::{Value, json};
    ///
    /// struct EchoSkill;
    ///
    /// #[async_trait]
    /// impl Skill for EchoSkill {
    ///     fn name(&self) -> &str { "echo" }
    ///     fn description(&self) -> Option<&str> { Some("回显输入") }
    ///     fn categories(&self) -> &'static [ToolCategory] { &[ToolCategory::Basic] }
    ///     fn input_schema(&self) -> Value { json!({"type": "object"}) }
    ///
    ///     async fn run_value(&self, input: Value) -> Result<Value, SkillError> {
    ///         Ok(input)
    ///     }
    /// }
    /// ```
    async fn run_value(&self, input: Value) -> Result<Value, SkillError>;
}
