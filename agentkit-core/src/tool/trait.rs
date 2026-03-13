use async_trait::async_trait;
use serde_json::Value;

use crate::error::ToolError;

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

    /// 工具输入参数的 schema。
    ///
    /// 上层 runtime/provider 可以基于该 schema 做 function-calling 工具注册。
    fn input_schema(&self) -> Value;

    /// 执行工具。
    ///
    /// `input` 应当符合 `input_schema()` 的约束；返回值同样建议为 JSON object。
    async fn call(&self, input: Value) -> Result<Value, ToolError>;
}
