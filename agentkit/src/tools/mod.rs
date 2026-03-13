//! Tools（工具）相关实现。
//!
//! 这里提供一些示例工具，帮助你快速验证 tool-calling loop。

use agentkit_core::{error::ToolError, tool::Tool};
use async_trait::async_trait;
use serde_json::{json, Value};

/// 一个最简单的 Echo 工具：原样返回输入。
pub struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> Option<&str> {
        Some("回显输入参数")
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "text": {"type": "string"}
            },
            "required": ["text"]
        })
    }

    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        Ok(input)
    }
}
