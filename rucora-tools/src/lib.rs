//! rucora-tools - rucora 内置工具集合
//!
//! 提供丰富的工具实现，包括文件操作、系统命令、Web 请求、搜索等功能。
//!
//! ## 模块结构
//!
//! - `file` - 文件操作工具（读、写、编辑）
//! - `system` - 系统工具（Shell、命令执行、时间）
//! - `web` - Web 工具（HTTP 请求、网页获取、搜索）
//! - `search` - 搜索工具（Glob 搜索、内容搜索）
//! - `math` - 数学工具（计算器）
//! - `media` - 媒体工具（图片信息）
//! - `git` - Git 工具
//! - `memory` - 记忆工具
//! - `echo` - 回显工具
//!
//! ## 使用示例
//!
//! ```rust,no_run
//! use rucora_tools::file::{FileReadTool, FileWriteTool};
//! use rucora_tools::system::ShellTool;
//! use rucora_tools::web::HttpRequestTool;
//!
//! // 使用工具...
//! ```

// ===== 核心模块（按功能分类）=====

/// 文件操作工具模块
///
/// 提供安全的文件读写和编辑功能
pub mod file;

/// 系统工具模块
///
/// 提供系统命令执行、时间获取等功能
pub mod system;

/// Web 工具模块
///
/// 提供网页浏览、HTTP 请求、Web 搜索等功能
pub mod web;

/// 搜索工具模块
///
/// 提供文件搜索和内容搜索功能
pub mod search;

/// 数学工具模块
///
/// 提供高级数学计算功能
pub mod math;

/// 媒体处理工具模块
///
/// 提供图片信息读取等媒体处理功能
pub mod media;

// ===== 独立工具模块 =====

/// Git 工具模块
pub mod git;

/// 记忆工具模块
pub mod memory;

/// 回显工具模块
pub mod echo;

// ===== 向后兼容：保留顶层模块 =====

/// 文件工具（向后兼容，建议使用 `file` 模块）
#[deprecated(since = "0.2.0", note = "请使用 `file` 模块代替")]
pub use file as file_legacy;

/// Shell 工具（向后兼容，建议使用 `system` 模块）
#[deprecated(since = "0.2.0", note = "请使用 `system` 模块代替")]
pub use system as system_legacy;

/// Web 工具（向后兼容，建议使用 `web` 模块）
#[deprecated(since = "0.2.0", note = "请使用 `web` 模块代替")]
pub use web as web_legacy;

// ===== 重新导出常用工具类型 =====

// 文件工具
pub use file::{FileEditTool, FileReadTool, FileToolConfig, FileWriteTool};

// 系统工具
pub use system::{CmdExecTool, DatetimeTool, ShellTool};

// Web 工具
pub use web::{
    BrowseTool, BrowserOpenTool, GithubTrendingTool, HttpRequestTool, SerpapiTool, TavilyTool,
    WebFetchTool,
};

// 搜索工具
pub use search::{ContentSearchTool, GlobSearchTool};

// 数学工具
pub use math::CalculatorTool;

// 媒体工具
pub use media::ImageInfoTool;

// 其他工具
pub use echo::EchoTool;
pub use git::GitTool;
pub use memory::{MemoryRecallTool, MemoryStoreTool};

// 重新导出 ToolRegistry
pub use rucora_core::tool::ToolRegistry;
