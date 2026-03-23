//! Conversation 使用示例
//!
//! 展示如何管理对话历史
//!
//! # 运行方式
//!
//! ```bash
//! cargo run --example 05_conversation -p agentkit
//! ```

use agentkit::conversation::ConversationManager;
use agentkit_core::provider::types::{ChatMessage, Role};
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

    info!("╔════════════════════════════════════════════════════════╗");
    info!("║         AgentKit Conversation 使用示例                 ║");
    info!("╚════════════════════════════════════════════════════════╝\n");

    // 创建对话管理器
    let mut conv = ConversationManager::new().with_max_messages(10);

    // 添加系统提示
    conv.ensure_system_prompt("你是一个有帮助的助手。");
    info!("✓ 已添加系统提示");

    // 模拟多轮对话
    info!("\n=== 模拟对话 ===\n");

    // 第 1 轮
    conv.add_user_message("你好，请介绍一下自己");
    info!("用户：你好，请介绍一下自己");

    conv.add_assistant_message("你好！我是一个 AI 助手，可以帮助你回答问题、完成任务。");
    info!("助手：你好！我是一个 AI 助手，可以帮助你回答问题、完成任务。");

    // 第 2 轮
    conv.add_user_message("你会做什么？");
    info!("\n用户：你会做什么？");

    conv.add_assistant_message(
        "我可以帮助你：\n1. 回答各种问题\n2. 编写和解释代码\n3. 翻译文本\n4. 总结文档\n等等。",
    );
    info!(
        "助手：我可以帮助你：\n1. 回答各种问题\n2. 编写和解释代码\n3. 翻译文本\n4. 总结文档\n等等。"
    );

    // 第 3 轮
    conv.add_user_message("用 Rust 写一个 Hello World");
    info!("\n用户：用 Rust 写一个 Hello World");

    conv.add_assistant_message("```rust\nfn main() {\n    print!(\"Hello, World!\");\n}\n```");
    info!("助手：```rust\\nfn main() {{\\n    print!(\\\"Hello, World!\\\");\\n}}\\n```");

    // 查看对话历史
    info!("\n=== 对话历史 ===");
    let messages = conv.get_messages();
    info!("总消息数：{}", messages.len());

    for (i, msg) in messages.iter().enumerate() {
        let role_str = match msg.role {
            Role::System => "系统",
            Role::User => "用户",
            Role::Assistant => "助手",
            Role::Tool => "工具",
        };
        let preview: String = msg.content.chars().take(50).collect();
        info!("{}. [{}] {}", i + 1, role_str, preview);
    }

    // 测试消息窗口限制
    info!("\n=== 测试消息窗口限制 ===");
    // info!("最大消息数：{}", conv.max_messages);

    // 添加更多消息以触发限制
    for i in 1..=5 {
        conv.add_user_message(format!("消息 {}", i));
        conv.add_assistant_message(format!("回复 {}", i));
    }

    let messages = conv.get_messages();
    info!("添加 10 条消息后的总数：{}", messages.len());
    // info!("（应该不超过最大限制 {}）", conv.max_messages());

    info!("\n=== Conversation 测试完成 ===");

    Ok(())
}
