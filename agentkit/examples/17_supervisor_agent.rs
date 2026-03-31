//! AgentKit Supervisor Agent 示例
//!
//! 展示主管模式的 Agent，协调多个专家 Agent。
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example 17_supervisor_agent
//! ```
//!
//! ## 功能演示
//!
//! 1. **多专家协作** - 不同 Agent 擅长不同领域
//! 2. **任务分配** - 主管分配任务给专家
//! 3. **结果聚合** - 汇总各专家的结果
//! 4. **质量把控** - 确保最终输出质量

use agentkit::agent::{ChatAgent, SimpleAgent, ToolAgent};
use agentkit::provider::OpenAiProvider;
use agentkit::tools::EchoTool;
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
    info!("║   AgentKit Supervisor Agent 示例      ║");
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
    // Supervisor 模式说明
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("Supervisor 模式说明:");
    info!("═══════════════════════════════════════");
    info!("Supervisor（主管）负责:");
    info!("1. 任务分析 - 理解任务需求");
    info!("2. 任务分配 - 分配给合适的专家 Agent");
    info!("3. 结果聚合 - 汇总各专家的结果");
    info!("4. 质量把控 - 确保最终输出质量");
    info!("═══════════════════════════════════════\n");

    // ═══════════════════════════════════════════════════════════
    // 创建专家 Agent
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("创建专家 Agent");
    info!("═══════════════════════════════════════\n");

    info!("1. 创建 Provider...\n");

    // 注意：由于 Provider 不能 Clone，实际使用中需要：
    // 1. 使用 Arc<Provider>
    // 2. 或者为每个 Agent 创建独立的 Provider 实例
    // 3. 或者实现 SupervisorAgent 统一管理

    info!("2. 创建 ChatAgent（对话专家）...");
    let provider1 = OpenAiProvider::from_env()?;
    let _chat_agent = ChatAgent::builder()
        .provider(provider1)
        .model(&model_name)
        .system_prompt("你是对话专家，擅长自然流畅的对话。")
        .build();
    info!("   ✓ ChatAgent 创建成功");

    info!("3. 创建 ToolAgent（工具专家）...");
    let provider2 = OpenAiProvider::from_env()?;
    let _tool_agent = ToolAgent::builder()
        .provider(provider2)
        .model(&model_name)
        .system_prompt("你是工具专家，擅长使用各种工具完成任务。")
        .tool(EchoTool)
        .build();
    info!("   ✓ ToolAgent 创建成功");

    info!("4. 创建 SimpleAgent（简单任务专家）...");
    let provider3 = OpenAiProvider::from_env()?;
    let _simple_agent = SimpleAgent::builder()
        .provider(provider3)
        .model(&model_name)
        .system_prompt("你是简单任务专家，擅长快速回答简单问题。")
        .build();
    info!("   ✓ SimpleAgent 创建成功\n");

    // ═══════════════════════════════════════════════════════════
    // Supervisor 模式架构
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("Supervisor 模式架构:");
    info!("═══════════════════════════════════════\n");

    info!("```\n");
    info!("                    ┌─────────────┐");
    info!("                    │ Supervisor  │");
    info!("                    │   (主管)    │");
    info!("                    └──────┬──────┘");
    info!("                           │");
    info!("          ┌────────────────┼────────────────┐");
    info!("          │                │                │");
    info!("          ▼                ▼                ▼");
    info!("   ┌──────────┐    ┌──────────┐    ┌──────────┐");
    info!("   │  Chat    │    │  Tool    │    │ Simple   │");
    info!("   │  Agent   │    │  Agent   │    │  Agent   │");
    info!("   │ (对话)    │    │ (工具)    │    │ (简单)    │");
    info!("   └──────────┘    └──────────┘    └──────────┘");
    info!("```\n");

    // ═══════════════════════════════════════════════════════════
    // 使用场景
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("使用场景:");
    info!("═══════════════════════════════════════\n");

    info!("1. 复杂项目:");
    info!("   - 需要多角色协作");
    info!("   - 任务需要分解");
    info!("   - 各部分相互依赖\n");

    info!("2. 多技能系统:");
    info!("   - 不同 Agent 擅长不同领域");
    info!("   - 例如：医疗 + 法律 + 财务");
    info!("   - 根据问题类型路由\n");

    info!("3. 任务分解:");
    info!("   - 大任务分解为小任务");
    info!("   - 并行执行子任务");
    info!("   - 聚合最终结果\n");

    info!("4. 质量保证:");
    info!("   - 多层审核确保质量");
    info!("   - 专家之间相互验证");
    info!("   - 主管最终把关\n");

    // ═══════════════════════════════════════════════════════════
    // 实现 Supervisor 的思路
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("实现 Supervisor 的思路:");
    info!("═══════════════════════════════════════\n");

    info!("1. 定义专家注册表:");
    info!("   - 记录每个专家的能力");
    info!("   - 支持按能力查找专家");
    info!("   - 管理专家生命周期\n");

    info!("2. 实现任务分析:");
    info!("   - 理解任务需求");
    info!("   - 识别需要的技能");
    info!("   - 决定是否需要多个专家\n");

    info!("3. 实现任务分配:");
    info!("   - 根据能力匹配专家");
    info!("   - 处理任务依赖关系");
    info!("   - 监控执行进度\n");

    info!("4. 实现结果聚合:");
    info!("   - 收集各专家结果");
    info!("   - 解决结果冲突");
    info!("   - 生成最终输出\n");

    // ═══════════════════════════════════════════════════════════
    // 总结
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例完成！");
    info!("═══════════════════════════════════════\n");

    info!("📝 Supervisor Agent 总结：\n");

    info!("1. Supervisor 优势:");
    info!("   - 专业化分工 - 每个 Agent 做好擅长的事");
    info!("   - 水平扩展 - 可以轻松添加新专家");
    info!("   - 质量保证 - 多层审核确保质量");
    info!("   - 灵活性强 - 动态调整专家组合\n");

    info!("2. 实现挑战:");
    info!("   - 任务分析准确性");
    info!("   - 专家间通信开销");
    info!("   - 结果冲突处理");
    info!("   - 性能优化\n");

    info!("3. 最佳实践:");
    info!("   - 明确定义专家职责");
    info!("   - 设计清晰的接口");
    info!("   - 实现有效的冲突解决");
    info!("   - 监控和优化性能\n");

    Ok(())
}
