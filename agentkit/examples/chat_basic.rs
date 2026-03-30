//! AgentKit 基础对话示例
//!
//! 这个示例展示如何创建一个支持多轮对话的 Agent
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example chat_basic
//! ```

use agentkit::agent::DefaultAgent;
use agentkit::provider::OpenAiProvider;
use tracing::{info, Level};
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
    info!("║   AgentKit 基础对话示例               ║");
    info!("╚════════════════════════════════════════╝\n");

    // 检查配置
    if std::env::var("OPENAI_API_KEY").is_err()
        && std::env::var("OPENAI_BASE_URL").is_err()
    {
        info!("⚠ 未设置 API 配置");
        info!("   使用 OpenAI: export OPENAI_API_KEY=sk-your-key");
        info!("   使用 Ollama: export OPENAI_BASE_URL=http://localhost:11434");
        return Ok(());
    }

    // 创建 Provider
    info!("1. 创建 Provider...");
    let provider = OpenAiProvider::from_env()?;
    info!("✓ Provider 创建成功\n");

    // 创建 Agent（启用对话历史）
    info!("2. 创建 Agent（启用对话历史）...");
    let agent = DefaultAgent::builder()
        .provider(provider)
        .model("gpt-4o-mini")
        .system_prompt("你是友好的智能助手。记住对话历史，提供连贯的回复。")
        .with_conversation(true)  // 启用对话历史
        .with_max_messages(20)    // 保留最近 20 条消息
        .build();
    info!("✓ Agent 创建成功\n");

    // 多轮对话测试
    info!("3. 多轮对话测试...\n");

    let queries = vec![
        "你好，我叫小明",
        "我今年 25 岁",
        "我喜欢编程",
        "你能记住我刚才说的吗？",
    ];

    for query in queries {
        info!("用户：{}", query);

        match agent.run(query).await {
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

    info!("示例完成");

    Ok(())
}
