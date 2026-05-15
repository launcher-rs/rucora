//! rucora 宏系统示例
//!
//! 展示如何使用 rucora 的宏系统快速构建 Agent、工具、消息等。
//!
//! ## 特性
//!
//! - `#[rucora_tool]` 过程宏：从 async fn 自动生成 Tool 实现
//! - `agent!` 宏：快速构建 Agent
//! - `messages!` 宏：快速构建消息列表
//! - `chat_request!` 宏：快速构建聊天请求
//! - `tool_params!` 宏：快速构建工具参数 JSON Schema
//! - `ToolRiskLevel`：工具风险等级
//! - `ToolResult` 增强：支持结构化数据和二进制数据
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! export MODEL_NAME=gpt-4o-mini
//! cargo run --example 26_macros
//! ```

use rucora::prelude::{Agent, Tool};
use rucora::provider::OpenAiProvider;
use rucora::rucora_tool;
use rucora::tool_params;
use rucora_core::error::ToolError;
use rucora_core::tool::types::ToolRiskLevel;
use serde_json::{Value, json};
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

// ────────────────────────────────────────────────────────────────────────────
// #[rucora_tool] 过程宏：从 async fn 自动生成 Tool 实现
// ────────────────────────────────────────────────────────────────────────────

/// 获取天气信息
#[rucora_tool(name = "get_weather", description = "获取指定城市的天气信息")]
async fn get_weather(city: String) -> Result<Value, ToolError> {
    Ok(json!({
        "city": city,
        "temperature": 22,
        "condition": "晴天",
        "humidity": 45
    }))
}

/// 计算器：执行简单的数学运算
#[rucora_tool(name = "calculator", description = "执行简单的数学运算（加、减、乘、除）")]
async fn calculator(a: f64, b: f64, operation: String) -> Result<Value, ToolError> {
    let result = match operation.as_str() {
        "add" | "+" => a + b,
        "sub" | "-" => a - b,
        "mul" | "*" => a * b,
        "div" | "/" => {
            if b == 0.0 {
                return Err(ToolError::Message("除数不能为零".to_string()));
            }
            a / b
        }
        _ => {
            return Err(ToolError::Message(format!(
                "不支持的运算：{operation}，支持 add/sub/mul/div"
            )));
        }
    };
    Ok(json!({
        "a": a,
        "b": b,
        "operation": operation,
        "result": result
    }))
}

/// 生成随机数（带风险等级标记）
struct RandomTool;

#[async_trait::async_trait]
impl rucora_core::tool::Tool for RandomTool {
    fn name(&self) -> &str {
        "random_number"
    }

    fn description(&self) -> Option<&str> {
        Some("生成指定范围的随机整数")
    }

    fn categories(&self) -> &'static [rucora_core::tool::ToolCategory] {
        &[rucora_core::tool::ToolCategory::Basic]
    }

    fn input_schema(&self) -> Value {
        tool_params! {
            "min" => (number, required, "最小值"),
            "max" => (number, required, "最大值"),
        }
    }

    fn risk_level(&self) -> ToolRiskLevel {
        ToolRiskLevel::Safe
    }

    async fn call(
        &self,
        input: Value,
        _context: &rucora_core::tool::types::ToolContext,
    ) -> Result<Value, ToolError> {
        let min = input["min"].as_f64().unwrap_or(0.0) as i64;
        let max = input["max"].as_f64().unwrap_or(100.0) as i64;
        // 示例使用固定值，避免引入额外依赖
        let random = (min + max) / 2;
        Ok(json!({ "result": random, "min": min, "max": max }))
    }
}

struct DangerTool;

#[async_trait::async_trait]
impl rucora_core::tool::Tool for DangerTool {
    fn name(&self) -> &str {
        "delete_all_files"
    }

    fn description(&self) -> Option<&str> {
        Some("删除所有文件（危险操作示例）")
    }

    fn categories(&self) -> &'static [rucora_core::tool::ToolCategory] {
        &[rucora_core::tool::ToolCategory::File]
    }

    fn input_schema(&self) -> Value {
        tool_params! {
            "confirm" => (boolean, required, "确认删除"),
        }
    }

    fn risk_level(&self) -> ToolRiskLevel {
        ToolRiskLevel::Dangerous
    }

    async fn call(
        &self,
        input: Value,
        _context: &rucora_core::tool::types::ToolContext,
    ) -> Result<Value, ToolError> {
        let confirm = input["confirm"].as_bool().unwrap_or(false);
        if !confirm {
            return Err(ToolError::Message("未确认删除操作".to_string()));
        }
        // 仅示例，不真正删除
        Ok(json!({ "status": "simulated", "message": "已模拟删除操作" }))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔══════════════════════════════════════════════════╗");
    info!("║   rucora 宏系统示例                          ║");
    info!("╚══════════════════════════════════════════════════╝\n");

    // ── 1. 演示 messages! 宏 ──
    info!("1. messages! 宏 - 快速构建消息列表");
    let msgs = rucora::messages![
        system("你是有用的助手"),
        user("你好"),
        assistant("你好！有什么可以帮你的？"),
        user("今天天气怎么样？"),
    ];
    info!("   构建了 {} 条消息", msgs.len());
    for (i, msg) in msgs.iter().enumerate() {
        info!("   [{i}] {:?}: {}", msg.role, msg.content);
    }
    info!("");

    // ── 2. 演示 chat_request! 宏 ──
    info!("2. chat_request! 宏 - 快速构建聊天请求");
    let req = rucora::chat_request!(
        messages: [
            system("你是翻译助手"),
            user("把 'Hello World' 翻译成中文"),
        ],
        model: "gpt-4o-mini",
        temperature: 0.3,
        max_tokens: 1024,
    );
    info!("   模型：{:?}", req.model);
    info!("   温度：{:?}", req.temperature);
    info!("   消息数：{}", req.messages.len());
    info!("");

    // ── 3. 演示 tool_params! 宏 ──
    info!("3. tool_params! 宏 - 快速构建工具参数 Schema");
    let schema = tool_params! {
        "city" => (string, required, "城市名称"),
        "unit" => (string, "温度单位，默认 celsius"),
        "include_forecast" => (boolean, "是否包含预报"),
    };
    info!("   Schema: {}", serde_json::to_string_pretty(&schema)?);
    info!("");

    // ── 4. 演示 ToolRiskLevel ──
    info!("4. ToolRiskLevel - 工具风险等级");
    let random_tool = RandomTool;
    let danger_tool = DangerTool;
    info!(
        "   RandomTool 风险等级：{:?} ({})",
        random_tool.risk_level(),
        random_tool.risk_level().as_str()
    );
    info!(
        "   DangerTool 风险等级：{:?} ({})",
        danger_tool.risk_level(),
        danger_tool.risk_level().as_str()
    );
    info!(
        "   Dangerous 工具需要审批：{}",
        danger_tool.risk_level().requires_approval()
    );
    info!("");

    // ── 5. 演示 ToolResult 增强 ──
    info!("5. ToolResult 增强 - 结构化数据与二进制数据");
    let success_result = rucora_core::tool::types::ToolResult::success(
        "call_001",
        json!({ "temperature": 22 }),
    );
    info!("   成功结果：success={}", success_result.is_success());

    let data_result = rucora_core::tool::types::ToolResult::success(
        "call_002",
        json!({ "summary": "数据已处理" }),
    )
    .with_data(json!({
        "rows": 100,
        "columns": 5,
        "stats": { "mean": 42.5, "std": 12.3 }
    }));
    info!("   带结构化数据：data={:?}", data_result.data);

    let failure_result =
        rucora_core::tool::types::ToolResult::failure("call_003", "网络连接超时");
    info!("   失败结果：success={}, error={:?}", failure_result.is_success(), failure_result.error);
    info!("");

    // ── 6. 演示 #[rucora_tool] 过程宏 ──
    info!("6. #[rucora_tool] 过程宏 - 自动生成 Tool 实现");
    info!("   已自动生成：");
    info!("   - GetWeatherTool: {}", GetWeatherTool.name());
    info!("     描述：{:?}", GetWeatherTool.description());
    info!("     Schema: {}", serde_json::to_string_pretty(&GetWeatherTool.input_schema())?);
    info!("");
    info!("   - CalculatorTool: {}", CalculatorTool.name());
    info!("     描述：{:?}", CalculatorTool.description());
    info!("     Schema: {}", serde_json::to_string_pretty(&CalculatorTool.input_schema())?);
    info!("");

    // ── 7. 演示 agent! 宏 ──
    info!("7. agent! 宏 - 快速构建 Agent");

    // 检查配置
    if std::env::var("OPENAI_API_KEY").is_err() && std::env::var("OPENAI_BASE_URL").is_err() {
        info!("⚠ 未设置 API 配置，跳过 agent! 宏的实际运行");
        info!("   使用 OpenAI: export OPENAI_API_KEY=sk-your-key");
        info!("   使用 Ollama: export OPENAI_BASE_URL=http://localhost:11434");
        info!("");
        info!("   agent! 宏使用示例：");
        info!("   ```rust");
        info!("   let agent = agent!(");
        info!("       ToolAgent,");
        info!("       provider: provider,");
        info!("       model: \"gpt-4o-mini\",");
        info!("       system_prompt: \"你是有用的助手\",");
        info!("       tools: [GetWeatherTool, CalculatorTool, RandomTool],");
        info!("       max_steps: 10,");
        info!("       temperature: 0.7,");
        info!("   )?;");
        info!("   ```");
        return Ok(());
    }

    let model_name = std::env::var("MODEL_NAME").unwrap_or_else(|_| "gpt-4o-mini".to_string());
    let provider = OpenAiProvider::from_env()?;

    let agent = rucora::agent!(
        ToolAgent,
        provider: provider,
        model: model_name,
        system_prompt: "你是有用的助手。当用户询问天气或使用计算器时，使用相应的工具。",
        tools: [GetWeatherTool, CalculatorTool, RandomTool],
        max_steps: 10,
        temperature: 0.7,
    )?;

    info!("   ✓ Agent 创建成功\n");
    info!("   已注册工具：{:?}", agent.tools());
    info!("");

    // ── 8. 实际运行测试 ──
    info!("8. 实际运行测试...\n");

    let queries = vec![
        "北京天气怎么样？",
        "计算 123 乘以 456 等于多少？",
        "生成一个 1 到 100 之间的随机数",
    ];

    for query in queries {
        info!("用户：{query}");
        match agent.run(query.into()).await {
            Ok(output) => {
                if let Some(text) = output.text() {
                    info!("助手：{text}\n");
                }
            }
            Err(e) => {
                info!("错误：{e}\n");
            }
        }
    }

    info!("示例完成！");
    Ok(())
}
