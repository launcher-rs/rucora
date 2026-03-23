//! 综合示例 - 智能研究助手
//!
//! 结合 Provider、Tools、Memory、Conversation 等多个模块
//! 创建一个完整的智能研究助手
//!
//! # 运行方式
//!
//! ```bash
//! export OPENAI_API_KEY=sk-xxx
//! cargo run --example 10_research_assistant -p agentkit
//! ```

use agentkit::conversation::ConversationManager;
use agentkit::memory::InMemoryMemory;
use agentkit::prelude::*;
use agentkit::provider::OpenAiProvider;
use agentkit::runtime::{DefaultRuntime, ToolRegistry};
use agentkit::tools::{EchoTool, FileReadTool, GitTool, ShellTool};
use agentkit_core::memory::{Memory, MemoryItem, MemoryQuery};
use agentkit_core::provider::types::{ChatMessage, ChatRequest};
use std::sync::Arc;
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
    info!("║     AgentKit 综合示例 - 智能研究助手                   ║");
    info!("╚════════════════════════════════════════════════════════╝\n");

    // 检查 API Key
    if std::env::var("OPENAI_API_KEY").is_err() {
        info!("❌ 未找到 OPENAI_API_KEY");
        info!("\n请设置环境变量：");
        info!("  export OPENAI_API_KEY=sk-xxx\n");
        return Ok(());
    }

    // 1. 创建 Provider
    info!("=== 1. 初始化 Provider ===");
    let provider = Arc::new(OpenAiProvider::from_env()?);
    info!("✓ OpenAI Provider 初始化成功\n");

    // 2. 创建工具注册表
    info!("=== 2. 注册工具 ===");
    let tools = ToolRegistry::new()
        .register(EchoTool)
        .register(FileReadTool::new())
        .register(GitTool::new())
        .register(ShellTool::new());
    info!("✓ 已注册 {} 个工具\n", tools.len());

    // 3. 创建记忆系统
    info!("=== 3. 初始化记忆系统 ===");
    let memory = InMemoryMemory::new();

    // 添加一些初始记忆
    memory
        .add(MemoryItem {
            id: "user:preference".to_string(),
            content: "用户喜欢简洁的回答".to_string(),
            metadata: Some(serde_json::json!({"category": "preference"})),
        })
        .await?;
    info!("✓ 记忆系统初始化成功\n");

    // 4. 创建对话管理器
    info!("=== 4. 初始化对话管理 ===");
    let mut conversation = ConversationManager::new().with_max_messages(20);
    conversation
        .ensure_system_prompt("你是一个智能研究助手，可以帮助用户研究问题、编写代码、分析数据。");
    info!("✓ 对话管理器初始化成功\n");

    // 5. 创建运行时
    info!("=== 5. 创建运行时 ===");
    let runtime =
        DefaultRuntime::new(provider.clone(), tools).with_system_prompt("你是一个智能研究助手。");
    info!("✓ 运行时创建成功\n");

    // 6. 模拟研究任务
    info!("=== 6. 执行研究任务 ===\n");

    let research_topic = "Rust 异步编程的基本概念";
    info!("研究主题：{}\n", research_topic);

    // 添加用户问题到对话
    conversation.add_user_message(research_topic);

    // 创建研究请求
    let request = ChatRequest::from_user_text(format!(
        "请详细解释以下主题：{}\n\n请包括：\n\
        1. 基本概念\n\
        2. 核心组件\n\
        3. 使用示例",
        research_topic
    ));

    // 执行研究
    info!("正在研究...\n");
    match provider.chat(request).await {
        Ok(response) => {
            info!("✓ 研究完成\n");
            info!("=== 研究结果 ===\n");
            info!("{}", response.message.content);

            // 添加助手回复到对话
            conversation.add_assistant_message(response.message.content.clone());
        }
        Err(e) => {
            info!("❌ 研究失败：{}\n", e);
        }
    }

    // 7. 保存研究结果到记忆
    info!("\n=== 7. 保存研究结果 ===");
    memory
        .add(MemoryItem {
            id: format!("research:{}", research_topic.replace(" ", "_")),
            content: format!("研究了：{}", research_topic),
            metadata: Some(serde_json::json!({"category": "research"})),
        })
        .await?;
    info!("✓ 研究结果已保存\n");

    // 8. 显示统计信息
    info!("=== 8. 统计信息 ===");
    info!("对话消息数：{}", conversation.get_messages().len());
    info!(
        "记忆条目数：{}",
        memory
            .query(MemoryQuery {
                text: "".to_string(),
                limit: 100
            })
            .await?
            .len()
    );

    info!("\n=== 智能研究助手示例完成 ===");

    Ok(())
}
