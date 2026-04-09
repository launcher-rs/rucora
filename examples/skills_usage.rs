//! Skills 使用示例
//!
//! 展示如何加载和执行 Skills，以及如何与 Agent/Runtime 集成
//!
//! # 运行方式
//!
//! ```bash
//! cargo run --example skills_usage -p agentkit
//! ```

use agentkit::skills::{SkillLoader, SkillExecutor};
use agentkit_providers::OpenAiProvider;
use agentkit::agent::DefaultAgent;
use agentkit::prelude::*;
use std::sync::Arc;
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

    info!("╔═══════════════════════════════════════════════════════════╗");
    info!("║         AgentKit Skills 使用示例                          ║");
    info!("╚═══════════════════════════════════════════════════════════╝\n");

    // 示例 1: 加载 Skills
    info!("=== 示例 1: 加载 Skills ===\n");
    test_load_skills().await?;

    // 示例 2: 执行 Skill
    info!("\n=== 示例 2: 执行 Skill ===\n");
    test_execute_skill().await?;

    // 示例 3: 与 Agent 集成
    info!("\n=== 示例 3: 与 Agent 集成 ===\n");
    test_agent_with_skills().await?;

    info!("\n=== 所有示例完成 ===");

    Ok(())
}

/// 示例 1: 加载 Skills
async fn test_load_skills() -> anyhow::Result<()> {
    info!("1. 创建 SkillLoader...");
    
    let skills_dir = std::path::Path::new("skills");
    
    if !skills_dir.exists() {
        info!("⚠ Skills 目录不存在：{:?}", skills_dir);
        info!("   请确保从项目根目录运行此示例");
        return Ok(());
    }
    
    // 创建加载器
    let mut loader = SkillLoader::new(skills_dir);
    
    // 加载所有 Skills
    let skills = loader.load_from_dir().await?;
    
    info!("✓ 成功加载 {} 个 Skills\n", skills.len());
    
    // 显示所有 Skills
    info!("已加载的 Skills:");
    for skill in &skills {
        info!("  - 名称：{}", skill.name);
        info!("    描述：{}", skill.description);
        info!("    版本：v{}", skill.version);
        info!("    超时：{}秒", skill.timeout);
        
        if !skill.tags.is_empty() {
            info!("    标签：{}", skill.tags.join(", "));
        }
        info!("");
    }
    
    // 显示 LLM 工具描述
    info!("LLM 工具描述:");
    let tool_descs = loader.to_tool_descriptions();
    for desc in &tool_descs {
        if let Some(name) = desc.get("function").and_then(|f| f.get("name")).and_then(|n| n.as_str()) {
            if let Some(desc_text) = desc.get("function").and_then(|f| f.get("description")).and_then(|d| d.as_str()) {
                info!("  - {}: {}", name, desc_text);
            }
        }
    }
    
    Ok(())
}

/// 示例 2: 执行 Skill
async fn test_execute_skill() -> anyhow::Result<()> {
    info!("1. 创建 SkillLoader 和 SkillExecutor...");
    
    let skills_dir = std::path::Path::new("skills");
    
    if !skills_dir.exists() {
        info!("⚠ Skills 目录不存在，跳过此示例");
        return Ok(());
    }
    
    let mut loader = SkillLoader::new(skills_dir);
    loader.load_from_dir().await?;
    
    let executor = SkillExecutor::new();
    
    // 查找 weather-query skill
    let skill = loader.get_skill("weather-query");
    
    if let Some(skill_def) = skill {
        info!("✓ 找到 skill: {}", skill_def.name);
        
        // 准备输入
        let input = serde_json::json!({
            "city": "Beijing",
            "format": "simple"
        });
        
        info!("输入：{}", input);
        
        // 执行 skill
        info!("执行 skill...");
        let skill_path = skills_dir.join("weather");
        
        match executor.execute(skill_def, &skill_path, &input).await {
            Ok(result) => {
                info!("✓ 执行完成 ({}ms)", result.execution_time_ms.unwrap_or(0));
                info!("结果：{}", serde_json::to_string_pretty(&result.to_json())?);
            }
            Err(e) => {
                info!("✗ 执行失败：{}", e);
            }
        }
    } else {
        info!("⚠ 未找到 weather-query skill");
    }
    
    Ok(())
}

/// 示例 3: 与 Agent 集成
async fn test_agent_with_skills() -> anyhow::Result<()> {
    info!("1. 检查环境变量...");
    
    if std::env::var("OPENAI_API_KEY").is_err() {
        info!("⚠ 未设置 OPENAI_API_KEY，跳过此示例");
        info!("   提示：export OPENAI_API_KEY=sk-your-key");
        return Ok(());
    }
    
    info!("✓ OPENAI_API_KEY 已设置");
    
    info!("\n2. 加载 Skills...");
    let skills_dir = std::path::Path::new("skills");
    
    if !skills_dir.exists() {
        info!("⚠ Skills 目录不存在，跳过此示例");
        return Ok(());
    }
    
    let mut loader = SkillLoader::new(skills_dir);
    let skills = loader.load_from_dir().await?;
    
    if skills.is_empty() {
        info!("⚠ 没有加载到任何 Skills");
        return Ok(());
    }
    
    info!("✓ 加载了 {} 个 Skills", skills.len());
    
    info!("\n3. 创建 Provider...");
    let provider = OpenAiProvider::from_env()?;
    info!("✓ Provider 创建成功");
    
    info!("\n4. 创建 Agent（注册 Skills）...");
    
    // 注意：当前 Agent API 可能需要调整以支持 Skills
    // 这里展示理想的使用方式
    let agent = DefaultAgent::builder()
        .provider(Arc::new(provider))
        .model("qwen3.5:9b")
        .system_prompt(
            "你是一个有用的助手。你可以使用以下技能：
            - weather-query: 查询天气
            根据用户请求选择合适的技能。"
        )
        .build();
    
    info!("✓ Agent 创建成功");
    
    info!("\n5. 测试 Agent...");
    
    let test_queries = vec![
        "北京天气怎么样？",
        "你好，请介绍一下自己",
    ];
    
    for query in test_queries {
        info!("用户：{}", query);
        
        match agent.run(query).await {
            Ok(output) => {
                if let Some(text) = output.text() {
                    info!("助手：{}", text.chars().take(100).collect::<String>());
                }
            }
            Err(e) => {
                info!("✗ 错误：{}", e);
            }
        }
        info!("");
    }
    
    Ok(())
}

