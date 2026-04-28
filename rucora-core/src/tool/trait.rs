//! Tool（工具）接口定义
//!
//! # 概述
//!
//! 本模块定义了 [`Tool`] trait，这是所有工具必须实现的接口。
//!
//! # 设计原则
//!
//! - **统一输入输出**: 所有工具使用 `serde_json::Value` 作为输入输出类型
//! - **自描述**: 每个工具提供自己的名称、描述和输入 schema
//! - **分类管理**: 支持工具分类，便于按类别加载和过滤
//! - **异步执行**: 所有工具异步执行，支持 IO 密集型操作
//!
//! # 实现示例
//!
//! ## 简单工具
//!
//! ```rust
//! use rucora_core::tool::{Tool, ToolCategory};
//! use rucora_core::error::ToolError;
//! use async_trait::async_trait;
//! use serde_json::{Value, json};
//!
//! /// 简单的回显工具
//! struct EchoTool;
//!
//! #[async_trait]
//! impl Tool for EchoTool {
//!     fn name(&self) -> &str {
//!         "echo"
//!     }
//!
//!     fn description(&self) -> Option<&str> {
//!         Some("回显输入内容")
//!     }
//!
//!     fn categories(&self) -> &'static [ToolCategory] {
//!         &[ToolCategory::Basic]
//!     }
//!
//!     fn input_schema(&self) -> Value {
//!         json!({
//!             "type": "object",
//!             "properties": {
//!                 "text": {
//!                     "type": "string",
//!                     "description": "要回显的文本"
//!                 }
//!             },
//!             "required": ["text"]
//!         })
//!     }
//!
//!     async fn call(&self, input: Value) -> Result<Value, ToolError> {
//!         let text = input.get("text")
//!             .and_then(|v| v.as_str())
//!             .ok_or_else(|| ToolError::Message("缺少 'text' 字段".to_string()))?;
//!         
//!         Ok(json!({"echo": text}))
//!     }
//! }
//! ```
//!
//! ## 带状态的工具
//!
//! ```rust
//! use std::sync::Arc;
//! use tokio::sync::RwLock;
//! use rucora_core::tool::{Tool, ToolCategory};
//! use rucora_core::error::ToolError;
//! use async_trait::async_trait;
//! use serde_json::{Value, json};
//!
//! /// 带状态的计数器工具
//! struct CounterTool {
//!     count: Arc<RwLock<i32>>,
//! }
//!
//! impl CounterTool {
//!     fn new() -> Self {
//!         Self {
//!             count: Arc::new(RwLock::new(0)),
//!         }
//!     }
//! }
//!
//! #[async_trait]
//! impl Tool for CounterTool {
//!     fn name(&self) -> &str {
//!         "counter"
//!     }
//!
//!     fn description(&self) -> Option<&str> {
//!         Some("计数器工具，每次调用递增")
//!     }
//!
//!     fn categories(&self) -> &'static [ToolCategory] {
//!         &[ToolCategory::Basic]
//!     }
//!
//!     fn input_schema(&self) -> Value {
//!         json!({
//!             "type": "object",
//!             "properties": {
//!                 "increment": {
//!                     "type": "integer",
//!                     "description": "递增的值，默认为 1"
//!                 }
//!             }
//!         })
//!     }
//!
//!     async fn call(&self, input: Value) -> Result<Value, ToolError> {
//!         let increment = input.get("increment")
//!             .and_then(|v| v.as_i64())
//!             .unwrap_or(1) as i32;
//!         
//!         let mut count = self.count.write().await;
//!         *count += increment;
//!         
//!         Ok(json!({"count": *count}))
//!     }
//! }
//! ```
//!
//! # 最佳实践
//!
//! ## 1. 提供清晰的描述
//!
//! ```rust
//! fn description(&self) -> Option<&str> {
//!     Some("读取文件内容。支持 txt、md、json 等文本文件格式。")
//! }
//! ```
//!
//! ## 2. 定义明确的输入 schema
//!
//! ```rust
//! fn input_schema(&self) -> Value {
//!     json!({
//!         "type": "object",
//!         "properties": {
//!             "path": {
//!                 "type": "string",
//!                 "description": "文件路径"
//!             },
//!             "max_size": {
//!                 "type": "integer",
//!                 "description": "最大文件大小（字节），默认 1MB",
//!                 "default": 1048576
//!             }
//!         },
//!         "required": ["path"]
//!     })
//! }
//! ```
//!
//! ## 3. 完善的错误处理
//!
//! ```rust
//! async fn call(&self, input: Value) -> Result<Value, ToolError> {
//!     let path = input.get("path")
//!         .and_then(|v| v.as_str())
//!         .ok_or_else(|| ToolError::Message("缺少必需的 'path' 字段".to_string()))?;
//!     
//!     // 执行操作...
//!     Ok(json!({"result": "success"}))
//! }
//! ```
//!
//! ## 4. 合理的分类
//!
//! ```rust
//! fn categories(&self) -> &'static [ToolCategory] {
//!     &[ToolCategory::File, ToolCategory::System]
//! }
//! ```

use async_trait::async_trait;
use serde_json::Value;

use crate::error::ToolError;
use crate::tool::types::ToolDefinition; // 引入 ToolDefinition

/// 工具分类枚举
///
/// 用于对工具进行分类，以便按类别加载和管理工具。
///
/// # 变体说明
///
/// - `Basic`: 基础工具，用于测试、调试等通用功能
/// - `File`: 文件操作，读取、写入、编辑文件
/// - `Network`: 网络请求，HTTP、网页获取等网络操作
/// - `System`: 系统命令，执行 shell 命令、Git 操作等
/// - `Browser`: 浏览器操作，打开浏览器、网页自动化等
/// - `Memory`: 记忆存储，存储和检索长期记忆
/// - `External`: 外部服务，与第三方 API 交互
/// - `Custom(&'static str)`: 自定义分类
///
/// # 示例
///
/// ```rust
/// use rucora_core::tool::ToolCategory;
///
/// // 使用预定义分类
/// let category = ToolCategory::File;
/// assert_eq!(category.name(), "file");
///
/// // 使用自定义分类
/// let custom = ToolCategory::Custom("ai_tool");
/// assert_eq!(custom.name(), "ai_tool");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCategory {
    /// 基础工具：用于测试、调试等通用功能
    Basic,
    /// 文件操作：读取、写入、编辑文件
    File,
    /// 网络请求：HTTP、网页获取等网络操作
    Network,
    /// 系统命令：执行 shell 命令、Git 操作等
    System,
    /// 浏览器操作：打开浏览器、网页自动化等
    Browser,
    /// 记忆存储：存储和检索长期记忆
    Memory,
    /// 外部服务：与第三方 API 交互
    External,
    /// 自定义工具
    Custom(&'static str),
}

impl ToolCategory {
    /// 返回分类名称
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rucora_core::tool::ToolCategory;
    ///
    /// assert_eq!(ToolCategory::Basic.name(), "basic");
    /// assert_eq!(ToolCategory::File.name(), "file");
    /// assert_eq!(ToolCategory::Custom("my_cat").name(), "my_cat");
    /// ```
    pub fn name(&self) -> String {
        match self {
            ToolCategory::Basic => "basic".to_string(),
            ToolCategory::File => "file".to_string(),
            ToolCategory::Network => "network".to_string(),
            ToolCategory::System => "system".to_string(),
            ToolCategory::Browser => "browser".to_string(),
            ToolCategory::Memory => "memory".to_string(),
            ToolCategory::External => "external".to_string(),
            ToolCategory::Custom(s) => s.to_string(),
        }
    }
}

/// Tool（工具）接口
///
/// 所有工具必须实现此 trait。
///
/// # 设计要求
///
/// - **输入输出统一使用 JSON**: 便于跨 provider、跨 runtime 复用
/// - **自描述**: 提供名称、描述和输入 schema
/// - **异步执行**: 支持 IO 密集型操作
/// - **线程安全**: 实现 `Send + Sync`
///
/// # 字段说明
///
/// - `name()`: 工具名称，必须唯一
/// - `description()`: 工具描述，帮助 LLM 理解工具用途
/// - `categories()`: 工具分类，支持多标签
/// - `input_schema()`: 输入参数的 JSON Schema
/// - `call()`: 执行工具的异步方法
///
/// # 示例
///
/// ```rust,no_run
/// use rucora_core::tool::{Tool, ToolCategory};
/// use rucora_core::error::ToolError;
/// use async_trait::async_trait;
/// use serde_json::{Value, json};
///
/// struct MyTool;
///
/// #[async_trait]
/// impl Tool for MyTool {
///     fn name(&self) -> &str {
///         "my_tool"
///     }
///
///     fn description(&self) -> Option<&str> {
///         Some("我的自定义工具")
///     }
///
///     fn categories(&self) -> &'static [ToolCategory] {
///         &[ToolCategory::Basic]
///     }
///
///     fn input_schema(&self) -> Value {
///         json!({
///             "type": "object",
///             "properties": {
///                 "param": {"type": "string"}
///             }
///         })
///     }
///
///     async fn call(&self, input: Value) -> Result<Value, ToolError> {
///         // 实现工具逻辑
///         Ok(json!({"result": "success"}))
///     }
/// }
/// ```
#[async_trait]
pub trait Tool: Send + Sync {
    /// 工具名称（必须唯一）
    ///
    /// # 说明
    ///
    /// 工具名称用于在运行时查找和调用工具。
    /// 名称应该：
    /// - 使用小写字母和下划线
    /// - 具有描述性
    /// - 避免与其他工具冲突
    ///
    /// # 示例
    ///
    /// ```rust
    /// // 好的命名
    /// "file_read"
    /// "http_request"
    /// "memory_store"
    ///
    /// // 避免的命名
    /// "tool1"  // 不具描述性
    /// "MyTool" // 包含大写字母
    /// ```
    fn name(&self) -> &str;

    /// 工具描述（可选）
    ///
    /// # 说明
    ///
    /// 工具描述帮助 LLM 理解工具的用途和使用场景。
    /// 描述应该：
    /// - 简洁明了
    /// - 说明工具的功能
    /// - 包含使用示例（可选）
    ///
    /// # 示例
    ///
    /// ```rust
    /// fn description(&self) -> Option<&str> {
    ///     Some("读取文件内容。支持 txt、md、json 等文本格式。")
    /// }
    /// ```
    fn description(&self) -> Option<&str> {
        None
    }

    /// 工具分类
    ///
    /// # 说明
    ///
    /// 返回工具所属的所有分类，支持多标签分类。
    /// 调用方可根据分类进行工具筛选、加载或禁用。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rucora_core::tool::{Tool, ToolCategory};
    ///
    /// // 单分类
    /// fn categories(&self) -> &'static [ToolCategory] {
    ///     &[ToolCategory::File]
    /// }
    ///
    /// // 多分类
    /// fn categories(&self) -> &'static [ToolCategory] {
    ///     &[ToolCategory::System, ToolCategory::File]
    /// }
    /// ```
    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::Basic]
    }

    /// 工具输入参数的 schema
    ///
    /// # 说明
    ///
    /// 上层 runtime/provider 可以基于该 schema 做 function-calling 工具注册。
    /// Schema 应该符合 JSON Schema 规范。
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
    ///             "path": {
    ///                 "type": "string",
    ///                 "description": "文件路径"
    ///             },
    ///             "encoding": {
    ///                 "type": "string",
    ///                 "description": "文件编码",
    ///                 "default": "utf-8"
    ///             }
    ///         },
    ///         "required": ["path"]
    ///     })
    /// }
    /// ```
    fn input_schema(&self) -> Value;

    /// 执行工具
    ///
    /// # 参数
    ///
    /// - `input`: 输入参数，应符合 `input_schema()` 定义的 schema
    ///
    /// # 返回值
    ///
    /// - `Ok(Value)`: 执行成功，返回结果
    /// - `Err(ToolError)`: 执行失败，返回错误
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use rucora_core::tool::{Tool, ToolCategory};
    /// use rucora_core::error::ToolError;
    /// use async_trait::async_trait;
    /// use serde_json::{Value, json};
    ///
    /// struct EchoTool;
    ///
    /// #[async_trait]
    /// impl Tool for EchoTool {
    ///     fn name(&self) -> &str { "echo" }
    ///     fn description(&self) -> Option<&str> { Some("回显输入") }
    ///     fn categories(&self) -> &'static [ToolCategory] { &[ToolCategory::Basic] }
    ///     fn input_schema(&self) -> Value { json!({"type": "object"}) }
    ///
    ///     async fn call(&self, input: Value) -> Result<Value, ToolError> {
    ///         Ok(input)
    ///     }
    /// }
    /// ```
    async fn call(&self, input: Value) -> Result<Value, ToolError>;

    /// 获取工具定义 (ToolDefinition)
    ///
    /// 该方法将工具的名称、描述和输入 Schema 聚合为一个结构体，
    /// 通常用于注册到 LLM 的 Function Calling 接口中。
    ///
    /// # 返回值
    ///
    /// - `ToolDefinition`: 包含工具元数据的结构体
    ///
    /// # 默认实现
    ///
    /// 默认实现会自动调用 `name()`, `description()`, `input_schema()` 组装。
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().map(String::from),
            input_schema: self.input_schema(),
        }
    }
}
