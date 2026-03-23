//! 深度研究示例 - 带配置管理

mod config;

use agentkit::prelude::*;
use agentkit::provider::OpenAiProvider;
use agentkit::runtime::{DefaultRuntime, ToolRegistry};
use agentkit::tools::{DatetimeTool, FileWriteTool, SerpapiTool, WebScraperTool, WebSearchTool};
use chrono::Local;
use config::{AppConfig, ProviderType};
use serde_json::json;
use std::io::{self, Write};
use std::sync::Arc;
use tracing::{info, Level};
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
            print!("\n是否使用现有配置？(Y/n): ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if input.trim().to_lowercase() != "n" {
                return Ok(config);
            }
        }
    }

    // 交互式配置
    println!("\n━━━ 配置向导 ━━━\n");

    // 选择 Provider
    println!("选择 Provider:");
    for (i, provider) in ProviderType::all().iter().enumerate() {
        println!(
            "  {}. {} (默认模型：{})",
            i + 1,
            provider.name(),
            provider.default_model()
        );
    }

    print!("\n请输入 Provider 编号 (1-{}): ", ProviderType::all().len());
    io::stdout().flush()?;
    let mut provider_input = String::new();
    io::stdin().read_line(&mut provider_input)?;
    let provider_idx = provider_input
        .trim()
        .parse::<usize>()
        .unwrap_or(1)
        .max(1)
        .min(ProviderType::all().len())
        - 1;
    let selected_provider = ProviderType::all()[provider_idx].clone();

    // 输入 API Key
    print!("\n输入 API Key (或 URL for Ollama): ");
    io::stdout().flush()?;
    let mut api_key = String::new();
    io::stdin().read_line(&mut api_key)?;
    api_key = api_key.trim().to_string();

    // 输入模型名称（可选，使用默认值）
    print!(
        "输入模型名称 (留空使用默认：{}): ",
        selected_provider.default_model()
    );
    io::stdout().flush()?;
    let mut model = String::new();
    io::stdin().read_line(&mut model)?;
    model = model.trim().to_string();
    if model.is_empty() {
        model = selected_provider.default_model().to_string();
    }

    // 输入 SerpAPI Keys（可选）
    print!("\n输入 SerpAPI Keys (可选，逗号分隔，留空跳过): ");
    io::stdout().flush()?;
    let mut serpapi_keys = String::new();
    io::stdin().read_line(&mut serpapi_keys)?;
    serpapi_keys = serpapi_keys.trim().to_string();

    let config = AppConfig {
        provider: Some(selected_provider.name().to_string()),
        api_key: Some(api_key),
        model: Some(model),
        base_url: None,
        serpapi_keys: if serpapi_keys.is_empty() {
            None
        } else {
            Some(serpapi_keys)
        },
    };

    // 保存配置
    config.save()?;
    println!("✓ 配置已保存到 ~/.agentkit/config.toml");

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

    let provider: Arc<dyn agentkit::core::provider::LlmProvider + Send + Sync> = Arc::new(
        OpenAiProvider::new("https://api.openai.com/v1", api_key.clone()),
    );

    info!("✓ Provider 初始化成功：OpenAI ({})", model);
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
    let _runtime = DefaultRuntime::new(provider, tools)
        .with_system_prompt("你是一个专业的深度研究助手。请制定详细研究计划，多轮迭代收集信息，分析整理，最后生成完整报告。")
        .with_max_steps(15);

    // 简化的研究流程（实际应该更复杂）
    // TODO: 使用 runtime 执行实际的研究任务
    info!("\n【研究开始】");

    // 第 1 轮：基础信息收集
    info!("\n━━━ 第 1 轮研究 ━━━");
    let queries = vec![
        format!("{} 基本原理", topic),
        format!("{} 应用场景", topic),
        format!("{} 发展趋势", topic),
    ];

    let mut collected_info = Vec::new();
    for query in &queries {
        info!("  搜索：{}", query);
        let search_tool = WebSearchTool::new().with_max_results(3);
        if let Ok(result) = search_tool.call(json!({"query": query})).await {
            if let Some(results) = result.get("results").and_then(|v| v.as_array()) {
                for item in results {
                    if let (Some(title), Some(snippet)) = (
                        item.get("title").and_then(|v| v.as_str()),
                        item.get("snippet").and_then(|v| v.as_str()),
                    ) {
                        collected_info.push(format!("[{}] {}", title, snippet));
                    }
                }
            }
        }
    }

    info!("✓ 收集到 {} 条信息", collected_info.len());

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

## 执行摘要

本报告对"{}"进行了初步研究。通过联网搜索收集了相关信息。

## 信息来源

{}

## 结论

建议进一步深入研究该主题。

---
*报告生成时间：{}*
*本研究报告由 AgentKit 深度研究助手自动生成*
"#,
        topic,
        Local::now().format("%Y 年%m 月%d 日"),
        topic,
        collected_info
            .iter()
            .map(|i| format!("- {}", i))
            .collect::<Vec<_>>()
            .join("\n"),
        Local::now().format("%Y-%m-%d %H:%M:%S"),
    );

    // 保存报告
    let filename = format!("research_report_{}.md", safe_filename);
    std::fs::write(&filename, &report)?;

    info!("\n✓ 报告已保存到：{}", filename);
    info!("\n=== 研究完成 ===");

    Ok(())
}
