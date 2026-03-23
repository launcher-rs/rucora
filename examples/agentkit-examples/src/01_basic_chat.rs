//! 基础聊天示例
//!
//! 展示最简单的对话场景，适合快速上手
//!
//! # 运行方式
//!
//! ```bash
//! # 使用 Mock Provider（不需要 API Key）
//! cargo run --bin basic-chat
//!
//! # 使用真实 OpenAI API
//! export OPENAI_API_KEY=sk-xxx
//! cargo run --bin basic-chat
//! ```

mod utils;

use agentkit::prelude::*;
use agentkit::runtime::{DefaultRuntime, ToolRegistry};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use utils::create_provider_or_mock;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("=== 基础聊天示例 ===\n");

    // 创建 Provider（真实或 Mock）
    let provider = create_provider_or_mock();

    // 创建运行时
    let runtime = DefaultRuntime::new(provider, ToolRegistry::new())
        .with_system_prompt("你是一个友好的助手，用简短的语句回复");

    // 对话循环
    info!("开始对话（输入 'quit' 退出）\n");

    loop {
        print!("你：");
        let mut input_text = String::new();
        std::io::stdin().read_line(&mut input_text)?;
        let input_text = input_text.trim();

        if input_text.is_empty() || input_text == "quit" || input_text == "exit" {
            info!("再见！");
            break;
        }

        // 创建输入
        let input = AgentInput::new(input_text.to_string());

        // 运行对话
        match runtime.run(input).await {
            Ok(output) => {
                if let Some(content) = output.text() {
                    info!("助手：{}\n", content);
                }
            }
            Err(e) => {
                info!("❌ 错误：{}\n", e);
            }
        }
    }

    Ok(())
}
