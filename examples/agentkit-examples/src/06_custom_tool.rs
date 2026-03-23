//! 自定义工具示例
//!
//! 展示如何实现和注册自定义工具
//!
//! # 运行方式
//!
//! ```bash
//! cargo run --bin custom-tool
//! ```

mod utils;

use agentkit::prelude::*;
use agentkit::runtime::{DefaultRuntime, ToolRegistry};
use agentkit_core::error::ToolError;
use agentkit_core::tool::{Tool, ToolCategory};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use utils::MockProvider;

/// 自定义计算器工具
struct CalculatorTool;

#[async_trait]
impl Tool for CalculatorTool {
    fn name(&self) -> &str {
        "calculator"
    }

    fn description(&self) -> Option<&str> {
        Some("执行简单的数学计算，支持加减乘除")
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
            .ok_or_else(|| ToolError::Message("缺少 operation 字段".to_string()))?;

        let a = input
            .get("a")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Message("缺少 a 字段".to_string()))?;

        let b = input
            .get("b")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Message("缺少 b 字段".to_string()))?;

        let result = match operation {
            "add" => a + b,
            "sub" => a - b,
            "mul" => a * b,
            "div" => {
                if b == 0.0 {
                    return Err(ToolError::Message("除数不能为 0".to_string()));
                }
                a / b
            }
            _ => {
                return Err(ToolError::Message(format!("不支持的操作：{}", operation)));
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("=== 自定义工具示例 ===\n");

    // 测试自定义工具
    info!("1. 测试 CalculatorTool：\n");

    let calculator = CalculatorTool;

    let test_cases = vec![
        ("加法", json!({"operation": "add", "a": 10, "b": 5})),
        ("减法", json!({"operation": "sub", "a": 10, "b": 5})),
        ("乘法", json!({"operation": "mul", "a": 10, "b": 5})),
        ("除法", json!({"operation": "div", "a": 10, "b": 5})),
    ];

    for (name, input) in test_cases {
        let result = calculator.call(input).await?;
        info!("   {}: {:?}", name, result);
    }

    // 注册工具到运行时
    info!("\n2. 注册工具到运行时：");

    let provider = MockProvider::new();
    let tools = ToolRegistry::new().register(CalculatorTool);

    let runtime = DefaultRuntime::new(Arc::new(provider), tools)
        .with_system_prompt("你是一个数学助手，可以帮助用户进行计算");

    info!("   ✓ 运行时创建成功");

    // 测试工具调用
    info!("\n3. 测试工具调用：");
    let input = AgentInput::new("请计算 10 + 5");

    match runtime.run(input).await {
        Ok(output) => {
            if let Some(content) = output.text() {
                info!("   ✓ 回复：{}", content);
            }
        }
        Err(e) => {
            info!("   ❌ 错误：{}", e);
        }
    }

    info!("\n=== 示例完成 ===");

    Ok(())
}
