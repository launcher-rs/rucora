//! AgentKit 代码助手示例
//!
//! 综合示例：创建一个专注于代码生成的助手。
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example 19_code_assistant
//! ```
//!
//! ## 功能演示
//!
//! 1. **代码生成** - 根据需求生成代码
//! 2. **代码审查** - 检查代码质量和安全
//! 3. **自动重构** - 改进现有代码
//! 4. **单元测试** - 生成测试代码
//! 5. **文档生成** - 生成代码文档

use agentkit::agent::ReflectAgent;
use agentkit::prelude::Agent;
use agentkit::provider::OpenAiProvider;
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
    info!("║   AgentKit 代码助手示例               ║");
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
    // 创建代码助手
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("创建代码助手");
    info!("═══════════════════════════════════════\n");

    info!("1. 创建 Provider...");
    let provider = OpenAiProvider::from_env()?;
    info!("✓ Provider 创建成功\n");

    info!("2. 创建代码助手（使用 Reflect Agent 保证代码质量）...\n");
    let agent = ReflectAgent::builder()
        .provider(provider)
        .model(&model_name)
        .system_prompt(
            "你是一个专业的代码助手。请遵循以下原则：\n\n\
             1. 代码质量\n\
                - 编写清晰、可读的代码\n\
                - 添加必要的注释\n\
                - 遵循最佳实践\n\n\
             2. 代码审查\n\
                - 检查边界条件\n\
                - 考虑错误处理\n\
                - 确保代码安全\n\n\
             3. 代码风格\n\
                - 使用一致的命名规范\n\
                - 保持函数简洁\n\
                - 避免重复代码\n\n\
             请不断反思和改进你的代码，直到达到高质量标准。",
        )
        .max_iterations(3)
        .build();
    info!("✓ 代码助手创建成功\n");

    // ═══════════════════════════════════════════════════════════
    // 代码助手功能
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("代码助手功能:");
    info!("═══════════════════════════════════════");
    info!("1. 代码生成 - 根据需求生成代码");
    info!("2. 代码审查 - 检查代码质量和安全");
    info!("3. 自动重构 - 改进现有代码");
    info!("4. 单元测试 - 生成测试代码");
    info!("5. 文档生成 - 生成代码文档");
    info!("═══════════════════════════════════════\n");

    // ═══════════════════════════════════════════════════════════
    // 演示任务 1: 算法实现
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("演示任务 1: 算法实现");
    info!("═══════════════════════════════════════\n");

    let task1 = "帮我用 Rust 写一个二分查找算法，要求：\n\
                 - 有详细的注释\n\
                 - 处理边界条件\n\
                 - 包含单元测试示例";

    info!("任务：\n{}\n", task1);

    match agent.run(task1.into()).await {
        Ok(output) => {
            if let Some(text) = output.text() {
                info!("生成的代码：\n{}\n", text);
            }
        }
        Err(e) => {
            info!("错误：{}\n", e);
        }
    }

    // ═══════════════════════════════════════════════════════════
    // 演示任务 2: 代码重构
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("演示任务 2: 代码重构");
    info!("═══════════════════════════════════════\n");

    let task2 = "请重构以下代码，使其更清晰、更符合 Rust 风格：\n\
                 ```rust\n\
                 fn process_data(data: Vec<i32>) -> Vec<i32> {\n\
                     let mut result = Vec::new();\n\
                     for i in 0..data.len() {\n\
                         if data[i] % 2 == 0 {\n\
                             result.push(data[i] * 2);\n\
                         }\n\
                     }\n\
                     result\n\
                 }\n\
                 ```";

    info!("任务：\n{}\n", task2);

    match agent.run(task2.into()).await {
        Ok(output) => {
            if let Some(text) = output.text() {
                info!("重构后的代码：\n{}\n", text);
            }
        }
        Err(e) => {
            info!("错误：{}\n", e);
        }
    }

    // ═══════════════════════════════════════════════════════════
    // 演示任务 3: 代码审查
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("演示任务 3: 代码审查");
    info!("═══════════════════════════════════════\n");

    let task3 = "请审查以下代码，指出潜在问题并提出改进建议：\n\
                 ```rust\n\
                 fn get_first_element(vec: &Vec<i32>) -> i32 {\n\
                     vec[0]\n\
                 }\n\
                 ```";

    info!("任务：\n{}\n", task3);

    match agent.run(task3.into()).await {
        Ok(output) => {
            if let Some(text) = output.text() {
                info!("审查意见：\n{}\n", text);
            }
        }
        Err(e) => {
            info!("错误：{}\n", e);
        }
    }

    // ═══════════════════════════════════════════════════════════
    // 演示任务 4: 生成文档
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("演示任务 4: 生成文档");
    info!("═══════════════════════════════════════\n");

    let task4 = "为以下函数生成 Rust 风格的文档注释：\n\
                 ```rust\n\
                 pub fn calculate_compound_interest(principal: f64, rate: f64, time: u32) -> f64 {\n\
                     principal * (1.0 + rate).powi(time as i32)\n\
                 }\n\
                 ```";

    info!("任务：\n{}\n", task4);

    match agent.run(task4.into()).await {
        Ok(output) => {
            if let Some(text) = output.text() {
                info!("生成的文档：\n{}\n", text);
            }
        }
        Err(e) => {
            info!("错误：{}\n", e);
        }
    }

    // ═══════════════════════════════════════════════════════════
    // 代码最佳实践
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("代码最佳实践:");
    info!("═══════════════════════════════════════\n");

    info!("1. Rust 代码风格:");
    info!("   - 使用 rustfmt 格式化代码");
    info!("   - 使用 clippy 检查代码问题");
    info!("   - 遵循 Rust API 命名规范\n");

    info!("2. 错误处理:");
    info!("   - 使用 Result 处理可恢复错误");
    info!("   - 使用 Option 处理可选值");
    info!("   - 避免 unwrap()，使用 expect() 或模式匹配\n");

    info!("3. 内存安全:");
    info!("   - 理解所有权和借用");
    info!("   - 优先使用引用而非克隆");
    info!("   - 使用智能指针管理资源\n");

    info!("4. 测试:");
    info!("   - 编写单元测试");
    info!("   - 编写集成测试");
    info!("   - 使用 cargo test 运行测试\n");

    info!("5. 文档:");
    info!("   - 使用 /// 编写文档注释");
    info!("   - 包含使用示例");
    info!("   - 使用 cargo doc 生成文档\n");

    // ═══════════════════════════════════════════════════════════
    // 总结
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例完成！");
    info!("═══════════════════════════════════════\n");

    info!("📝 代码助手总结：\n");

    info!("1. 代码助手能力:");
    info!("   - 代码生成 - 快速实现功能");
    info!("   - 代码审查 - 发现潜在问题");
    info!("   - 代码重构 - 提高代码质量");
    info!("   - 测试生成 - 保证代码正确性");
    info!("   - 文档生成 - 提高可维护性\n");

    info!("2. 使用 Reflect Agent 的优势:");
    info!("   - 自我反思 - 发现代码问题");
    info!("   - 持续改进 - 迭代优化代码");
    info!("   - 高质量 - 达到更高标准\n");

    info!("3. 集成建议:");
    info!("   - FileReadTool - 读取现有代码");
    info!("   - FileWriteTool - 保存生成的代码");
    info!("   - ShellTool - 运行测试和格式化");
    info!("   - GitTool - 管理代码版本\n");

    Ok(())
}
