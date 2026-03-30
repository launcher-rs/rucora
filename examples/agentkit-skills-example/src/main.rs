//! Agent + Skills 完整示例
//!
//! 展示：
//! 1. Skills 加载
//! 2. Full/Compact 两种提示词模式
//! 3. read_skill 工具的使用
//! 4. Agent 自动调用 Skills

use agentkit::provider::OpenAiProvider;
use agentkit::runtime::{DefaultRuntime, ToolRegistry, ToolSource};
use agentkit::runtime::tool_registry::ToolWrapper;
use agentkit::skills::{SkillExecutor, SkillLoader, ReadSkillTool, SkillsPromptMode, skills_to_prompt_with_mode, skills_to_tools};
use agentkit::tools::{CmdExecTool, HttpRequestTool, ShellTool};
use agentkit_core::agent::AgentInput;
use agentkit_core::runtime::Runtime;
use agentkit_core::tool::Tool;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔═══════════════════════════════════════════════════════════╗");
    info!("║         Agent + Skills 完整示例                           ║");
    info!("╚═══════════════════════════════════════════════════════════╝\n");

    // 1. 加载 Skills（使用当前目录的 skills 文件夹）
    info!("1. 加载 Skills...");
    // 使用相对于可执行文件的 skills 目录
    let skills_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("skills");

    // 如果当前目录没有 skills，尝试上级目录
    if !skills_dir.exists() {
        info!("   当前目录未找到 skills，尝试上级目录...");
    }
    
    info!("   Skills 目录：{:?}", skills_dir);
    
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
    if std::env::var("OPENAI_API_KEY").is_err() {
        info!("⚠ 未设置 OPENAI_API_KEY");
        info!("   请设置：export OPENAI_API_KEY=sk-your-key");
        info!("   或者使用 Ollama: export OPENAI_BASE_URL=http://localhost:11434");
        return Ok(());
    }

    // 3. 演示 Full 模式
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("示例 A: Full 模式（包含所有详细信息）");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    run_full_mode(&skills_dir, &skills).await?;

    // 4. 演示 Compact 模式
    info!("");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("示例 B: Compact 模式（简洁模式 + read_skill 工具）");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    run_compact_mode(&skills_dir, &skills).await?;

    info!("");
    info!("╔═══════════════════════════════════════════════════════════╗");
    info!("║              所有示例运行完成！                            ║");
    info!("╚═══════════════════════════════════════════════════════════╝");

    Ok(())
}

/// 运行 Full 模式示例
async fn run_full_mode(skills_dir: &std::path::Path, skills: &[agentkit::skills::SkillDefinition]) -> anyhow::Result<()> {
    info!("1. 构建 Full 模式系统提示词...");
    let workspace_dir = std::env::current_dir().unwrap_or_default();
    let skills_prompt = skills_to_prompt_with_mode(skills, &workspace_dir, SkillsPromptMode::Full);
    info!("✓ 提示词长度：{} 字符\n", skills_prompt.len());
    
    // 显示提示词预览
    info!("提示词预览:");
    for line in skills_prompt.lines().take(15) {
        info!("  {}", line);
    }
    info!("  ...\n");

    // 创建 Runtime
    info!("2. 创建 Runtime (Full 模式)...");
    let provider = OpenAiProvider::from_env()?;
    let executor = Arc::new(SkillExecutor::new());
    let skill_tools = skills_to_tools(skills, executor, skills_dir);

    let mut registry = ToolRegistry::new();
    registry = registry.register_wrapper(ToolWrapper::new(CmdExecTool::new()).with_source(ToolSource::BuiltIn));
    registry = registry.register_wrapper(ToolWrapper::new(ShellTool::new()).with_source(ToolSource::BuiltIn));
    registry = registry.register_wrapper(ToolWrapper::new(HttpRequestTool::new()).with_source(ToolSource::BuiltIn));
    for tool in skill_tools {
        registry = registry.register_wrapper(ToolWrapper::new_arc(tool).with_source(ToolSource::Skill));
    }
    
    let system_prompt = format!(
        "你是智能助手，可以使用工具帮助用户解决问题。\n\n\
         可用技能已预加载在系统提示词中，请直接使用这些技能。\
         {}",
        skills_prompt
    );
    
    let runtime = DefaultRuntime::new(Arc::new(provider), registry, "qwen3.5:9b")
        .with_system_prompt(system_prompt)
        .with_max_steps(10)
        .with_max_tool_concurrency(2);

    info!("✓ Runtime 创建成功\n");

    // 测试对话
    info!("3. 测试对话 (Full 模式)...");
    let query = "你好，请介绍一下自己";
    info!("用户：{}\n", query);
    
    let input = AgentInput::new(query);
    match runtime.run(input).await {
        Ok(output) => {
            if let Some(text) = output.text() {
                info!("助手：{}\n", text);
            }
        }
        Err(e) => {
            info!("错误：{}\n", e);
        }
    }

    Ok(())
}

/// 运行 Compact 模式示例
async fn run_compact_mode(skills_dir: &std::path::Path, skills: &[agentkit::skills::SkillDefinition]) -> anyhow::Result<()> {
    info!("1. 构建 Compact 模式系统提示词...");
    let workspace_dir = std::env::current_dir().unwrap_or_default();
    let skills_prompt = skills_to_prompt_with_mode(skills, &workspace_dir, SkillsPromptMode::Compact);
    info!("✓ 提示词长度：{} 字符\n", skills_prompt.len());
    
    // 显示提示词预览
    info!("提示词预览:");
    for line in skills_prompt.lines().take(10) {
        info!("  {}", line);
    }
    info!("  ...\n");

    // 创建 Runtime
    info!("2. 创建 Runtime (Compact 模式)...");
    let provider = OpenAiProvider::from_env()?;
    let executor = Arc::new(SkillExecutor::new());
    let skill_tools = skills_to_tools(skills, executor, skills_dir);

    let mut registry = ToolRegistry::new();
    registry = registry.register_wrapper(ToolWrapper::new(CmdExecTool::new()).with_source(ToolSource::BuiltIn));
    registry = registry.register_wrapper(ToolWrapper::new(ShellTool::new()).with_source(ToolSource::BuiltIn));
    registry = registry.register_wrapper(ToolWrapper::new(HttpRequestTool::new()).with_source(ToolSource::BuiltIn));
    for tool in skill_tools {
        registry = registry.register_wrapper(ToolWrapper::new_arc(tool).with_source(ToolSource::Skill));
    }
    registry = registry.register_wrapper(
        ToolWrapper::new(ReadSkillTool::new(skills_dir.to_path_buf())).with_source(ToolSource::Skill)
    );
    info!("   注册了 {} 个 Tools (包括 read_skill)", registry.enabled_len());
    
    let system_prompt = format!(
        "你是智能助手，可以使用工具帮助用户解决问题。\n\n\
         {}",
        skills_prompt
    );
    
    let runtime = DefaultRuntime::new(Arc::new(provider), registry, "qwen3.5:9b")
        .with_system_prompt(system_prompt)
        .with_max_steps(10)
        .with_max_tool_concurrency(2);

    info!("✓ Runtime 创建成功\n");

    // 测试对话
    info!("3. 测试对话 (Compact 模式)...");
    
    let queries = vec![
        "现在几点了",
        "北京天气怎么样？",
    ];
    
    for query in queries {
        info!("用户：{}", query);
        
        let input = AgentInput::new(query);
        match runtime.run(input).await {
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

    // 演示 read_skill 工具
    info!("4. 演示 read_skill 工具...");
    if let Some(first_skill) = skills.first() {
        info!("读取 '{}' 技能的详细信息...\n", first_skill.name);
        
        let read_skill_tool = ReadSkillTool::new(skills_dir.to_path_buf());
        let input = serde_json::json!({
            "skill_name": first_skill.name
        });
        
        let result: Result<serde_json::Value, _> = read_skill_tool.call(input).await;
        match result {
            Ok(result) => {
                info!("✓ 读取成功:");
                if let Some(content) = result.get("content").and_then(|v: &serde_json::Value| v.as_str()) {
                    // 显示前 500 字符
                    let preview: String = content.chars().take(500).collect();
                    info!("  内容预览:\n  {}", preview);
                    if content.len() > 500 {
                        info!("  ... (内容过长，已截断)");
                    }
                }
            }
            Err(e) => {
                info!("✗ 读取失败：{}", e);
            }
        }
    }

    Ok(())
}
