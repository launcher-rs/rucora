//! Tool（工具）抽象模块
//!
//! # 概述
//!
//! Tool（工具）是可以被 Agent 调用的"可执行能力"，例如：
//! - 读取文件、写入文件
//! - 发送 HTTP 请求
//! - 执行系统命令
//! - 访问数据库
//! - 调用外部 API
//!
//! 在 core 层，我们只定义工具的接口与 schema，不关心具体执行实现。
//!
//! # 核心类型
//!
//! ## Tool trait
//!
//! [`Tool`] trait 是所有工具必须实现的接口：
//!
//! ```rust,no_run
//! use agentkit_core::tool::{Tool, ToolCategory};
//! use agentkit_core::error::ToolError;
//! use async_trait::async_trait;
//! use serde_json::{Value, json};
//!
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
//!                 "text": {"type": "string", "description": "要回显的文本"}
//!             },
//!             "required": ["text"]
//!         })
//!     }
//!
//!     async fn call(&self, input: Value) -> Result<Value, ToolError> {
//!         let text = input.get("text").and_then(|v| v.as_str()).unwrap_or("");
//!         Ok(json!({"echo": text}))
//!     }
//! }
//! ```
//!
//! ## ToolRegistry trait
//!
//! [`ToolRegistry`] trait 是工具注册表的接口，用于管理和调用多个工具：
//!
//! ```rust,no_run
//! use agentkit_core::tool::{Tool, ToolRegistry, ToolCategory};
//! use agentkit_core::error::ToolError;
//! use async_trait::async_trait;
//! use serde_json::{Value, json};
//! use std::sync::Arc;
//!
//! struct MyRegistry;
//!
//! #[async_trait]
//! impl ToolRegistry for MyRegistry {
//!     fn get_tool(&self, name: &str) -> Option<Arc<dyn Tool>> {
//!         None // 实现工具查找逻辑
//!     }
//!
//!     fn list_tools(&self) -> Vec<Arc<dyn Tool>> {
//!         vec![] // 实现工具列表逻辑
//!     }
//!
//!     async fn call(&self, name: &str, input: Value) -> Result<Value, ToolError> {
//!         Err(ToolError::NotFound(name.to_string()))
//!     }
//! }
//! ```
//!
//! ## ToolCategory
//!
//! 工具分类枚举，用于对工具进行分类管理：
//!
//! - [`ToolCategory::Basic`]: 基础工具（测试、调试等）
//! - [`ToolCategory::File`]: 文件操作（读取、写入、编辑）
//! - [`ToolCategory::Network`]: 网络请求（HTTP、网页获取）
//! - [`ToolCategory::System`]: 系统命令（shell、Git）
//! - [`ToolCategory::Browser`]: 浏览器操作
//! - [`ToolCategory::Memory`]: 记忆存储
//! - [`ToolCategory::External`]: 外部服务
//! - [`ToolCategory::Custom`]: 自定义分类
//!
//! ## ToolDefinition
//!
//! 工具定义，用于注册到 LLM 的 function calling：
//!
//! ```rust
//! use agentkit_core::tool::types::ToolDefinition;
//! use serde_json::json;
//!
//! let def = ToolDefinition {
//!     name: "echo".to_string(),
//!     description: Some("回显输入内容".to_string()),
//!     input_schema: json!({
//!         "type": "object",
//!         "properties": {
//!             "text": {"type": "string"}
//!         }
//!     }),
//! };
//! ```
//!
//! ## ToolCall
//!
//! 工具调用，由 LLM 生成：
//!
//! ```rust
//! use agentkit_core::tool::types::ToolCall;
//! use serde_json::json;
//!
//! let call = ToolCall {
//!     id: "call_123".to_string(),
//!     name: "echo".to_string(),
//!     input: json!({"text": "Hello"}),
//! };
//! ```
//!
//! ## ToolResult
//!
//! 工具执行结果：
//!
//! ```rust
//! use agentkit_core::tool::types::ToolResult;
//! use serde_json::json;
//!
//! let result = ToolResult {
//!     tool_call_id: "call_123".to_string(),
//!     output: json!({"echo": "Hello"}),
//! };
//! ```
//!
//! # 工具生命周期
//!
//! ```text
//! 1. LLM 决定调用工具
//!    │
//!    ▼
//! 2. 生成 ToolCall（包含工具名称和输入）
//!    │
//!    ▼
//! 3. Runtime 执行工具
//!    │
//!    ▼
//! 4. 返回 ToolResult（包含输出）
//!    │
//!    ▼
//! 5. 将结果返回给 LLM 继续推理
//! ```

/// Tool trait 定义
pub mod r#trait;

/// Tool 相关类型定义
pub mod types;

/// 工具过滤分组系统
pub mod filter;

/// 重新导出 tool 相关 trait，方便 `agentkit_core::tool::*` 使用
pub use r#trait::*;

/// 重新导出 tool 相关类型，方便使用
pub use types::*;

/// 重新导出工具过滤相关类型
pub use filter::{
    ToolFilter, ToolFilterConfig, ToolFilterStats, ToolGroup, ToolGroupManager, ToolVisibility,
};
