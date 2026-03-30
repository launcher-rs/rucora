//! AgentKit Skills（技能）示例
//!
//! 展示如何使用技能系统。
//!
//! ## 运行方法
//! ```bash
//! cargo run --example 14_skills --features skills
//! ```

#[cfg(feature = "skills")]
use agentkit::skills::{SkillExecutor, SkillLoader};
#[cfg(feature = "skills")]
use tracing::{Level, info};
#[cfg(feature = "skills")]
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
#[cfg(feature = "skills")]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔════════════════════════════════════════╗");
    info!("║   AgentKit Skills 示例                ║");
    info!("╚════════════════════════════════════════╝\n");

    info!("Skills（技能）系统允许你通过配置文件定义可复用任务。\n");

    info!("═══════════════════════════════════════");
    info!("技能系统功能:");
    info!("═══════════════════════════════════════");
    info!("1. YAML 配置 - 通过 YAML 文件定义技能");
    info!("2. Rhai 脚本 - 支持 Rhai 脚本编写复杂逻辑");
    info!("3. 工具转换 - 技能可以转换为工具");
    info!("4. 热加载 - 动态加载和卸载技能");
    info!("═══════════════════════════════════════\n");

    info!("技能配置示例 (skills/greeting.yaml):");
    info!(
        "```yaml
name: greeting
description: 问候技能
version: 1.0
inputs:
  - name: user_name
    type: string
    required: true
    description: 用户姓名
steps:
  - type: template
    template: \"你好，{{user_name}}！欢迎使用 AgentKit。\"
```"
    );
    info!("```\n");

    // 创建技能执行器
    let _executor = SkillExecutor::new();
    info!("✓ 技能执行器创建成功\n");

    info!("提示：技能功能需要启用 skills feature");
    info!("运行：cargo run --example 14_skills --features skills\n");

    info!("示例完成！");

    Ok(())
}

#[cfg(not(feature = "skills"))]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Skills feature 未启用");
    println!("请使用以下命令运行:");
    println!("  cargo run --example 14_skills --features skills");
    Ok(())
}
