//! AgentKit 代码助手示例
//!
//! 综合示例：创建一个专注于代码生成的助手。
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example 19_code_assistant
//! ```

use agentkit::agent::ReflectAgent;
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
    info!("║   AgentKit 代码助手示例               ║");
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

    // 创建代码助手（使用 Reflect Agent 保证代码质量）
    info!("2. 创建代码助手...\n");
    let agent = ReflectAgent::builder()
        .provider(provider)
        .model("gpt-4o-mini")
        .system_prompt(
            "你是一个专业的代码助手。请遵循以下原则：
            
            1. 代码质量
               - 编写清晰、可读的代码
               - 添加必要的注释
               - 遵循最佳实践
            
            2. 代码审查
               - 检查边界条件
               - 考虑错误处理
               - 确保代码安全
            
            3. 代码风格
               - 使用一致的命名规范
               - 保持函数简洁
               - 避免重复代码
            
            请不断反思和改进你的代码，直到达到高质量标准。",
        )
        .max_iterations(3)
        .build();
    info!("✓ 代码助手创建成功\n");

    info!("═══════════════════════════════════════");
    info!("代码助手功能:");
    info!("═══════════════════════════════════════");
    info!("1. 代码生成 - 根据需求生成代码");
    info!("2. 代码审查 - 检查代码质量和安全");
    info!("3. 自动重构 - 改进现有代码");
    info!("4. 单元测试 - 生成测试代码");
    info!("5. 文档生成 - 生成代码文档");
    info!("═══════════════════════════════════════\n");

    // 演示任务
    info!("═══════════════════════════════════════");
    info!("演示任务：生成二分查找算法");
    info!("═══════════════════════════════════════\n");

    let task = "帮我用 Rust 写一个二分查找算法，要求：
- 有详细的注释
- 处理边界条件
- 包含单元测试示例";

    info!("任务：{}\n", task);

    match agent.run(task.into()).await {
        Ok(output) => {
            if let Some(text) = output.text() {
                info!("助手：{}\n", text);
            }
        }
        Err(e) => {
            info!("错误：{}\n", e);
        }
    }

    info!("═══════════════════════════════════════");
    info!("提示:");
    info!("═══════════════════════════════════════");
    info!("• 使用 Reflect Agent 确保代码质量");
    info!("• 可以集成 ShellTool 执行代码测试");
    info!("• 可以集成 FileReadTool 读取现有代码");
    info!("• 可以集成 FileWriteTool 保存生成的代码");
    info!("═══════════════════════════════════════\n");

    info!("示例完成！");

    Ok(())
}
