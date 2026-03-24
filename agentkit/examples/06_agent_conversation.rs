//! Agent 自动对话历史管理示例
//!
//! 展示如何使用 Agent 内置的对话历史管理功能，无需手动管理 ConversationManager。
//!
//! # 运行方式
//!
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! export OPENAI_BASE_URL=http://your-server:11434/v1
//!
//! cargo run --example 06_agent_conversation -p agentkit
//! ```

use agentkit::agent::DefaultAgent;
use agentkit::prelude::*;
use agentkit::provider::OpenAiProvider;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔═══════════════════════════════════════════════════════════╗");
    info!("║    Agent 自动对话历史管理示例                              ║");
    info!("╚═══════════════════════════════════════════════════════════╝\n");

    // 创建 Provider
    let provider = OpenAiProvider::from_env()?;
    info!("✓ 已创建 OpenAiProvider");

    // 创建 Agent（启用自动对话历史管理）
    let model = "qwen3.5:9b";
    let agent = DefaultAgent::builder()
        .provider(provider)
        .system_prompt("你是一个专业的ai")
        .model(model)
        .with_conversation(true) // ← 启用自动对话历史管理
        .with_max_messages(20) // ← 保留最近 20 条消息
        .build();

    info!("✓ 已创建 DefaultAgent (模型：{}, 启用对话历史)", model);
    info!("");

    // 示例 1: 简单多轮对话
    info!("=== 示例 1: 简单多轮对话（自动记忆）===\n");

    info!("第 1 轮：用户：'你好，我叫张三'");
    let output = agent.run("你好，我叫张三").await?;
    if let Some(content) = output.text() {
        info!("        助手：{}\n", content);
    }

    info!("第 2 轮：用户：'我今年 25 岁'");
    let output = agent.run("我今年 25 岁").await?;
    if let Some(content) = output.text() {
        info!("        助手：{}\n", content);
    }

    info!("第 3 轮：用户：'你还记得我叫什么吗？'");
    let output = agent.run("你还记得我叫什么吗？").await?;
    if let Some(content) = output.text() {
        info!("        助手：{}\n", content);
        info!("        ✓ Agent 自动记住了用户的名字！\n");
    }

    // 示例 2: 查看对话历史
    info!("=== 示例 2: 查看对话历史 ===\n");

    if let Some(history) = agent.get_conversation_history().await {
        info!("当前对话历史消息数：{}", history.len());
        info!("\n历史记录:");
        for (i, msg) in history.iter().enumerate() {
            let role = match msg.role {
                agentkit_core::provider::types::Role::System => "系统",
                agentkit_core::provider::types::Role::User => "用户",
                agentkit_core::provider::types::Role::Assistant => "助手",
                agentkit_core::provider::types::Role::Tool => "工具",
            };
            let preview: String = msg.content.chars().take(30).collect();
            info!("  {}. [{}] {}", i + 1, role, preview);
        }
    }
    info!("");

    // 示例 3: 清空对话历史
    info!("=== 示例 3: 清空对话历史 ===\n");

    agent.clear_conversation().await;
    info!("✓ 已清空对话历史");

    if let Some(history) = agent.get_conversation_history().await {
        info!("清空后消息数：{}", history.len());
    }
    info!("");

    // 示例 4: 清空后重新开始对话
    info!("=== 示例 4: 清空后重新开始对话 ===\n");

    info!("新第 1 轮：用户：'我是李四'");
    let output = agent.run("我是李四").await?;
    if let Some(content) = output.text() {
        info!("        助手：{}\n", content);
    }

    info!("新第 2 轮：用户：'你还记得我叫什么吗？'");
    let output = agent.run("你还记得我叫什么吗？").await?;
    if let Some(content) = output.text() {
        info!("        助手：{}\n", content);
        info!("        ✓ 因为清空了历史，助手只记得李四，不记得张三了\n");
    }

    // 示例 5: 对比 - 不启用对话历史
    info!("=== 示例 5: 不启用对话历史（对比）===\n");

    let provider2 = OpenAiProvider::from_env()?;
    let agent_no_conv = DefaultAgent::builder()
        .provider(provider2)
        .model(model)
        .with_conversation(false) // 不启用对话历史
        .build();

    info!("创建了一个不启用对话历史的 Agent");

    info!("\n第 1 轮：用户：'我叫王五'");
    let output: Result<AgentOutput, _> = agent_no_conv.run("我叫王五").await;
    if let Ok(output) = output {
        if let Some(content) = output.text() {
            info!("        助手：{}\n", content);
        }
    }

    info!("第 2 轮：用户：'你还记得我叫什么吗？'");
    let output: Result<AgentOutput, _> = agent_no_conv.run("你还记得我叫什么吗？").await;
    if let Ok(output) = output {
        if let Some(content) = output.text() {
            info!("        助手：{}\n", content);
            info!("        ✗ 因为没有启用对话历史，助手不记得用户的名字\n");
        }
    }

    info!("\n=== 示例完成 ===");
    info!("\n总结:");
    info!("  • 启用 with_conversation(true) 后，Agent 自动管理对话历史");
    info!("  • 无需手动调用 ConversationManager");
    info!("  • 每次 run() 会自动添加用户消息和助手回复到历史");
    info!("  • 可以使用 get_conversation_history() 查看历史");
    info!("  • 可以使用 clear_conversation() 清空历史");

    Ok(())
}
