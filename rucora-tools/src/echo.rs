//! Echo 工具模块。
//!
//! 提供简单的回显功能，用于测试和调试。

use rucora_core::{
    error::ToolError,
    tool::{Tool, ToolCategory},
};
use async_trait::async_trait;
use serde_json::{Value, json};

/// Echo 工具：原样返回输入。
///
/// 这是最简单的工具实现，用于测试和调试工具调用流程。
/// 它不会对输入进行任何处理，直接返回原始数据。
///
/// 适用场景：
/// - 测试工具调用机制
/// - 调试数据传递流程
/// - 作为其他工具实现的参考模板
///
/// 输入格式：
/// ```json
/// {
///   "text": "要回显的文本"
/// }
/// ```
pub struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    /// 返回工具名称。
    fn name(&self) -> &str {
        "echo"
    }

    /// 返回工具描述。
    fn description(&self) -> Option<&str> {
        Some("回显输入参数，用于测试和调试")
    }

    /// 返回工具分类。
    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::Basic]
    }

    /// 返回输入参数的 JSON Schema。
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "要回显的文本内容"
                }
            },
            "required": ["text"]
        })
    }

    /// 执行工具的核心逻辑。
    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        // 验证输入是否包含必需的字段
        if input.get("text").is_none() {
            return Err(ToolError::Message("缺少必需的 'text' 字段".to_string()));
        }

        Ok(input)
    }
}
