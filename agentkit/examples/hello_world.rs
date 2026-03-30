//! AgentKit Hello World 示例
//!
//! 这个示例展示如何用最少的代码创建一个 Agent 应用。
//!
//! ## 运行方法
//!
//! ### 使用 OpenAI
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example hello_world
//! ```
//!
//! ### 使用 Ollama（本地）
//! ```bash
//! export OPENAI_BASE_URL=http://localhost:11434
//! cargo run --example hello_world
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
    info!("║   AgentKit Hello World 示例           ║");
    info!("╚════════════════════════════════════════╝\n");

    // 1. 检查 API Key
    info!("1. 检查配置...");
    if std::env::var("OPENAI_API_KEY").is_err()
        && std::env::var("OPENAI_BASE_URL").is_err()
    {
        info!("⚠ 未设置 API 配置");
        info!("   使用 OpenAI: export OPENAI_API_KEY=sk-your-key");
        info!("   使用 Ollama: export OPENAI_BASE_URL=http://localhost:11434");
        return Ok(());
    }
    info!("✓ 配置检查通过\n");

    // 2. 创建 Provider
    info!("2. 创建 Provider...");
    let provider = OpenAiProvider::from_env()?;
    info!("✓ Provider 创建成功\n");

    // 3. 创建 Agent
    info!("3. 创建 Agent...");
    let agent = DefaultAgent::builder()
        .provider(provider)
        .model("gpt-4o-mini")
        .system_prompt("你是友好的智能助手，简洁地回答用户问题。")
        .build();
    info!("✓ Agent 创建成功\n");

    // 4. 测试对话
    info!("4. 测试对话...\n");

    let queries = vec![
        "你好，请介绍一下自己",
        "1+1 等于多少？",
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

    info!("示例完成！");

    Ok(())
}
