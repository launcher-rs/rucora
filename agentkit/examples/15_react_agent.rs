//! AgentKit ReAct Agent 示例
//!
//! 展示 ReAct（Reason + Act）模式的 Agent。
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example 15_react_agent
//! ```
//!
//! ## 功能演示
//!
//! 1. **ReAct 模式** - 推理 + 行动循环
//! 2. **思考过程** - 显示 Agent 的思考
//! 3. **工具调用** - 自动调用工具
//! 4. **多步推理** - 处理复杂任务

use agentkit::agent::ReActAgent;
use agentkit::prelude::Agent;
use agentkit::provider::OpenAiProvider;
use agentkit::tools::{EchoTool, ShellTool};
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
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

    let model_name = std::env::var("MODEL_NAME").unwrap_or_else(|_| "gpt-4o-mini".to_string());

    // ═══════════════════════════════════════════════════════════
    // ReAct 模式说明
    // ═══════════════════════════════════════════════════════════
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

    // ═══════════════════════════════════════════════════════════
    // 创建 ReAct Agent
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("创建 ReAct Agent");
    info!("═══════════════════════════════════════\n");

    info!("1. 创建 Provider...");
    let provider = OpenAiProvider::from_env()?;
    info!("✓ Provider 创建成功\n");

    info!("2. 创建 ReAct Agent...");
    let agent = ReActAgent::builder()
        .provider(provider)
        .model(&model_name)
        .system_prompt(
            "你是一个善于推理的助手。请遵循以下步骤：\n\
             1. 先思考问题需要什么\n\
             2. 决定是否需要使用工具\n\
             3. 如果需要，调用合适的工具\n\
             4. 观察工具结果\n\
             5. 重复直到能给出最终答案\n\n\
             重要提示：\n\
             - 当前操作系统是 Windows\n\
             - 使用 shell 工具时，请使用 Windows 命令（如 dir 代替 ls，findstr 代替 grep，type 代替 cat）\n\
             - 查找 Rust 文件使用：dir /s /b *.rs\n\
             - 如果不确定命令，请先说明当前平台并询问用户"
        )
        .tool(EchoTool)
        .tool(ShellTool)
        .max_steps(50)
        .build();
    info!("✓ ReAct Agent 创建成功\n");

    // ═══════════════════════════════════════════════════════════
    // 演示任务 1: 简单任务
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("演示任务 1: 简单任务");
    info!("═══════════════════════════════════════\n");

    let task1 = "请重复这句话：Hello, ReAct!";
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
    // 演示任务 2: 需要工具的任务
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("演示任务 2: 需要工具的任务");
    info!("═══════════════════════════════════════\n");

    let task2 = "帮我列出当前目录的所有 Rust 文件";
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
    // 演示任务 3: 多步推理任务
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("演示任务 3: 多步推理任务");
    info!("═══════════════════════════════════════\n");

    let task3 = "先列出当前目录的文件，然后告诉我有多少个文件";
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

    info!("📝 ReAct Agent 总结：\n");

    info!("1. ReAct 优势:");
    info!("   - 透明性 - 可以看到思考过程");
    info!("   - 灵活性 - 动态决定行动");
    info!("   - 准确性 - 多步推理提高准确度");
    info!("   - 可调试 - 容易发现问题所在\n");

    info!("2. 适用场景:");
    info!("   - 复杂问题求解");
    info!("   - 需要多步推理的任务");
    info!("   - 需要工具辅助的任务");
    info!("   - 探索性问题\n");

    info!("3. 配置建议:");
    info!("   - max_steps: 设置合理的最大步数（10-20）");
    info!("   - 系统提示词：明确思考步骤");
    info!("   - 工具选择：提供相关工具\n");

    Ok(())
}
