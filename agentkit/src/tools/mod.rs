//! Tools（工具）相关实现。
//!
//! 本模块包含各种工具的具体实现，工具是技能的基础构建块。
//! 每个工具都实现了 Tool trait，提供特定的功能。
//!
//! 设计原则（参考 zeroclaw）：
//! - 每个工具专注于单一职责，功能明确
//! - 提供清晰的输入输出定义（JSON Schema）
//! - 支持异步执行，避免阻塞
//! - 统一的错误处理机制
//! - 内置安全限制（超时、输出大小限制等）
//!
//! 工具按功能分类组织：
//!
//! **基础工具** (ToolCategory::Basic)
//! - EchoTool: 回显工具，用于测试和调试
//!
//! **系统工具** (ToolCategory::System)
//! - ShellTool: 执行系统命令
//! - GitTool: Git 操作
//!
//! **文件工具** (ToolCategory::File)
//! - FileReadTool: 读取文件内容
//! - FileWriteTool: 写入文件内容
//! - FileEditTool: 精确编辑文件内容
//!
//! **网络工具** (ToolCategory::Network)
//! - HttpRequestTool: 发送 HTTP 请求
//! - WebFetchTool: 获取网页 HTML 内容
//!
//! **浏览器工具** (ToolCategory::Browser)
//! - BrowserOpenTool: 在系统浏览器打开 URL
//!
//! **记忆工具** (ToolCategory::Memory)
//! - MemoryStoreTool: 存储信息到长期记忆
//! - MemoryRecallTool: 从记忆检索信息
//!
//! 使用示例：
//! ```rust
//! use agentkit::tools::EchoTool;
//! use agentkit::tools::ShellTool;
//! use agentkit_core::tool::{Tool, ToolCategory};
//!
//! let tool = EchoTool;
//! assert_eq!(tool.category(), ToolCategory::Basic);
//! let result = tool.call(json!({"text": "hello"})).await?;
//! ```

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
// 浏览器工具模块
pub mod browser;
// 记忆工具模块
pub mod memory;

// 按分类重新导出
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
