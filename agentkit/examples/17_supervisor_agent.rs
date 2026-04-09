//! AgentKit Supervisor Agent 示例
//!
//! 展示主管模式的 Agent，协调多个专家 Agent 完成复杂任务。
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
//! 2. **任务分配** - 主管分析并分配任务给专家
//! 3. **结果聚合** - 汇总各专家的结果
//! 4. **实际演示** - 完成一个需要多专家协作的任务

use agentkit::agent::{SimpleAgent, ToolAgent};
use agentkit::prelude::Agent;
use agentkit::provider::OpenAiProvider;
use agentkit::tools::{EchoTool, ShellTool};
use std::sync::Arc;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

// ═══════════════════════════════════════════════════════════
// 简单的主管 Agent 实现
// ═══════════════════════════════════════════════════════════

/// 主管 Agent - 负责任务分析和分配
struct SupervisorAgent<P> {
    #[allow(dead_code)]
    provider: Arc<P>,
    #[allow(dead_code)]
    model: String,
    experts: Vec<ExpertInfo>,
}

/// 专家信息
#[derive(Clone)]
struct ExpertInfo {
    name: String,
    specialty: String,
    description: String,
}

impl<P> SupervisorAgent<P>
where
    P: agentkit_core::provider::LlmProvider + Send + Sync + 'static,
{
    /// 创建新的主管 Agent
    fn new(provider: Arc<P>, model: String) -> Self {
        Self {
            provider,
            model,
            experts: Vec::new(),
        }
    }

    /// 添加专家
    fn add_expert(mut self, name: String, specialty: String, description: String) -> Self {
        self.experts.push(ExpertInfo {
            name,
            specialty,
            description,
        });
        self
    }

    /// 分析任务并返回执行建议
    async fn analyze_task(&self, task: &str) -> String {
        // 简单实现：返回专家列表和任务分析
        format!(
            "任务分析：{}\n\n可用专家:\n{}",
            task,
            self.experts
                .iter()
                .map(|e| format!("  - {} ({}): {}", e.name, e.specialty, e.description))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

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
    // 创建主管 Agent
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("创建主管 Agent");
    info!("═══════════════════════════════════════\n");

    info!("1. 创建 Provider...");
    let provider = Arc::new(OpenAiProvider::from_env()?);
    info!("✓ Provider 创建成功\n");

    info!("2. 创建主管 Agent...");
    let supervisor = SupervisorAgent::new(provider.clone(), model_name.clone())
        .add_expert(
            "对话专家".to_string(),
            "自然语言处理".to_string(),
            "擅长自然流畅的对话、心理咨询、客服问答".to_string(),
        )
        .add_expert(
            "工具专家".to_string(),
            "系统操作".to_string(),
            "擅长使用 Shell、文件操作等工具完成任务".to_string(),
        )
        .add_expert(
            "分析专家".to_string(),
            "数据分析".to_string(),
            "擅长数据分析、统计、报告生成".to_string(),
        );

    info!("✓ 主管 Agent 创建成功");
    info!("  注册了 {} 个专家\n", supervisor.experts.len());

    // ═══════════════════════════════════════════════════════════
    // 创建专家 Agent
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("创建专家 Agent");
    info!("═══════════════════════════════════════\n");

    // 专家 1: 对话专家
    info!("1. 创建对话专家 (SimpleAgent)...");
    let provider1 = OpenAiProvider::from_env()?;
    let chat_expert = SimpleAgent::builder()
        .provider(provider1)
        .model(&model_name)
        .system_prompt(
            "你是对话专家，擅长：\n\
             - 自然流畅的对话\n\
             - 心理咨询和倾听\n\
             - 客服问答\n\
             请用友好、专业的语气回答。",
        )
        .build();
    info!("   ✓ 对话专家创建成功\n");

    // 专家 2: 工具专家
    info!("2. 创建工具专家 (ToolAgent)...");
    let provider2 = OpenAiProvider::from_env()?;
    let tool_expert = ToolAgent::builder()
        .provider(provider2)
        .model(&model_name)
        .system_prompt(
            "你是工具专家，擅长：\n\
             - 使用 Shell 命令执行系统操作\n\
             - 文件读写和管理\n\
             - 执行具体任务\n\
             请准确选择合适的工具完成任务。",
        )
        .tool(ShellTool::new())
        .tool(EchoTool)
        .max_steps(10)
        .build();
    info!("   ✓ 工具专家创建成功\n");

    // 专家 3: 分析专家
    info!("3. 创建分析专家 (SimpleAgent)...");
    let provider3 = OpenAiProvider::from_env()?;
    let analysis_expert = SimpleAgent::builder()
        .provider(provider3)
        .model(&model_name)
        .system_prompt(
            "你是分析专家，擅长：\n\
             - 数据分析和统计\n\
             - 报告生成\n\
             - 信息整理和总结\n\
             请提供清晰、结构化的分析结果。",
        )
        .build();
    info!("   ✓ 分析专家创建成功\n");

    // ═══════════════════════════════════════════════════════════
    // Supervisor 模式架构
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("Supervisor 模式架构:");
    info!("═══════════════════════════════════════\n");

    info!("```");
    info!("                    ┌─────────────┐");
    info!("                    │ Supervisor  │");
    info!("                    │   (主管)    │");
    info!("                    └──────┬──────┘");
    info!("                           │");
    info!("          ┌────────────────┼────────────────┐");
    info!("          │                │                │");
    info!("          ▼                ▼                ▼");
    info!("   ┌──────────┐    ┌──────────┐    ┌──────────┐");
    info!("   │  Chat    │    │  Tool    │    │Analysis  │");
    info!("   │  Expert  │    │  Expert  │    │  Expert  │");
    info!("   │ (对话)    │    │ (工具)    │    │ (分析)    │");
    info!("   └──────────┘    └──────────┘    └──────────┘");
    info!("```\n");

    // ═══════════════════════════════════════════════════════════
    // 演示任务 1: 主管分析任务
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("演示任务 1: 主管分析任务");
    info!("═══════════════════════════════════════\n");

    let task1 = "帮我分析当前目录的文件结构";
    info!("任务：\"{}\"\n", task1);

    let analysis = supervisor.analyze_task(task1).await;
    info!("主管分析:\n{}\n", analysis);

    // ═══════════════════════════════════════════════════════════
    // 演示任务 2: 工具专家执行任务
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("演示任务 2: 工具专家执行任务");
    info!("═══════════════════════════════════════\n");

    info!("主管分配任务给工具专家...\n");

    let tool_task = "列出当前目录的所有文件和子目录，使用 dir 命令";
    info!("工具专家收到任务：\"{}\"\n", tool_task);

    match tool_expert.run(tool_task.into()).await {
        Ok(output) => {
            let output: agentkit_core::agent::AgentOutput = output;
            if let Some(text) = output.text() {
                info!("工具专家执行结果:\n{}\n", text);
            }
        }
        Err(e) => {
            info!("工具专家执行失败：{}\n", e);
        }
    }

    // ═══════════════════════════════════════════════════════════
    // 演示任务 3: 分析专家总结结果
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("演示任务 3: 分析专家总结结果");
    info!("═══════════════════════════════════════\n");

    info!("主管分配任务给分析专家...\n");

    let analysis_task = "根据以下文件列表，分析项目结构特点：\n\
                         - 这是一个 Rust 项目\n\
                         - 包含多个子 crate\n\
                         - 有完整的示例代码\n\
                         请总结项目特点。";
    info!("分析专家收到任务：\"{}\"\n", analysis_task);

    match analysis_expert.run(analysis_task.into()).await {
        Ok(output) => {
            let output: agentkit_core::agent::AgentOutput = output;
            if let Some(text) = output.text() {
                info!("分析专家总结:\n{}\n", text);
            }
        }
        Err(e) => {
            info!("分析专家执行失败：{}\n", e);
        }
    }

    // ═══════════════════════════════════════════════════════════
    // 演示任务 4: 对话专家进行用户交互
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("演示任务 4: 对话专家进行用户交互");
    info!("═══════════════════════════════════════\n");

    info!("主管分配任务给对话专家...\n");

    let chat_task = "用户询问：'这个项目是做什么的？请用简单易懂的方式解释。'";
    info!("对话专家收到任务：\"{}\"\n", chat_task);

    match chat_expert.run(chat_task.into()).await {
        Ok(output) => {
            let output: agentkit_core::agent::AgentOutput = output;
            if let Some(text) = output.text() {
                info!("对话专家回复:\n{}\n", text);
            }
        }
        Err(e) => {
            info!("对话专家执行失败：{}\n", e);
        }
    }

    // ═══════════════════════════════════════════════════════════
    // Supervisor 工作流程总结
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("Supervisor 工作流程总结:");
    info!("═══════════════════════════════════════\n");

    info!("1. 接收用户任务");
    info!("2. 分析任务需求，识别需要的技能");
    info!("3. 分配合适的专家 Agent 执行");
    info!("4. 收集各专家结果");
    info!("5. 聚合结果并返回给用户\n");

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

    info!("4. 本示例演示:");
    info!("   - 创建主管 Agent 管理专家");
    info!("   - 注册 3 个不同领域的专家");
    info!("   - 主管分析任务并分配");
    info!("   - 各专家执行专业任务");
    info!("   - 展示完整的多 Agent 协作流程\n");

    Ok(())
}
