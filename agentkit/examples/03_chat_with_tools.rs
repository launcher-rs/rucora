//! AgentKit 带工具聊天示例
//!
//! 展示如何让 Agent 使用工具完成具体任务。
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example 03_chat_with_tools
//! ```

use agentkit::agent::ToolAgent;
use agentkit::prelude::Agent;
use agentkit::provider::OpenAiProvider;
use agentkit::tools::{DatetimeTool, EchoTool};
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔════════════════════════════════════════╗");
    info!("║   AgentKit 带工具聊天示例             ║");
    info!("╚════════════════════════════════════════╝\n");

    // 检查配置
    if std::env::var("OPENAI_API_KEY").is_err() && std::env::var("OPENAI_BASE_URL").is_err() {
        info!("⚠ 未设置 API 配置");
        info!("   使用 OpenAI: export OPENAI_API_KEY=sk-your-key");
        info!("   使用 Ollama: export OPENAI_BASE_URL=http://localhost:11434");
        return Ok(());
    }
    let model_name = std::env::var("MODEL_NAME").expect("没有设置环境变量MODEL_NAME");

    // 创建 Provider
    info!("1. 创建 Provider...");
    let provider = OpenAiProvider::from_env()?;
    info!("✓ Provider 创建成功\n");

    // 创建 ToolAgent（注册工具）
    info!("2. 创建 ToolAgent（注册工具）...");
    let agent = ToolAgent::builder()
        .provider(provider)
        .model(model_name)
        .system_prompt("你是有用的智能助手。当用户询问时间或需要回显时，使用相应的工具。")
        .tool(DatetimeTool) // 注册日期时间工具
        .tool(EchoTool) // 注册回显工具
        .max_steps(10)
        .build();

    info!("✓ ToolAgent 创建成功\n");

    info!("已注册工具：");
    info!("  - DatetimeTool: 获取当前日期和时间");
    info!("  - EchoTool: 回显用户输入\n");

    // 工具调用测试
    info!("3. 工具调用测试...\n");

    let queries = vec!["现在几点了？", "请重复这句话：你好世界", "今天是什么日期？"];

    for query in queries {
        info!("用户：{}", query);

        match agent.run(query.into()).await {
            Ok(output) => {
                if let Some(text) = output.text() {
                    info!("助手：{}\n", text);
                }
            }
            Err(e) => {
                info!("错误：{}\n", e);
            }
        }
    }

    info!("示例完成！");

    Ok(())
}


