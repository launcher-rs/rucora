//! AgentKit 对话管理示例
//!
//! 展示如何管理多轮对话历史。
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example 05_conversation
//! ```

use agentkit::agent::ChatAgent;
use agentkit::prelude::Agent;
use agentkit::provider::OpenAiProvider;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔════════════════════════════════════════╗");
    info!("║   AgentKit 对话管理示例               ║");
    info!("╚════════════════════════════════════════╝\n");

    // 检查配置
    if std::env::var("OPENAI_API_KEY").is_err() && std::env::var("OPENAI_BASE_URL").is_err() {
        info!("⚠ 未设置 API 配置");
        info!("   使用 OpenAI: export OPENAI_API_KEY=sk-your-key");
        info!("   使用 Ollama: export OPENAI_BASE_URL=http://localhost:11434");
        return Ok(());
    }

    // 创建 Provider
    info!("1. 创建 Provider...");
    let provider = OpenAiProvider::from_env()?;
    info!("✓ Provider 创建成功\n");

    // 创建 ChatAgent（带对话历史）
    info!("2. 创建 ChatAgent（带对话历史）...");
    let agent = ChatAgent::builder()
        .provider(provider)
        .model("gpt-4o-mini")
        .system_prompt("你是友好的 AI 助手。请记住对话历史，以便进行连贯的多轮对话。")
        .with_conversation(true) // 启用对话历史
        .max_history_messages(10) // 保留最近 10 条消息
        .build();
    info!("✓ ChatAgent 创建成功\n");

    // 演示多轮对话
    info!("═══════════════════════════════════════");
    info!("演示多轮对话");
    info!("═══════════════════════════════════════\n");

    // 第一轮
    info!("第一轮对话:");
    info!("用户：你好，我叫小明");

    match agent.run("你好，我叫小明".into()).await {
        Ok(output) => {
            if let Some(text) = output.text() {
                info!("助手：{}\n", text);
            }
        }
        Err(e) => {
            info!("错误：{}\n", e);
        }
    }

    // 第二轮
    info!("第二轮对话:");
    info!("用户：你还记得我叫什么吗？");

    match agent.run("你还记得我叫什么吗？".into()).await {
        Ok(output) => {
            if let Some(text) = output.text() {
                info!("助手：{}\n", text);
            }
        }
        Err(e) => {
            info!("错误：{}\n", e);
        }
    }

    // 第三轮
    info!("第三轮对话:");
    info!("用户：我今年 25 岁，是一名程序员");

    match agent.run("我今年 25 岁，是一名程序员".into()).await {
        Ok(output) => {
            if let Some(text) = output.text() {
                info!("助手：{}\n", text);
            }
        }
        Err(e) => {
            info!("错误：{}\n", e);
        }
    }

    // 第四轮
    info!("第四轮对话:");
    info!("用户：你还记得我今年多大吗？");

    match agent.run("你还记得我今年多大吗？".into()).await {
        Ok(output) => {
            if let Some(text) = output.text() {
                info!("助手：{}\n", text);
            }
        }
        Err(e) => {
            info!("错误：{}\n", e);
        }
    }

    // 查看对话历史
    info!("═══════════════════════════════════════");
    info!("查看对话历史");
    info!("═══════════════════════════════════════\n");

    if let Some(history) = agent.get_conversation_history().await {
        info!("对话历史消息数：{}", history.len());
        for (i, msg) in history.iter().enumerate() {
            info!("  {}: [{:?}] {}", i + 1, msg.role, msg.content);
        }
    }

    // 清空对话历史
    info!("\n═══════════════════════════════════════");
    info!("清空对话历史");
    info!("═══════════════════════════════════════\n");

    agent.clear_conversation().await;
    info!("✓ 对话历史已清空\n");

    // 验证清空
    if let Some(history) = agent.get_conversation_history().await {
        info!("清空后的对话历史消息数：{}", history.len());
    }

    info!("\n═══════════════════════════════════════");
    info!("示例完成！");
    info!("═══════════════════════════════════════");

    Ok(())
}
