//! AgentKit Supervisor Agent 示例
//!
//! 展示主管模式的 Agent，协调多个专家 Agent。
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example 17_supervisor_agent
//! ```

use agentkit::agent::{ChatAgent, SimpleAgent, ToolAgent};
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
    info!("║   AgentKit Supervisor Agent 示例      ║");
    info!("╚════════════════════════════════════════╝\n");

    // 检查配置
    if std::env::var("OPENAI_API_KEY").is_err() && std::env::var("OPENAI_BASE_URL").is_err() {
        info!("⚠ 未设置 API 配置");
        info!("   使用 OpenAI: export OPENAI_API_KEY=sk-your-key");
        info!("   使用 Ollama: export OPENAI_BASE_URL=http://localhost:11434");
        return Ok(());
    }

    info!("1. 创建 Provider...\n");

    info!("2. 创建专家 Agent...\n");

    // 注意：由于 Provider 不能 Clone，实际使用中需要：
    // 1. 使用 Arc<Provider>
    // 2. 或者为每个 Agent 创建独立的 Provider 实例
    // 3. 或者实现 SupervisorAgent 统一管理

    info!("   创建 ChatAgent（对话专家）...");
    let provider1 = OpenAiProvider::from_env()?;
    let _chat_agent = ChatAgent::builder()
        .provider(provider1)
        .model("gpt-4o-mini")
        .system_prompt("你是对话专家，擅长自然流畅的对话。")
        .build();
    info!("   ✓ ChatAgent 创建成功");

    info!("   创建 ToolAgent（工具专家）...");
    let provider2 = OpenAiProvider::from_env()?;
    let _tool_agent = ToolAgent::builder()
        .provider(provider2)
        .model("gpt-4o-mini")
        .system_prompt("你是工具专家，擅长使用各种工具完成任务。")
        .tool(EchoTool)
        .build();
    info!("   ✓ ToolAgent 创建成功");

    info!("   创建 SimpleAgent（简单任务专家）...");
    let provider3 = OpenAiProvider::from_env()?;
    let _simple_agent = SimpleAgent::builder()
        .provider(provider3)
        .model("gpt-4o-mini")
        .system_prompt("你是简单任务专家，擅长快速回答简单问题。")
        .build();
    info!("   ✓ SimpleAgent 创建成功\n");

    info!("═══════════════════════════════════════");
    info!("Supervisor 模式说明:");
    info!("═══════════════════════════════════════");
    info!("Supervisor（主管）负责:");
    info!("1. 任务分析 - 理解任务需求");
    info!("2. 任务分配 - 分配给合适的专家 Agent");
    info!("3. 结果聚合 - 汇总各专家的结果");
    info!("4. 质量把控 - 确保最终输出质量");
    info!("═══════════════════════════════════════\n");

    info!("使用场景:");
    info!("• 复杂项目 - 需要多角色协作");
    info!("• 多技能系统 - 不同 Agent 擅长不同领域");
    info!("• 任务分解 - 大任务分解为小任务");
    info!("• 质量保证 - 多层审核确保质量\n");

    info!("提示：SupervisorAgent 需要自定义实现");
    info!("参考本示例的架构设计自己的 Supervisor\n");

    info!("示例完成！");

    Ok(())
}
