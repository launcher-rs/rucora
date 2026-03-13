use async_trait::async_trait;
use serde_json::Value;

use crate::error::ToolError;

/// 工具分类枚举。
///
/// 用于对工具进行分类，以便按类别加载和管理工具。
/// 每个类别代表一组功能相关的工具。
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

/// Tool（工具）接口。
///
/// - 输入输出统一使用 JSON（`serde_json::Value`），便于跨 provider、跨 runtime 复用。
/// - `input_schema()` 用于描述输入参数的 JSON Schema（或兼容的 schema 结构）。
#[async_trait]
pub trait Tool: Send + Sync {
    /// 工具名称（必须唯一）。
    fn name(&self) -> &str;

    /// 工具描述（可选）。
    fn description(&self) -> Option<&str> {
        None
    }

    /// 工具分类（可选，默认为 Basic）。
    ///
    /// 返回工具所属的所有分类，支持多标签分类。
    /// 调用方可根据分类进行工具筛选、加载或禁用。
    ///
    /// 示例：
    /// - 单分类工具：`&[ToolCategory::File]`
    /// - 多分类工具：`&[ToolCategory::System, ToolCategory::File]`
    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::Basic]
    }

    /// 工具输入参数的 schema。
    ///
    /// 上层 runtime/provider 可以基于该 schema 做 function-calling 工具注册。
    fn input_schema(&self) -> Value;

    /// 执行工具。
    ///
    /// `input` 应当符合 `input_schema()` 的约束；返回值同样建议为 JSON object。
    async fn call(&self, input: Value) -> Result<Value, ToolError>;
}
