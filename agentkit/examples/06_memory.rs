//! AgentKit 记忆系统与 Agent 结合使用示例
//!
//! 展示如何将记忆系统与 Agent 集成，实现长期记忆功能。
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example 06_memory
//! ```
//!
//! ## 功能演示
//!
//! 1. **基础记忆操作** - 直接使用 Memory API
//! 2. **Agent + 记忆工具** - Agent 使用 MemoryStoreTool/MemoryRecallTool
//! 3. **对话记忆持久化** - 使用 FileMemory 保存对话历史
//! 4. **RAG 增强记忆** - 结合向量检索的记忆系统

use agentkit::agent::ToolAgent;
use agentkit::memory::{FileMemory, InMemoryMemory};
use agentkit::provider::OpenAiProvider;
use agentkit::tools::{MemoryRecallTool, MemoryStoreTool};
use agentkit_core::agent::Agent;
use agentkit_core::memory::{Memory, MemoryItem, MemoryQuery};
use std::sync::Arc;
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
    info!("║   AgentKit 记忆系统与 Agent 结合示例   ║");
    info!("╚════════════════════════════════════════╝\n");

    // 检查配置
    if std::env::var("OPENAI_API_KEY").is_err() && std::env::var("OPENAI_BASE_URL").is_err() {
        info!("⚠ 未设置 API 配置");
        info!("   使用 OpenAI: export OPENAI_API_KEY=sk-your-key");
        info!("   使用 Ollama: export OPENAI_BASE_URL=http://localhost:11434");
        return Ok(());
    }
    let model_name = std::env::var("MODEL_NAME").unwrap_or_else(|_| "gpt-4o-mini".to_string());

    // ═══════════════════════════════════════════════════════════
    // 示例 1: 基础记忆操作
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 1: 基础记忆操作");
    info!("═══════════════════════════════════════\n");

    let memory = InMemoryMemory::new();

    // 添加记忆
    info!("1.1 添加记忆...");
    memory
        .add(MemoryItem {
            id: "core:user_name".to_string(),
            content: "张三".to_string(),
            metadata: None,
        })
        .await?;

    memory
        .add(MemoryItem {
            id: "core:user_location".to_string(),
            content: "北京市海淀区".to_string(),
            metadata: None,
        })
        .await?;

    memory
        .add(MemoryItem {
            id: "core:user_preference".to_string(),
            content: "喜欢 Python 和 Rust 编程语言".to_string(),
            metadata: None,
        })
        .await?;

    memory
        .add(MemoryItem {
            id: "daily:last_topic".to_string(),
            content: "讨论了机器学习项目".to_string(),
            metadata: None,
        })
        .await?;

    info!("✓ 已添加 4 条记忆\n");

    // 查询记忆
    info!("1.2 查询记忆...");
    let queries = vec!["用户姓名", "编程语言", "机器学习"];

    for query in queries {
        info!("  查询：\"{}\"", query);
        let results = memory
            .query(MemoryQuery {
                text: query.to_string(),
                limit: 3,
            })
            .await?;

        for (i, item) in results.iter().enumerate() {
            info!("    {}. [{}] {}", i + 1, item.id, item.content);
        }
    }
    info!("");

    // ═══════════════════════════════════════════════════════════
    // 示例 2: Agent + 记忆工具（自动记忆和检索）
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 2: Agent + 记忆工具");
    info!("═══════════════════════════════════════\n");

    info!("2.1 创建共享记忆系统...");
    let shared_memory = Arc::new(InMemoryMemory::new());
    info!("✓ 记忆系统创建成功\n");

    info!("2.2 创建记忆工具...");
    let memory_store = MemoryStoreTool::from_memory(shared_memory.clone());
    let memory_recall = MemoryRecallTool::from_memory(shared_memory.clone());
    info!("✓ 记忆工具创建成功\n");

    info!("2.3 创建带记忆功能的 Agent...");
    let provider = OpenAiProvider::from_env()?;

    let agent = ToolAgent::builder()
        .provider(provider)
        .model(&model_name)
        .system_prompt(
            "你是一个有帮助的助手，拥有长期记忆能力。\n\
             你可以使用 memory_store 工具存储重要信息（如用户偏好、事实）。\n\
             你可以使用 memory_recall 工具检索之前存储的信息。\n\
             当用户提到个人信息、偏好或重要事实时，记得存储到记忆中。\n\
             当需要回忆之前的信息时，使用记忆检索工具。",
        )
        .tool(memory_store)
        .tool(memory_recall)
        .max_steps(5)
        .build();
    info!("✓ Agent 创建成功\n");

    // 第一轮对话：存储信息
    info!("2.4 第一轮：存储用户信息...");
    info!("  用户：\"我叫李四，是一名软件工程师，喜欢打篮球。\"");

    let output = agent
        .run("我叫李四，是一名软件工程师，喜欢打篮球。".into())
        .await?;

    info!("  Agent: {}\n", output.text().unwrap_or("无回复"));

    // 检查记忆是否被存储
    info!("2.5 检查记忆存储情况...");
    let stored_memories = shared_memory
        .query(MemoryQuery {
            text: "core:".to_string(),
            limit: 10,
        })
        .await?;

    info!("  当前存储了 {} 条核心记忆:", stored_memories.len());
    for item in &stored_memories {
        info!("    - [{}] {}", item.id, item.content);
    }
    info!("");

    // 第二轮对话：检索信息
    info!("2.6 第二轮：检索用户信息...");
    info!("  用户：\"你还记得关于我的什么信息？\"");

    let output = agent
        .run("你还记得关于我的什么信息？请告诉我。".into())
        .await?;

    info!("  Agent: {}\n", output.text().unwrap_or("无回复"));

    // ═══════════════════════════════════════════════════════════
    // 示例 3: 文件记忆（持久化）
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 3: 文件记忆（持久化）");
    info!("═══════════════════════════════════════\n");

    let memory_path = "memory_test.json";
    info!("3.1 创建文件记忆系统 (路径：{})...", memory_path);

    let file_memory = FileMemory::new(memory_path);

    // 添加记忆
    file_memory
        .add(MemoryItem {
            id: "core:project_info".to_string(),
            content: "项目使用 Rust 语言开发，是一个 Agent 框架".to_string(),
            metadata: None,
        })
        .await?;

    file_memory
        .add(MemoryItem {
            id: "core:team_info".to_string(),
            content: "团队有 5 名开发者，分布在 3 个城市".to_string(),
            metadata: None,
        })
        .await?;

    info!("✓ 已添加 2 条记忆到文件\n");

    // 查询记忆
    info!("3.2 查询文件记忆...");
    let results = file_memory
        .query(MemoryQuery {
            text: "Rust".to_string(),
            limit: 5,
        })
        .await?;

    for item in results {
        info!("  - [{}] {}", item.id, item.content);
    }

    info!("\n✓ 记忆已保存到文件，下次启动时会自动加载\n");

    // ═══════════════════════════════════════════════════════════
    // 示例 4: 对话历史 + 记忆系统
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 4: 对话历史 + 记忆系统");
    info!("═══════════════════════════════════════\n");

    info!("4.1 创建带对话记忆的 Agent...");

    let provider = OpenAiProvider::from_env()?;

    // 创建一个能记住对话历史的 Agent
    let agent_with_memory = ToolAgent::builder()
        .provider(provider)
        .model(&model_name)
        .system_prompt(
            "你是一个友好的对话助手。\n\
             请记住对话中的重要信息，并在适当时候提及。",
        )
        .max_steps(3)
        .build();

    info!("✓ Agent 创建成功\n");

    // 模拟多轮对话
    info!("4.2 多轮对话演示...");

    let conversations = vec![
        "你好，我想了解一下 Rust 语言的特点。",
        "听起来很有趣！那我应该如何开始学习 Rust 呢？",
        "谢谢建议！我之前的编程经验主要是 Python，这对学习 Rust 有帮助吗？",
    ];

    for (i, input) in conversations.iter().enumerate() {
        info!("  第 {} 轮:", i + 1);
        info!("    用户：\"{}\"", input);

        let output = agent_with_memory.run(input.to_string().into()).await?;
        info!("    Agent: {}\n", output.text().unwrap_or("无回复"));
    }

    // ═══════════════════════════════════════════════════════════
    // 总结
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例完成！");
    info!("═══════════════════════════════════════\n");

    info!("📝 记忆系统使用总结：\n");
    info!("1. 基础使用：");
    info!("   - InMemoryMemory: 进程内记忆，适合测试和临时会话");
    info!("   - FileMemory: 文件持久化记忆，适合长期存储\n");

    info!("2. 与 Agent 集成：");
    info!("   - MemoryStoreTool: 让 Agent 主动存储重要信息");
    info!("   - MemoryRecallTool: 让 Agent 检索历史记忆\n");

    info!("3. 记忆分类：");
    info!("   - core: 永久记忆（用户偏好、基本信息）");
    info!("   - daily: 会话记忆（当天内容）");
    info!("   - conversation: 对话上下文\n");

    info!("4. 最佳实践：");
    info!("   - 使用有意义的 ID 前缀进行分类");
    info!("   - 定期清理过期记忆");
    info!("   - 结合向量检索实现语义搜索（RAG）\n");

    Ok(())
}
