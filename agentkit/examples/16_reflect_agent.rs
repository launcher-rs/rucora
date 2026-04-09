//! AgentKit Reflect Agent 示例
//!
//! 展示反思迭代模式的 Agent。
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example 16_reflect_agent
//! ```
//!
//! ## 功能演示
//!
//! 1. **反思迭代** - 生成 - 反思 - 改进循环
//! 2. **自我批评** - Agent 自我评估
//! 3. **持续改进** - 迭代优化输出
//! 4. **质量保证** - 达到质量阈值

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
    info!("║   AgentKit Reflect Agent 示例         ║");
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
    // Reflect 模式说明
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("Reflect 模式说明:");
    info!("═══════════════════════════════════════");
    info!("Reflect = Generate（生成） + Reflect（反思） + Improve（改进）");
    info!("");
    info!("循环流程:");
    info!("1. Generate - 生成初始版本");
    info!("2. Reflect - 自我批评，分析问题");
    info!("3. Improve - 根据反思改进");
    info!("4. 重复直到达到质量阈值或最大迭代次数");
    info!("═══════════════════════════════════════\n");

    // ═══════════════════════════════════════════════════════════
    // 创建 Reflect Agent
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("创建 Reflect Agent");
    info!("═══════════════════════════════════════\n");

    info!("1. 创建 Provider...");
    let provider = OpenAiProvider::from_env()?;
    info!("✓ Provider 创建成功\n");

    info!("2. 创建 Reflect Agent...");
    let agent = ReflectAgent::builder()
        .provider(provider)
        .model(&model_name)
        .system_prompt(
            "你是一个追求卓越的助手。请遵循以下步骤：\n\
             1. 生成初始版本的答案\n\
             2. 反思答案的不足之处\n\
             3. 改进答案\n\
             4. 重复直到达到高质量标准",
        )
        .max_iterations(3)
        .build();
    info!("✓ Reflect Agent 创建成功\n");

    // ═══════════════════════════════════════════════════════════
    // 演示任务 1: 代码生成
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("演示任务 1: 代码生成");
    info!("═══════════════════════════════════════\n");

    let task1 = "帮我写一个快速排序算法，要求有详细注释";
    info!("任务：\"{}\"\n", task1);

    match agent.run(task1.into()).await {
        Ok(output) => {
            if let Some(text) = output.text() {
                info!("回答：\n{}\n", text);
            }
        }
        Err(e) => {
            info!("错误：{}\n", e);
        }
    }

    // ═══════════════════════════════════════════════════════════
    // 演示任务 2: 文档写作
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("演示任务 2: 文档写作");
    info!("═══════════════════════════════════════\n");

    let task2 = "帮我写一个项目 README 文件的简介部分，项目是一个 Rust 编写的 Web 框架";
    info!("任务：\"{}\"\n", task2);

    match agent.run(task2.into()).await {
        Ok(output) => {
            if let Some(text) = output.text() {
                info!("回答：\n{}\n", text);
            }
        }
        Err(e) => {
            info!("错误：{}\n", e);
        }
    }

    // ═══════════════════════════════════════════════════════════
    // 演示任务 3: 方案设计
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("演示任务 3: 方案设计");
    info!("═══════════════════════════════════════\n");

    let task3 = "设计一个用户认证系统，需要考虑安全性和扩展性";
    info!("任务：\"{}\"\n", task3);

    match agent.run(task3.into()).await {
        Ok(output) => {
            if let Some(text) = output.text() {
                info!("回答：\n{}\n", text);
            }
        }
        Err(e) => {
            info!("错误：{}\n", e);
        }
    }

    // ═══════════════════════════════════════════════════════════
    // 总结
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例完成！");
    info!("═══════════════════════════════════════\n");

    info!("📝 Reflect Agent 总结：\n");

    info!("1. Reflect 优势:");
    info!("   - 高质量输出 - 多次迭代优化");
    info!("   - 自我改进 - 自动发现问题");
    info!("   - 一致性 - 减少矛盾和错误");
    info!("   - 完整性 - 考虑更全面的因素\n");

    info!("2. 适用场景:");
    info!("   - 代码生成 - 生成高质量代码");
    info!("   - 文档写作 - 不断改进文档质量");
    info!("   - 方案设计 - 迭代优化方案");
    info!("   - 任何需要高质量输出的任务\n");

    info!("3. 配置建议:");
    info!("   - max_iterations: 3-5 次迭代");
    info!("   - quality_threshold: 0.8-0.9 质量阈值");
    info!("   - 系统提示词：明确质量标准\n");

    info!("4. 性能考虑:");
    info!("   - 更多迭代 = 更长时间");
    info!("   - 权衡质量和成本");
    info!("   - 简单任务不需要反思\n");

    Ok(())
}
