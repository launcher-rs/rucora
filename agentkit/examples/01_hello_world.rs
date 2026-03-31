//! AgentKit Hello World 示例
//!
//! 这是最简单的 AgentKit 示例，展示如何快速创建一个能对话的 Agent。
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example 01_hello_world
//! ```

use agentkit::agent::SimpleAgent;
use agentkit::prelude::Agent;
use agentkit::provider::OpenAiProvider;
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
    info!("║   AgentKit Hello World                ║");
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

    // 创建 SimpleAgent
    info!("2. 创建 SimpleAgent...");
    let agent = SimpleAgent::builder()
        .provider(provider)
        .model(model_name)
        .system_prompt("你是友好的 AI 助手。请简洁地回答用户的问题。")
        .temperature(0.7)
        .build();
    info!("✓ SimpleAgent 创建成功\n");

    // 运行对话
    info!("3. 运行对话...\n");

    let input = "你好，请介绍一下你自己";
    info!("用户：{}", input);

    match agent.run(input.into()).await {
        Ok(output) => {
            if let Some(text) = output.text() {
                info!("助手：{}\n", text);
            }
        }
        Err(e) => {
            info!("错误：{}\n", e);
        }
    }

    info!("示例完成！");

    Ok(())
}
