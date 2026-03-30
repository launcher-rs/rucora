//! AgentKit 研究助手示例
//!
//! 综合示例：结合 Provider、Tools 等模块，创建一个智能研究助手。
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example 18_research_assistant
//! ```

use agentkit::agent::ToolAgent;
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
    info!("║   AgentKit 研究助手示例               ║");
    info!("╚════════════════════════════════════════╝\n");

    // 检查配置
    if std::env::var("OPENAI_API_KEY").is_err() && std::env::var("OPENAI_BASE_URL").is_err() {
        info!("⚠ 未设置 API 配置");
        info!("   使用 OpenAI: export OPENAI_API_KEY=sk-your-key");
        info!("   使用 Ollama: export OPENAI_BASE_URL=http://localhost:11434");
        return Ok(());
    }

    // 1. 创建 Provider
    info!("1. 创建 Provider...");
    let provider = OpenAiProvider::from_env()?;
    info!("✓ Provider 创建成功\n");

    // 2. 创建研究助手 Agent
    info!("2. 创建研究助手 Agent...");
    let agent = ToolAgent::builder()
        .provider(provider)
        .model("gpt-4o-mini")
        .system_prompt(
            "你是一个智能研究助手。你可以帮助用户：
            - 搜集和整理信息
            - 分析和总结资料
            - 生成研究报告
            
            请使用可用的工具完成任务。",
        )
        .tool(EchoTool)
        .max_steps(10)
        .build();
    info!("✓ 研究助手创建成功\n");

    info!("═══════════════════════════════════════");
    info!("研究助手功能:");
    info!("═══════════════════════════════════════");
    info!("1. 信息搜集 - 使用工具搜集相关资料");
    info!("2. 资料整理 - 整理和分类信息");
    info!("3. 分析总结 - 分析资料并生成总结");
    info!("4. 报告生成 - 生成结构化研究报告");
    info!("═══════════════════════════════════════\n");

    // 演示任务
    info!("═══════════════════════════════════════");
    info!("演示任务：Rust 生态系统调研");
    info!("═══════════════════════════════════════\n");

    let task = "帮我调研 Rust 生态系统中常用的 Web 框架";
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
