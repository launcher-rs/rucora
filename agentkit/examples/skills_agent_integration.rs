//! Skills 与 Agent 集成示例

use agentkit::skills::{SkillsAutoIntegrator, SkillLoader, SkillExecutor};
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
    info!("║    AgentKit Skills 使用示例                               ║");
    info!("╚═══════════════════════════════════════════════════════════╝\n");

    // 示例 1: 加载 Skills
    info!("=== 示例 1: 加载 Skills ===\n");
    test_load_skills().await?;

    // 示例 2: 分析工具需求
    info!("\n=== 示例 2: 分析工具需求 ===\n");
    test_analyze_tools().await?;

    // 示例 3: 执行 Weather Skill
    info!("\n=== 示例 3: 执行 Weather Skill ===\n");
    test_execute_skill().await?;

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
    
    let mut loader = SkillLoader::new(skills_dir);
    let skills = loader.load_from_dir().await?;
    
    info!("✓ 成功加载 {} 个 Skills\n", skills.len());
    
    // 显示 Skills 信息
    info!("已加载的 Skills:");
    for skill in &skills {
        info!("  - 名称：{}", skill.name);
        info!("    描述：{}", skill.description);
        info!("    版本：v{}", skill.version);
        info!("    超时：{}秒", skill.timeout);
        info!("");
    }
    
    Ok(())
}

/// 示例 2: 分析工具需求
async fn test_analyze_tools() -> anyhow::Result<()> {
    info!("1. 创建 SkillsAutoIntegrator...");
    
    let skills_dir = std::path::Path::new("skills");
    
    if !skills_dir.exists() {
        info!("⚠ Skills 目录不存在，跳过此示例");
        return Ok(());
    }
    
    let mut integrator = SkillsAutoIntegrator::new(skills_dir);
    let skills = integrator.load_and_analyze().await
        .map_err(|e| anyhow::anyhow!("加载 Skills 失败：{}", e))?;
    
    info!("✓ 加载了 {} 个 Skills", skills.len());
    
    // 显示检测到的工具
    let detected_tools = integrator.detected_tools();
    if !detected_tools.is_empty() {
        info!("\n检测到的工具需求:");
        for tool in detected_tools {
            info!("  - {}", tool);
        }
    } else {
        info!("\nℹ️  没有检测到外部工具需求");
    }
    
    Ok(())
}

/// 示例 3: 执行 Weather Skill
async fn test_execute_skill() -> anyhow::Result<()> {
    info!("1. 加载 Weather Skill...");
    
    let skills_dir = std::path::Path::new("skills");
    
    if !skills_dir.exists() {
        info!("⚠ Skills 目录不存在，跳过此示例");
        return Ok(());
    }
    
    let mut loader = SkillLoader::new(skills_dir);
    let skills = loader.load_from_dir().await?;
    
    // 查找 weather-query skill
    let weather_skill = skills.iter().find(|s| s.name == "weather-query");
    
    if let Some(skill) = weather_skill {
        info!("✓ 找到 weather-query skill");
        
        // 创建执行器
        let executor = SkillExecutor::new();
        
        // 准备输入
        let input = serde_json::json!({
            "city": "Beijing",
            "format": "simple"
        });
        
        info!("输入：{}", input);
        
        // 执行 skill
        info!("执行 skill...");
        let skill_path = skills_dir.join("weather");
        
        // 检查是否有 Python 实现
        let py_path = skill_path.join("SKILL.py");
        if py_path.exists() {
            info!("使用 Python 实现：SKILL.py");
            
            match executor.execute(skill, &skill_path, &input).await {
                Ok(result) => {
                    info!("✓ 执行完成 ({}ms)", result.execution_time_ms.unwrap_or(0));
                    info!("结果：{}", serde_json::to_string_pretty(&result.to_json())?);
                }
                Err(e) => {
                    info!("✗ 执行失败：{}", e);
                }
            }
        } else {
            info!("⚠ 未找到 SKILL.py 实现");
            info!("   提示：Skills 需要 Python/JavaScript/Shell 实现文件");
        }
    } else {
        info!("⚠ 未找到 weather-query skill");
    }
    
    Ok(())
}
