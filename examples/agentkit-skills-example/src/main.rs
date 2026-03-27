//! Agent + Skills 简单示例
//!
//! 展示 Agent 如何自动调用 Skills 完成任务

use agentkit::skills::{SkillLoader, skills_to_tools, SkillExecutor};
use agentkit::provider::OpenAiProvider;
use agentkit::agent::DefaultAgent;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔═══════════════════════════════════════════════════════════╗");
    info!("║         Agent + Skills 示例                               ║");
    info!("╚═══════════════════════════════════════════════════════════╝\n");

    // 1. 加载 Skills
    info!("1. 加载 Skills...");
    let skills_dir = std::path::Path::new("skills");
    let mut loader = SkillLoader::new(skills_dir);
    let skills = loader.load_from_dir().await?;
    
    if skills.is_empty() {
        info!("⚠ 没有加载到 Skills");
        return Ok(());
    }
    
    info!("✓ 加载了 {} 个 Skills\n", skills.len());
    
    // 2. 检查 API Key
    if std::env::var("OPENAI_API_KEY").is_err() {
        info!("⚠ 未设置 OPENAI_API_KEY");
        info!("   请设置：export OPENAI_API_KEY=sk-your-key");
        return Ok(());
    }

    // 3. 创建 Agent 并注册 Skills
    info!("2. 创建 Agent 并注册 Skills...");
    let provider = OpenAiProvider::from_env()?;
    
    // 将 Skills 转换为 Tools 并注册到 Agent
    let executor = Arc::new(SkillExecutor::new());
    let tools = skills_to_tools(&skills, executor, skills_dir);
    
    info!("   转换了 {} 个 Tools", tools.len());
    
    // 显示已注册的 Tools
    info!("   已注册 Tools:");
    for tool in &tools {
        info!("     - {}", tool.name());
    }
    
    let agent = DefaultAgent::builder()
        .provider(provider)
        .model("qwen3.5:9b")
        .system_prompt("你是智能助手，可以使用工具帮助用户解决问题。")
        .tools(tools)  // 使用 tools() 方法批量注册
        .build();
    
    info!("✓ Agent 创建成功\n");

    // 4. 测试对话
    info!("3. 测试对话...\n");
    
    let queries = vec![
        "你好",
        "北京天气怎么样？",
    ];
    
    for query in queries {
        info!("用户：{}", query);
        
        let output: Result<_, _> = agent.run(query).await;
        match output {
            Ok(output) => {
                if let Some(text) = output.text() {
                    info!("助手：{}", text);
                }
            }
            Err(e) => {
                info!("错误：{}", e);
            }
        }
        info!("");
    }

    info!("示例完成！");
    
    Ok(())
}
