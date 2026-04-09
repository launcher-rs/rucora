//! AgentKit 基础聊天示例
//!
//! 展示如何使用 ChatAgent 进行交互式多轮对话。
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example 02_basic_chat
//! ```

use agentkit::agent::ChatAgent;
use agentkit::prelude::Agent;
use agentkit::provider::OpenAiProvider;
use tokio::io::{self, AsyncBufReadExt, BufReader};
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
    info!("║   AgentKit 基础聊天示例               ║");
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

    // 创建 ChatAgent（带对话历史）
    info!("2. 创建 ChatAgent（带对话历史）...");
    let agent = ChatAgent::builder()
        .provider(provider)
        .model(model_name)
        .system_prompt("你是友好的 AI 助手。请记住对话历史，以便进行连贯的多轮对话。")
        .with_conversation(true) // 启用对话历史
        .max_history_messages(20) // 保留最近 20 条消息
        .build();
    info!("✓ ChatAgent 创建成功\n");

    info!("═══════════════════════════════════════");
    info!("开始聊天（输入 'quit' 退出）");
    info!("═══════════════════════════════════════\n");

    // 交互式对话
    let stdin = BufReader::new(io::stdin());
    let mut lines = stdin.lines();

    loop {
        info!("你：");

        if let Ok(Some(line)) = lines.next_line().await {
            let input = line.trim();

            if input.is_empty() {
                continue;
            }

            if input.eq_ignore_ascii_case("quit") || input.eq_ignore_ascii_case("exit") {
                info!("再见！");
                break;
            }

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
        }
    }

    Ok(())
}
