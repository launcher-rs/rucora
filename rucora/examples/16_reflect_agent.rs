//! rucora Reflect Agent 示例
//!
//! 展示反思迭代模式的 Agent，支持使用工具辅助完成任务。
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! # 或使用 Ollama
//! export OPENAI_BASE_URL=http://127.0.0.1:11434
//! cargo run --example 16_reflect_agent
//! ```
//!
//! ## 功能演示
//!
//! 1. **反思迭代** - 生成 - 反思 - 改进循环
//! 2. **自我批评** - Agent 自我评估
//! 3. **工具增强** - 使用工具获取信息辅助反思
//! 4. **质量保证** - 达到质量阈值

use rucora::agent::ReflectAgent;
use rucora::prelude::Agent;
use rucora::provider::OpenAiProvider;
use rucora::tools::{DatetimeTool, EchoTool, ShellTool};
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔════════════════════════════════════════╗");
    info!("║   rucora Reflect Agent 示例         ║");
    info!("╚════════════════════════════════════════╝\n");

    if std::env::var("OPENAI_API_KEY").is_err() && std::env::var("OPENAI_BASE_URL").is_err() {
        info!("⚠ 未设置 API 配置");
        info!("   使用 OpenAI: export OPENAI_API_KEY=sk-your-key");
        info!("   使用 Ollama: export OPENAI_BASE_URL=http://localhost:11434");
        return Ok(());
    }

    let model = std::env::var("MODEL_NAME").unwrap_or_else(|_| "gpt-4o".to_string());
    info!("使用模型: {}\n", model);

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
    // 创建 Reflect Agent（带工具支持）
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("创建 Reflect Agent");
    info!("═══════════════════════════════════════\n");

    let provider = OpenAiProvider::from_env()?;

    let agent = ReflectAgent::builder()
        .provider(provider)
        .model(&model)
        .system_prompt(
            "你是一个追求卓越的助手，擅长使用工具获取信息。\n\
             请遵循以下步骤：\n\
             1. 如有需要，先使用工具获取相关信息\n\
             2. 生成初始版本的答案\n\
             3. 反思答案的不足之处（正确性、完整性、清晰度）\n\
             4. 根据反思改进答案\n\
             5. 重复直到达到高质量标准",
        )
        .tool(EchoTool)
        .tool(DatetimeTool)
        .tool(ShellTool::new())
        .max_iterations(3)
        .quality_threshold(0.85)
        .build();

    info!("✓ Reflect Agent 创建成功");
    info!("  注册工具: {:?}", agent.tools());
    info!("  最大迭代: 3, 质量阈值: 0.85\n");

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
            info!("❌ 处理失败：{}\n", e);
        }
    }

    // ═══════════════════════════════════════════════════════════
    // 演示任务 2: 需要实时信息的方案
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("演示任务 2: 需要实时信息的方案");
    info!("═══════════════════════════════════════\n");

    let task2 = "今天是几号？帮我安排一个本周的工作计划";
    info!("任务：\"{}\"\n", task2);

    match agent.run(task2.into()).await {
        Ok(output) => {
            if let Some(text) = output.text() {
                info!("回答：\n{}\n", text);
            }
        }
        Err(e) => {
            info!("❌ 处理失败：{}\n", e);
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
            info!("❌ 处理失败：{}\n", e);
        }
    }

    info!("═══════════════════════════════════════");
    info!("示例完成！");
    info!("═══════════════════════════════════════\n");

    info!("📝 ReflectAgent 总结：\n");
    info!("1. 核心优势: 高质量输出、自我改进、减少错误");
    info!("2. 工具增强: 可注册工具辅助信息获取和执行");
    info!("3. 适用场景: 代码生成、文档写作、方案设计");
    info!("4. 配置建议: max_iterations=3-5, quality_threshold=0.8-0.9");

    Ok(())
}
