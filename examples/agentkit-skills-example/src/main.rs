//! Agent + Skills 完整示例
//!
//! 展示：
//! 1. Skills 加载
//! 2. Skills 转 Tools
//! 3. Agent 自动调用 Skills

use agentkit::agent::ToolAgent;
use agentkit::agent::ToolRegistry;
use agentkit::prelude::Agent;
use agentkit_providers::OpenAiProvider;
use agentkit_skills::{SkillExecutor, SkillLoader, SkillTool};
use agentkit_tools::system::ShellTool;
use std::sync::Arc;
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
    info!("║   AgentKit Skills 示例                ║");
    info!("╚════════════════════════════════════════╝\n");

    // 1. 加载 Skills
    info!("1. 加载 Skills...");
    let skills_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("skills");
    info!("   Skills 目录：{:?}", skills_dir);

    if !skills_dir.exists() {
        info!("⚠ Skills 目录不存在");
        return Ok(());
    }

    let mut loader = SkillLoader::new(&skills_dir);
    let skills = loader.load_from_dir().await?;

    if skills.is_empty() {
        info!("⚠ 没有加载到 Skills");
        return Ok(());
    }

    info!("✓ 加载了 {} 个 Skills\n", skills.len());

    // 显示加载的 Skills
    info!("已加载的 Skills:");
    for skill in &skills {
        info!("  - {}: {}", skill.name, skill.description);
    }
    info!("");

    // 2. 检查 API Key
    if std::env::var("OPENAI_API_KEY").is_err() && std::env::var("OPENAI_BASE_URL").is_err() {
        info!("⚠ 未设置 API 配置");
        info!("   使用 OpenAI: export OPENAI_API_KEY=sk-your-key");
        info!("   使用 Ollama: export OPENAI_BASE_URL=http://localhost:11434");
        return Ok(());
    }

    let model_name = std::env::var("MODEL_NAME").unwrap_or_else(|_| "qwen3.5:9b".to_string());

    // 3. 创建 Skill Executor
    info!("2. 创建 Skill Executor...");
    let skill_executor = Arc::new(SkillExecutor::new());
    info!("✓ Skill Executor 创建成功\n");

    // 4. 将 Skills 转换为 Tools 并注册
    info!("3. 注册 Skills 为 Tools...");
    let mut tool_registry = ToolRegistry::new();

    // 注册内置工具
    tool_registry = tool_registry.register(ShellTool::new());

    // 注册 Skills 转换的 Tools
    for skill in &skills {
        let skill_path = skills_dir.join(&skill.name);
        let skill_tool = SkillTool::new(skill.clone(), skill_executor.clone(), skill_path);
        tool_registry = tool_registry.register_arc(Arc::new(skill_tool));
        info!("  ✓ 注册技能：{}", skill.name);
    }
    info!("");

    // 5. 创建 Agent
    info!("4. 创建带 Skills 的 Agent...");
    let provider = OpenAiProvider::from_env()?;

    let agent = ToolAgent::builder()
        .provider(provider)
        .model(&model_name)
        .system_prompt(
            "你是一个有用的助手，可以使用各种技能帮助用户解决问题。\n\
             可用技能包括：\n\
             - datetime: 获取当前日期和时间\n\
             - calculator: 执行数学计算\n\
             - weather-query: 查询天气（需要提供城市英文名，如 Beijing）\n\n\
             请根据用户需求自动选择合适的技能，每个技能只调用一次即可。",
        )
        .tool_registry(tool_registry)
        .max_steps(15)
        // 配置 LLM 请求参数
        .temperature(0.7)
        .top_p(0.9)
        .max_tokens(2048)
        .build();

    info!("✓ Agent 创建成功\n");

    // 6. 测试对话
    info!("═══════════════════════════════════════");
    info!("测试 Skills");
    info!("═══════════════════════════════════════\n");

    let queries = vec![
        ("现在几点了？", "测试时间技能"),
        ("计算 10 + 20 * 3", "测试计算器技能"),
        ("北京天气怎么样？", "测试天气查询技能"),
    ];

    for (query, description) in queries {
        info!("测试：{} ({})", query, description);

        match agent.run(query.into()).await {
            Ok(output) => {
                if let Some(text) = output.text() {
                    info!("  助手：{}\n", text);
                }
            }
            Err(e) => {
                info!("  错误：{}\n", e);
            }
        }
    }

    info!("═══════════════════════════════════════");
    info!("示例完成！");
    info!("═══════════════════════════════════════\n");

    info!("📝 Skills 使用总结：\n");
    info!("1. Skills 加载:");
    info!("   - 使用 SkillLoader 从目录加载");
    info!("   - 自动识别 SKILL.md 配置文件\n");

    info!("2. Skills 转 Tools:");
    info!("   - 使用 SkillExecutor 创建工具");
    info!("   - 注册到 ToolRegistry\n");

    info!("3. Agent 使用:");
    info!("   - Agent 自动选择合适的技能");
    info!("   - 支持多步推理和技能组合\n");

    Ok(())
}
