//! AgentKit ReAct Agent 示例
//!
//! 展示 ReAct（Reason + Act）模式的 Agent。
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example 15_react_agent
//! ```

use agentkit::agent::ReActAgent;
use agentkit::prelude::Agent;
use agentkit::provider::OpenAiProvider;
use agentkit::tools::EchoTool;
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
    info!("║   AgentKit ReAct Agent 示例           ║");
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

    // 创建 ReAct Agent
    info!("2. 创建 ReAct Agent...\n");
    let agent = ReActAgent::builder()
        .provider(provider)
        .model("gpt-4o-mini")
        .system_prompt("你是一个善于推理的助手。请先思考，再行动。")
        .tool(EchoTool)
        .max_steps(15)
        .build();
    info!("✓ ReAct Agent 创建成功\n");

    info!("═══════════════════════════════════════");
    info!("ReAct 模式说明:");
    info!("═══════════════════════════════════════");
    info!("ReAct = Reason（推理） + Act（行动）");
    info!("");
    info!("循环流程:");
    info!("1. Think（思考）- 分析问题，规划步骤");
    info!("2. Act（行动）- 执行工具调用");
    info!("3. Observe（观察）- 分析工具结果");
    info!("4. 重复直到完成任务");
    info!("═══════════════════════════════════════\n");

    // 演示任务
    info!("═══════════════════════════════════════");
    info!("演示任务：分析当前目录结构");
    info!("═══════════════════════════════════════\n");

    let task = "帮我列出当前目录的所有 Rust 文件";
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
