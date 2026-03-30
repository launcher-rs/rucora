//! Memory 与 LLM/Agent/Runtime 结合使用示例
//!
//! 本示例详细展示如何将记忆系统与 LLM Provider、Agent、Runtime 结合使用，
//! 实现具有长期记忆能力的智能 Agent。
//!
//! # 运行方式
//!
//! ```bash
//! # 设置环境变量
//! export OPENAI_API_KEY=sk-your-key
//!
//! # 运行示例
//! cargo run --example 04_memory -p agentkit
//! ```

use agentkit::agent::DefaultAgent;
use agentkit::memory::{FileMemory, InMemoryMemory};
use agentkit::prelude::*;
use agentkit::provider::OpenAiProvider;
use agentkit::runtime::{DefaultRuntime, ToolRegistry};
use agentkit::tools::{MemoryRecallTool, MemoryStoreTool};
use agentkit_core::memory::{Memory, MemoryItem, MemoryQuery};
use agentkit_core::provider::LlmProvider;
use std::sync::Arc;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;
use agentkit_core::Runtime;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔═══════════════════════════════════════════════════════════╗");
    info!("║    AgentKit Memory 与 LLM/Agent/Runtime 结合使用示例      ║");
    info!("╚═══════════════════════════════════════════════════════════╝\n");

    // 示例 1: 基础记忆操作
    info!("=== 示例 1: 基础记忆操作 ===");
    test_basic_memory().await?;

    // 示例 2: 记忆工具与 Agent 配合
    info!("\n=== 示例 2: 记忆工具与 Agent 配合 ===");
    test_memory_with_agent().await?;

    // 示例 3: Runtime 中使用记忆
    info!("\n=== 示例 3: Runtime 中使用记忆 ===");
    test_memory_with_runtime().await?;

    // 示例 4: 实际对话场景中的记忆使用
    info!("\n=== 示例 4: 实际对话场景中的记忆使用 ===");
    test_conversation_with_memory().await?;

    // 示例 5: 文件记忆持久化
    info!("\n=== 示例 5: 文件记忆持久化 ===");
    test_file_memory_persistence().await?;

    info!("\n=== 所有示例完成 ===");

    Ok(())
}

/// 示例 1: 基础记忆操作
/// 展示记忆系统的基本使用方法
async fn test_basic_memory() -> anyhow::Result<()> {
    let memory = Arc::new(InMemoryMemory::new());

    // 1. 添加用户偏好记忆
    info!("1. 添加用户偏好记忆...");
    memory
        .add(MemoryItem {
            id: "core:user_name".to_string(),
            content: "张三".to_string(),
            metadata: Some(serde_json::json!({"category": "personal", "priority": "high"})),
        })
        .await?;
    info!("   ✓ 已添加：用户姓名 = 张三");

    memory
        .add(MemoryItem {
            id: "core:user_lang".to_string(),
            content: "偏好使用 Rust 进行开发".to_string(),
            metadata: Some(serde_json::json!({"category": "technical"})),
        })
        .await?;
    info!("   ✓ 已添加：编程语言偏好 = Rust");

    memory
        .add(MemoryItem {
            id: "core:user_timezone".to_string(),
            content: "Asia/Shanghai (UTC+8)".to_string(),
            metadata: Some(serde_json::json!({"category": "config"})),
        })
        .await?;
    info!("   ✓ 已添加：时区 = Asia/Shanghai");

    // 2. 添加项目上下文记忆
    info!("\n2. 添加项目上下文记忆...");
    memory
        .add(MemoryItem {
            id: "project:current".to_string(),
            content: "AgentKit - 高性能 Rust Agent 框架".to_string(),
            metadata: Some(serde_json::json!({"category": "project", "status": "active"})),
        })
        .await?;
    info!("   ✓ 已添加：当前项目 = AgentKit");

    // 3. 检索记忆
    info!("\n3. 检索记忆...");

    // 按关键词检索
    let results = memory
        .query(MemoryQuery {
            text: "user".to_string(),
            limit: 10,
        })
        .await?;
    info!("   ✓ 找到 {} 条匹配 'user' 的记忆", results.len());
    for item in &results {
        info!("      - {}: {}", item.id, item.content);
    }

    // 按类别检索
    let results = memory
        .query(MemoryQuery {
            text: "core:".to_string(),
            limit: 10,
        })
        .await?;
    info!("\n   ✓ 找到 {} 条 'core:' 类别的记忆", results.len());
    for item in &results {
        info!("      - {}: {}", item.id, item.content);
    }

    // 4. 更新记忆
    info!("\n4. 更新记忆...");
    memory
        .add(MemoryItem {
            id: "core:user_lang".to_string(),
            content: "偏好使用 Rust 和 TypeScript 进行开发".to_string(),
            metadata: Some(serde_json::json!({"category": "technical", "updated": true})),
        })
        .await?;
    info!("   ✓ 已更新：编程语言偏好 = Rust 和 TypeScript");

    // 验证更新
    let results = memory
        .query(MemoryQuery {
            text: "user_lang".to_string(),
            limit: 1,
        })
        .await?;
    if let Some(item) = results.first() {
        info!("   ✓ 验证更新后内容：{}", item.content);
    }

    Ok(())
}

/// 示例 2: 记忆工具与 Agent 配合
/// 展示如何让 Agent 使用记忆工具存储和检索信息
async fn test_memory_with_agent() -> anyhow::Result<()> {
    // 检查是否设置了 API Key
    if std::env::var("OPENAI_API_KEY").is_err() {
        info!("   ⚠ 未设置 OPENAI_API_KEY，跳过此示例");
        info!("   提示：export OPENAI_API_KEY=sk-your-key");
        return Ok(());
    }

    // 1. 创建共享记忆实例
    info!("1. 创建共享记忆实例...");
    let memory = Arc::new(InMemoryMemory::new());
    info!("   ✓ 已创建 InMemoryMemory");

    // 2. 创建记忆工具
    info!("\n2. 创建记忆工具...");
    let memory_store = MemoryStoreTool::from_memory(memory.clone());
    let memory_recall = MemoryRecallTool::from_memory(memory.clone());
    info!("   ✓ 已创建 MemoryStoreTool 和 MemoryRecallTool");

    // 3. 创建 Agent，配备记忆工具
    info!("\n3. 创建配备记忆工具的 Agent...");
    let provider = OpenAiProvider::from_env()?;

    // 从环境变量读取模型或使用默认值
    let model = std::env::var("OPENAI_DEFAULT_MODEL").unwrap_or_else(|_| "qwen3.5:9b".to_string());

    let agent = DefaultAgent::builder()
        .provider(provider)
        .model(model.clone()) // 必须指定模型
        .system_prompt(
            "你是一个有助手的 AI 助手，具有长期记忆能力。

            你可以使用以下工具：
            - memory_store: 存储重要信息到长期记忆（如用户偏好、事实、笔记）
            - memory_recall: 从长期记忆中检索已存储的信息

            记忆类别说明：
            - core: 永久记忆（用户基本信息、偏好）
            - daily: 会话记忆（当天的对话主题）
            - conversation: 对话上下文（最近的对话内容）

            当用户告诉你重要信息时，请使用 memory_store 存储。
            当需要回忆用户信息时，请使用 memory_recall 检索。",
        )
        .tool(memory_store)
        .tool(memory_recall)
        .max_steps(10)
        .build();

    info!("   ✓ 已创建 DefaultAgent，配备记忆工具 (模型：{})", model);

    // 4. 预存一些记忆
    info!("\n4. 预存一些记忆...");
    memory
        .add(MemoryItem {
            id: "core:user_name".to_string(),
            content: "李四".to_string(),
            metadata: None,
        })
        .await?;
    memory
        .add(MemoryItem {
            id: "core:user_company".to_string(),
            content: "某科技公司".to_string(),
            metadata: None,
        })
        .await?;
    info!("   ✓ 已预存：姓名=李四，公司=某科技公司");

    // 5. 与 Agent 对话
    info!("\n5. 与 Agent 对话...");

    // 测试记忆检索
    info!("   问：'你还记得我的名字吗？'");
    let input = AgentInput::new(
        "你还记得我的名字吗？如果不知道，请使用 memory_recall 工具检索 core:user_name",
    );
    let output = agent.run(input).await?;
    info!("   答：{}", output.text().unwrap_or("无回复"));

    // 测试记忆存储
    info!("\n   问：'我喜欢吃川菜，请记住'");
    let input = AgentInput::new("我喜欢吃川菜，请记住这个偏好");
    let output = agent.run(input).await?;
    info!("   答：{}", output.text().unwrap_or("无回复"));

    // 验证记忆已存储
    info!("\n6. 验证记忆已存储...");
    let results = memory
        .query(MemoryQuery {
            text: "food".to_string(),
            limit: 5,
        })
        .await?;
    info!("   ✓ 找到 {} 条相关记忆", results.len());
    for item in &results {
        info!("      - {}: {}", item.id, item.content);
    }

    Ok(())
}

/// 示例 3: Runtime 中使用记忆
/// 展示如何在 Runtime 中集成记忆功能
async fn test_memory_with_runtime() -> anyhow::Result<()> {
    // 1. 创建记忆实例
    info!("1. 创建记忆实例...");
    let memory = Arc::new(InMemoryMemory::new());

    // 预存一些上下文信息
    memory
        .add(MemoryItem {
            id: "context:project".to_string(),
            content: "当前项目是 AgentKit，一个 Rust 编写的 Agent 框架".to_string(),
            metadata: None,
        })
        .await?;
    memory
        .add(MemoryItem {
            id: "context:goal".to_string(),
            content: "目标是构建高性能、类型安全的 LLM 应用开发框架".to_string(),
            metadata: None,
        })
        .await?;
    info!("   ✓ 已预存项目上下文信息");

    // 2. 创建记忆工具
    info!("\n2. 创建记忆工具...");
    let memory_store = MemoryStoreTool::from_memory(memory.clone());
    let memory_recall = MemoryRecallTool::from_memory(memory.clone());

    // 3. 创建工具注册表并添加记忆工具
    info!("\n3. 创建 ToolRegistry 并添加记忆工具...");
    let tools = ToolRegistry::new()
        .register(memory_store)
        .register(memory_recall);
    info!("   ✓ 已注册记忆工具");

    // 4. 创建 Runtime
    info!("\n4. 创建 DefaultRuntime...");
    let provider: Arc<dyn LlmProvider + Send + Sync> = Arc::new(OpenAiProvider::from_env()?);

    let runtime = DefaultRuntime::new(provider, tools, "qwen3.5:9b")
        .with_system_prompt(
            "你是一个项目助手，帮助管理项目信息和上下文。

        你可以使用：
        - memory_store: 存储项目信息、任务、笔记
        - memory_recall: 检索已存储的项目信息

        请主动使用这些工具来记住重要信息。",
        )
        .with_max_steps(10);

    info!("   ✓ 已创建 DefaultRuntime");

    // 5. 与 Runtime 交互
    info!("\n5. 与 Runtime 交互...");

    info!("   问：'当前项目的目标是什么？'");
    let input = AgentInput::new("当前项目的目标是什么？请使用 memory_recall 检索 context:goal");
    let output = runtime.run(input).await?;
    info!("   答：{}", output.text().unwrap_or("无回复"));

    info!("\n   问：'请记住：下一个里程碑是完成 v0.2.0 版本'");
    let input = AgentInput::new("请记住：下一个里程碑是完成 v0.2.0 版本，预计下个月发布");
    let output = runtime.run(input).await?;
    info!("   答：{}", output.text().unwrap_or("无回复"));

    // 6. 验证记忆
    info!("\n6. 验证记忆已存储...");
    let results = memory
        .query(MemoryQuery {
            text: "milestone".to_string(),
            limit: 5,
        })
        .await?;
    info!("   ✓ 找到 {} 条相关记忆", results.len());
    for item in &results {
        info!("      - {}: {}", item.id, item.content);
    }

    Ok(())
}

/// 示例 4: 实际对话场景中的记忆使用
/// 展示在多轮对话中如何使用记忆保持上下文
async fn test_conversation_with_memory() -> anyhow::Result<()> {
    info!("场景：多轮对话中保持用户信息记忆\n");

    // 1. 创建记忆和工具
    info!("1. 初始化记忆系统...");
    let memory = Arc::new(InMemoryMemory::new());
    let memory_store = MemoryStoreTool::from_memory(memory.clone());
    let memory_recall = MemoryRecallTool::from_memory(memory.clone());

    // 2. 创建 Agent
    info!("2. 创建具有记忆能力的 Agent...");
    let provider = OpenAiProvider::from_env()?;

    let agent = DefaultAgent::builder()
        .provider(provider)
        .model("qwen3.5:9b")
        .system_prompt(
            "你是一个贴心的个人助手，具有优秀的记忆能力。
            
            工具使用指南：
            1. 当用户告诉你个人信息（姓名、偏好、习惯等）时，立即使用 memory_store 存储
            2. 当用户询问你之前记录的信息时，使用 memory_recall 检索
            3. 记忆键命名规范：core:user_<类型>，如 core:user_name, core:user_food_preference
            
            请自然地与用户对话，适时使用记忆工具。",
        )
        .tool(memory_store)
        .tool(memory_recall)
        .max_steps(10)
        .build();

    info!("   ✓ Agent 已就绪\n");

    // 3. 模拟多轮对话
    info!("=== 开始多轮对话 ===\n");

    // 第一轮：收集信息
    info!("【对话 1】用户：'你好，我叫王五'");
    let input = AgentInput::new("你好，我叫王五");
    let output = agent.run(input).await?;
    info!("        助手：{}\n", output.text().unwrap_or("无回复"));

    // 第二轮：收集更多偏好
    info!("【对话 2】用户：'我平时喜欢喝咖啡，特别是拿铁'");
    let input = AgentInput::new("我平时喜欢喝咖啡，特别是拿铁");
    let output = agent.run(input).await?;
    info!("        助手：{}\n", output.text().unwrap_or("无回复"));

    // 第三轮：测试记忆检索
    info!("【对话 3】用户：'你还记得我叫什么吗？'");
    let input = AgentInput::new("你还记得我叫什么名字吗？");
    let output = agent.run(input).await?;
    info!("        助手：{}\n", output.text().unwrap_or("无回复"));

    // 第四轮：测试更多记忆
    info!("【对话 4】用户：'我喜欢喝什么咖啡？'");
    let input = AgentInput::new("我喜欢喝什么类型的咖啡？");
    let output = agent.run(input).await?;
    info!("        助手：{}\n", output.text().unwrap_or("无回复"));

    // 4. 检查记忆存储
    info!("=== 记忆存储状态 ===");
    let results = memory
        .query(MemoryQuery {
            text: "core:user".to_string(),
            limit: 10,
        })
        .await?;
    info!("已存储 {} 条用户记忆:", results.len());
    for item in &results {
        info!("  - {}: {}", item.id, item.content);
    }

    Ok(())
}

/// 示例 5: 文件记忆持久化
/// 展示如何使用 FileMemory 实现记忆持久化
async fn test_file_memory_persistence() -> anyhow::Result<()> {
    use std::path::Path;

    let memory_file = "demo_memory.json";

    info!("1. 创建文件记忆（持久化存储）...");
    let memory = Arc::new(FileMemory::new(memory_file));
    info!("   ✓ 记忆文件：{}", memory_file);

    info!("\n2. 添加记忆...");
    memory
        .add(MemoryItem {
            id: "core:favorite_color".to_string(),
            content: "蓝色".to_string(),
            metadata: Some(serde_json::json!({"learned_at": "2024-01-01"})),
        })
        .await?;
    info!("   ✓ 已添加：喜欢的颜色 = 蓝色");

    memory
        .add(MemoryItem {
            id: "core:birthday".to_string(),
            content: "3 月 15 日".to_string(),
            metadata: None,
        })
        .await?;
    info!("   ✓ 已添加：生日 = 3 月 15 日");

    memory
        .add(MemoryItem {
            id: "daily:last_conversation".to_string(),
            content: "讨论了关于 Rust 异步编程的话题".to_string(),
            metadata: None,
        })
        .await?;
    info!("   ✓ 已添加：上次对话主题 = Rust 异步编程");

    info!("\n3. 检索记忆...");
    let results = memory
        .query(MemoryQuery {
            text: "core:".to_string(),
            limit: 10,
        })
        .await?;
    info!("   ✓ 找到 {} 条 'core:' 记忆", results.len());
    for item in &results {
        info!("      - {}: {}", item.id, item.content);
    }

    info!("\n4. 验证文件已创建...");
    if Path::new(memory_file).exists() {
        info!("   ✓ 记忆文件已创建：{}", memory_file);

        // 显示文件内容
        let content = tokio::fs::read_to_string(memory_file).await?;
        info!("\n   文件内容预览:");
        for line in content.lines().take(15) {
            info!("      {}", line);
        }
        info!("      ...");
    } else {
        info!("   ✗ 记忆文件未创建");
    }

    info!("\n5. 模拟重启后重新加载...");
    // 创建新的 FileMemory 实例，模拟程序重启
    let memory2 = Arc::new(FileMemory::new(memory_file));
    let results = memory2
        .query(MemoryQuery {
            text: "birthday".to_string(),
            limit: 1,
        })
        .await?;
    if let Some(item) = results.first() {
        info!("   ✓ 成功从文件加载记忆：{} = {}", item.id, item.content);
    }

    // 清理测试文件
    let _ = std::fs::remove_file(memory_file);
    info!("\n6. ✓ 已清理测试文件");

    Ok(())
}
