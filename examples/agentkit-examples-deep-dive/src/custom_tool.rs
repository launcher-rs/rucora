//! 自定义 Tool 实现示例

use agentkit_core::error::ToolError;
use agentkit_core::tool::{Tool, ToolCategory};
use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::info;

/// 计算器工具
pub struct CalculatorTool;

#[async_trait]
impl Tool for CalculatorTool {
    fn name(&self) -> &str {
        "calculator"
    }

    fn description(&self) -> Option<&str> {
        Some("执行数学计算，支持加减乘除")
    }

    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::Basic]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "description": "操作类型：add, sub, mul, div",
                    "enum": ["add", "sub", "mul", "div"]
                },
                "a": {
                    "type": "number",
                    "description": "第一个操作数"
                },
                "b": {
                    "type": "number",
                    "description": "第二个操作数"
                }
            },
            "required": ["operation", "a", "b"]
        })
    }

    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let operation = input
            .get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少 operation 参数".to_string()))?;

        let a = input
            .get("a")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Message("缺少 a 参数".to_string()))?;

        let b = input
            .get("b")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Message("缺少 b 参数".to_string()))?;

        let result = match operation {
            "add" => a + b,
            "sub" => a - b,
            "mul" => a * b,
            "div" => {
                if b == 0.0 {
                    return Err(ToolError::Message("除数不能为零".to_string()));
                }
                a / b
            }
            _ => {
                return Err(ToolError::Message(format!("未知操作：{}", operation)));
            }
        };

        Ok(json!({
            "operation": operation,
            "a": a,
            "b": b,
            "result": result
        }))
    }
}

/// 运行示例
pub async fn run() -> anyhow::Result<()> {
    info!("\n=== 自定义 Tool 示例 ===");

    let calc = CalculatorTool;
    info!("工具名称：{}", calc.name());
    info!("工具描述：{:?}", calc.description());

    // 测试加法
    let result = calc
        .call(json!({
            "operation": "add",
            "a": 10,
            "b": 5
        }))
        .await?;
    info!("✓ 10 + 5 = {}", result["result"]);

    // 测试乘法
    let result = calc
        .call(json!({
            "operation": "mul",
            "a": 6,
            "b": 7
        }))
        .await?;
    info!("✓ 6 * 7 = {}", result["result"]);

    Ok(())
}
