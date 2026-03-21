//! Tools（工具）实现模块
//!
//! # 概述
//!
//! 本模块包含 12+ 种工具的具体实现。工具是技能的基础构建块，每个工具都：
//! - 实现了 [`agentkit_core::tool::Tool`] trait
//! - 提供清晰的输入输出定义（JSON Schema）
//! - 支持异步执行
//! - 统一的错误处理
//! - 内置安全限制
//!
//! # 工具列表
//!
//! ## 基础工具 (ToolCategory::Basic)
//!
//! | 工具 | 说明 | 示例 |
//! |------|------|------|
//! | [`EchoTool`] | 回显工具，用于测试和调试 | `{"text": "hello"}` |
//!
//! ## 系统工具 (ToolCategory::System)
//!
//! | 工具 | 说明 | 安全限制 |
//! |------|------|----------|
//! | [`ShellTool`] | 执行系统命令 | 禁止危险操作符 |
//! | [`CmdExecTool`] | 受限命令执行 | 仅允许 curl |
//! | [`GitTool`] | Git 操作 | 命令白名单、参数检查 |
//!
//! ## 文件工具 (ToolCategory::File)
//!
//! | 工具 | 说明 | 安全限制 |
//! |------|------|----------|
//! | [`FileReadTool`] | 读取文件内容 | 扩展名白名单、路径限制 |
//! | [`FileWriteTool`] | 写入文件内容 | 扩展名白名单、路径限制 |
//! | [`FileEditTool`] | 精确编辑文件 | 扩展名白名单、路径限制 |
//!
//! ## 网络工具 (ToolCategory::Network)
//!
//! | 工具 | 说明 | 安全限制 |
//! |------|------|----------|
//! | [`HttpRequestTool`] | 发送 HTTP 请求 | 禁止内网、域名过滤 |
//! | [`WebFetchTool`] | 获取网页 HTML | 禁止内网、超时限制 |
//!
//! ## 浏览器工具 (ToolCategory::Browser)
//!
//! | 工具 | 说明 |
//! |------|------|
//! | [`BrowseTool`] | 浏览网页（带 HTML 解析） |
//! | [`BrowserOpenTool`] | 在系统浏览器打开 URL |
//!
//! ## 记忆工具 (ToolCategory::Memory)
//!
//! | 工具 | 说明 |
//! |------|------|
//! | [`MemoryStoreTool`] | 存储信息到长期记忆 |
//! | [`MemoryRecallTool`] | 从记忆检索信息 |
//!
//! # 使用示例
//!
//! ## 基本使用
//!
//! ```rust,no_run
//! use agentkit::tools::EchoTool;
//! use agentkit_core::tool::Tool;
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let tool = EchoTool;
//! let result = tool.call(json!({"text": "hello"})).await?;
//! println!("结果：{}", result);
//! # Ok(())
//! # }
//! ```
//!
//! ## 文件操作
//!
//! ```rust,no_run
//! use agentkit::tools::{FileReadTool, FileWriteTool};
//! use agentkit_core::tool::Tool;
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // 写入文件
//! let write_tool = FileWriteTool::new();
//! write_tool.call(json!({
//!     "path": "test.txt",
//!     "content": "Hello, World!"
//! })).await?;
//!
//! // 读取文件
//! let read_tool = FileReadTool::new();
//! let result = read_tool.call(json!({
//!     "path": "test.txt"
//! })).await?;
//! println!("内容：{}", result["content"]);
//! # Ok(())
//! # }
//! ```
//!
//! ## 网络请求
//!
//! ```rust,no_run
//! use agentkit::tools::HttpRequestTool;
//! use agentkit_core::tool::Tool;
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let tool = HttpRequestTool::new();
//! let result = tool.call(json!({
//!     "method": "GET",
//!     "url": "https://api.example.com/data"
//! })).await?;
//! println!("响应：{}", result["body"]);
//! # Ok(())
//! # }
//! ```
//!
//! ## Git 操作
//!
//! ```rust,no_run
//! use agentkit::tools::GitTool;
//! use agentkit_core::tool::Tool;
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let tool = GitTool::new();
//! let result = tool.call(json!({
//!     "command": "status",
//!     "args": ["--short"]
//! })).await?;
//! println!("Git 状态：{}", result["stdout"]);
//! # Ok(())
//! # }
//! ```
//!
//! # 安全限制
//!
//! ## 文件工具
//!
//! - **扩展名白名单**: 仅允许 txt, md, rs, py, js, json, yaml 等
//! - **路径限制**: 可配置允许的工作目录
//! - **大小限制**: 默认 1MB
//! - **系统路径禁止**: 禁止访问 /etc/, C:\Windows\ 等
//!
//! ## 网络工具
//!
//! - **禁止内网**: 禁止访问 10.x.x.x, 192.168.x.x, localhost 等
//! - **域名过滤**: 支持白名单/黑名单
//! - **超时限制**: 默认 30 秒
//! - **响应大小限制**: 默认 5MB
//!
//! ## 命令工具
//!
//! - **命令白名单**: CmdExecTool 仅允许 curl
//! - **参数检查**: 禁止管道、重定向、命令注入
//! - **Git 参数检查**: 禁止 --exec、--pager 等危险参数
//!
//! # 创建工具注册表
//!
//! ```rust
//! use agentkit::tools::*;
//! use agentkit_runtime::ToolRegistry;
//!
//! let tools = ToolRegistry::new()
//!     .register(ShellTool::new())
//!     .register(GitTool::new())
//!     .register(FileReadTool::new())
//!     .register(FileWriteTool::new())
//!     .register(HttpRequestTool::new())
//!     .register(WebFetchTool::new())
//!     .register(MemoryStoreTool::new())
//!     .register(MemoryRecallTool::new());
//! ```
//!
//! # 子模块
//!
//! - [`echo`]: 回显工具
//! - [`cmd_exec`]: 受限命令执行
//! - [`git`]: Git 操作
//! - [`shell`]: Shell 命令执行
//! - [`file`]: 文件操作（读/写/编辑）
//! - [`http`]: HTTP 请求
//! - [`web`]: 网页获取
//! - [`browse`]: 网页浏览
//! - [`browser`]: 浏览器操作
//! - [`memory`]: 记忆存储

// 基础工具模块
pub mod echo;

// 系统工具模块
pub mod cmd_exec;
pub mod git;
pub mod shell;

// 文件工具模块
pub mod file;

// 网络工具模块
pub mod http;
pub mod web;

// 浏览器工具模块
pub mod browse;
pub mod browser;

// 记忆工具模块
pub mod memory;

// 重新导出所有工具

// 基础工具
pub use echo::EchoTool;

// 系统工具
pub use cmd_exec::CmdExecTool;
pub use git::GitTool;
pub use shell::ShellTool;

// 文件工具
pub use file::{FileEditTool, FileReadTool, FileWriteTool};

// 网络工具
pub use http::HttpRequestTool;
pub use web::WebFetchTool;

// 浏览器工具
pub use browse::BrowseTool;
pub use browser::BrowserOpenTool;

// 记忆工具
pub use memory::{MemoryRecallTool, MemoryStoreTool};
