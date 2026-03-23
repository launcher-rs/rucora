//! 深度研究示例 - 带配置管理

mod config;

use agentkit::prelude::*;
use agentkit::provider::OpenAiProvider;
use agentkit::runtime::{DefaultRuntime, ToolRegistry};
use agentkit::tools::{DatetimeTool, FileWriteTool, SerpapiTool, WebScraperTool, WebSearchTool};
use chrono::Local;
use config::{AppConfig, ProviderType};
use console::style;
use dialoguer::{Input, Select};
use std::io::{self, Write};
use std::sync::Arc;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    println!("╔════════════════════════════════════════════════════════╗");
    println!("║         AgentKit 深度研究助手                          ║");
    println!("╚════════════════════════════════════════════════════════╝\n");

    // 加载或创建配置
    let config = load_or_create_config()?;

    // 创建 Provider
    let provider = create_provider(&config)?;

    // 获取研究主题
    println!("\n请输入您要研究的主题（输入 'q' 退出）：");
    print!("> ");
    io::stdout().flush()?;

    let mut topic = String::new();
    io::stdin().read_line(&mut topic)?;
    topic = topic.trim().to_string();

    if topic.is_empty() || topic.to_lowercase() == "q" {
        info!("已退出");
        return Ok(());
    }

    info!("\n开始研究：{}\n", topic);

    // 执行研究
    run_research(provider, &topic, &config)
}

/// 加载或创建配置
fn load_or_create_config() -> anyhow::Result<AppConfig> {
    // 尝试加载现有配置
    if let Some(config) = AppConfig::load() {
        if config.is_complete() {
            println!("✓ 已加载现有配置");
            println!("  Provider: {}", config.provider.as_ref().unwrap());
            println!("  模型：{}", config.model.as_ref().unwrap());

            // 询问是否使用现有配置
            let use_existing = dialoguer::Confirm::new()
                .with_prompt("是否使用现有配置？")
                .default(true)
                .interact()?;

            if use_existing {
                return Ok(config);
            }
        }
    }

    // 交互式配置
    println!("\n{}", style("━━━ 配置向导 ━━━").blue().bold());

    // 选择 Provider - 使用 Select 组件
    let provider_names: Vec<String> = ProviderType::all()
        .iter()
        .map(|p| format!("{} - {}", p.name(), p.default_model()))
        .collect();

    let selected_idx = Select::new()
        .with_prompt("选择 Provider")
        .items(&provider_names)
        .default(0)
        .interact()?;

    let selected_provider = ProviderType::all()[selected_idx].clone();

    // 输入 API Key - 使用 Input 组件
    let api_key: String = Input::new().with_prompt("输入 API Key").interact_text()?;

    // 输入模型名称 - 使用 Input 组件，带默认值
    let model: String = Input::new()
        .with_prompt("输入模型名称")
        .default(selected_provider.default_model().to_string())
        .interact_text()?;

    // 输入 base_url（可选，所有 Provider 都允许自定义）
    let mut base_url: Option<String> = None;
    if selected_provider.show_base_url_input() {
        let default_url = selected_provider.default_base_url().unwrap_or("");
        if !default_url.is_empty() {
            let url_input: String = Input::new()
                .with_prompt("输入 Base URL")
                .default(default_url.to_string())
                // .with_initial_text("按 Enter 使用默认值")
                .interact_text()?;
            // 如果用户输入了不同的值，使用用户输入；否则使用默认值
            if url_input.is_empty() {
                base_url = Some(default_url.to_string());
            } else if url_input != default_url {
                base_url = Some(url_input);
            } else {
                base_url = Some(default_url.to_string());
            }
        } else {
            let url_input: String = Input::new()
                .with_prompt("输入 Base URL (可选)")
                .allow_empty(true)
                .with_initial_text("留空使用标准 API 端点")
                .interact_text()?;
            if !url_input.is_empty() {
                base_url = Some(url_input);
            }
        }
    }

    // 输入 SerpAPI Keys（可选）
    println!();
    let serpapi_keys: String = Input::new()
        .with_prompt("输入 SerpAPI Keys (可选，逗号分隔)")
        .allow_empty(true)
        .interact_text()?;

    let config = AppConfig {
        provider: Some(selected_provider.name().to_string()),
        api_key: Some(api_key),
        model: Some(model),
        base_url,
        serpapi_keys: if serpapi_keys.is_empty() {
            None
        } else {
            Some(serpapi_keys)
        },
    };

    // 保存配置
    config.save()?;
    println!(
        "✓ 配置已保存到 {}",
        AppConfig::config_path().unwrap().display()
    );

    Ok(config)
}

/// 创建 Provider
fn create_provider(
    config: &AppConfig,
) -> anyhow::Result<Arc<dyn agentkit::core::provider::LlmProvider + Send + Sync>> {
    let api_key = config
        .api_key
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("缺少 API Key"))?;
    let model = config
        .model
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("缺少模型配置"))?;
    let base_url = config
        .base_url
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or("https://api.openai.com/v1");

    let provider: Arc<dyn agentkit::core::provider::LlmProvider + Send + Sync> =
        Arc::new(OpenAiProvider::new(base_url, api_key.clone()).with_default_model(model.clone()));

    info!("✓ Provider 初始化成功：OpenAI ({})", model);
    if let Some(ref url) = config.base_url {
        info!("  Base URL: {}", url);
    }
    Ok(provider)
}

/// 执行研究
#[tokio::main]
async fn run_research(
    provider: Arc<dyn agentkit::core::provider::LlmProvider + Send + Sync>,
    topic: &str,
    config: &AppConfig,
) -> anyhow::Result<()> {
    // 检查 SerpAPI
    let has_serpapi = config.serpapi_keys.is_some()
        || std::env::var("SERPAPI_API_KEYS").is_ok()
        || std::env::var("SERPAPI_API_KEY").is_ok();

    // 创建工具注册表
    let mut tools = ToolRegistry::new()
        .register(WebSearchTool::new().with_max_results(5))
        .register(WebScraperTool::new())
        .register(DatetimeTool::new())
        .register(FileWriteTool::new());

    // 添加 SerpAPI
    if has_serpapi {
        if let Ok(tool) = SerpapiTool::from_env() {
            tools = tools.register(tool);
            info!("✓ SerpAPI 工具已加载");
        }
    }

    // 创建运行时
    let runtime = DefaultRuntime::new(provider, tools)
        .with_system_prompt("你是一个专业的深度研究助手。请制定详细研究计划，多轮迭代收集信息，分析整理，最后生成完整报告。")
        .with_max_steps(15);

    info!("\n【研究开始】");

    // 第 1 轮：使用 runtime 执行研究
    info!("\n━━━ 第 1 轮研究 ━━━");

    // 构建研究问题
    let research_prompt = format!(
        "请研究以下主题：{}\n\n\
         请从以下几个方面进行研究：\n\
         1. 主题的基本情况和背景\n\
         2. 主要影响因素和相关方\n\
         3. 可能的影响和后果\n\
         4. 未来发展趋势\n\n\
         请提供详细、客观的分析。",
        topic
    );

    let input = AgentInput::new(research_prompt);

    // 使用 runtime 执行研究
    info!("  正在执行研究...");
    if let Some(ref url) = config.base_url {
        info!("  Base URL: {}", url);
    }
    if let Some(ref m) = config.model {
        info!("  Model: {}", m);
    }
    match runtime.run(input).await {
        Ok(output) => {
            if let Some(content) = output.text() {
                info!("✓ 研究完成");

                // 生成报告
                let safe_filename = topic
                    .chars()
                    .map(|c| {
                        if c.is_alphanumeric() || c == '-' || c == '_' {
                            c
                        } else {
                            '_'
                        }
                    })
                    .collect::<String>();

                let report = format!(
                    r#"# {} 深度研究报告

**研究日期**: {}
**研究轮数**: 1

## 研究结果

{}

---
*报告生成时间：{}*
*本研究报告由 AgentKit 深度研究助手自动生成*
"#,
                    topic,
                    Local::now().format("%Y 年%m 月%d 日"),
                    content,
                    Local::now().format("%Y-%m-%d %H:%M:%S"),
                );

                // 保存报告
                let filename = format!("research_report_{}.md", safe_filename);
                std::fs::write(&filename, &report)?;

                info!("\n✓ 报告已保存到：{}", filename);
            }
        }
        Err(e) => {
            info!("❌ 研究失败：{}", e);
            info!("\n可能的原因：");
            info!("  1. API Key 无效或过期");
            info!("  2. Base URL 不正确");
            info!("  3. Model 名称不正确");
            info!("  4. 网络连接问题");
            info!("\n请检查配置并重试。");
            info!("提示：可以使用 `cargo run -p agentkit-deep-research` 重新配置");
        }
    }

    info!("\n=== 研究完成 ===");

    Ok(())
}
