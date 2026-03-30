//! AgentKit Reflect Agent 示例
//!
//! 展示反思迭代模式的 Agent。
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example 16_reflect_agent
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
    info!("║   AgentKit Reflect Agent 示例         ║");
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

    // 创建 Reflect Agent
    info!("2. 创建 Reflect Agent...\n");
    let agent = ReflectAgent::builder()
        .provider(provider)
        .model("gpt-4o-mini")
        .system_prompt("你是一个追求卓越的助手。请不断反思和改进你的答案。")
        .max_iterations(3)
        .quality_threshold(0.9)
        .build();
    info!("✓ Reflect Agent 创建成功\n");

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

    info!("适用场景:");
    info!("• 代码生成 - 生成高质量代码");
    info!("• 文档写作 - 不断改进文档质量");
    info!("• 方案设计 - 迭代优化方案");
    info!("• 任何需要高质量输出的任务\n");

    // 演示任务
    info!("═══════════════════════════════════════");
    info!("演示任务：生成快速排序算法");
    info!("═══════════════════════════════════════\n");

    let task = "帮我写一个快速排序算法，要求有详细注释";
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

    info!("示例完成！");

    Ok(())
}
