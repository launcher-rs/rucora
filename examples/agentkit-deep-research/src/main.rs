//! 深度研究示例 - 带配置管理和多轮迭代研究
//!
//! 功能：
//! 1. 配置管理 - 支持多种 LLM Provider
//! 2. 多轮迭代研究 - 自动制定研究计划并执行
//! 3. 工具自动调用 - Web 搜索、网页抓取等
//! 4. 报告生成 - 生成结构化 Markdown 报告

mod config;

use agentkit::agent::ToolRegistry;
use agentkit::agent::execution::DefaultExecution;
use agentkit::prelude::*;
use agentkit_providers::OpenAiProvider;
use agentkit_tools::{DatetimeTool, FileWriteTool, SerpapiTool, WebScraperTool, WebSearchTool};
use agentkit_core::agent::{Agent, AgentContext, AgentDecision};
use chrono::Local;
use config::{AppConfig, ProviderType};
use console::style;
use dialoguer::{Input, Select};
use std::io::{self, Write};
use std::sync::Arc;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

// 简单的 Dummy Agent 用于执行
struct DummyAgent;

#[async_trait::async_trait]
impl Agent for DummyAgent {
    async fn think(&self, context: &AgentContext) -> AgentDecision {
        AgentDecision::Chat {
            request: context.default_chat_request(),
        }
    }

    fn name(&self) -> &str {
        "research_agent"
    }

    fn description(&self) -> Option<&str> {
        Some("深度研究助手")
    }
}

fn main() -> anyhow::Result<()> {
    // 加载 .env 文件
    dotenv::dotenv().ok();

    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    println!("╔════════════════════════════════════════════════════════╗");
    println!("║         AgentKit 深度研究助手                          ║");
    println!("╚════════════════════════════════════════════════════════╝\n");

    // 加载或创建配置
    let config = load_or_create_config()?;

    // 显示当前配置
    config.display();

    // 创建 Provider
    let provider = create_provider(&config)?;

    // 获取研究主题
    println!("\n{}", style("━━━ 研究主题 ━━━").green().bold());
    println!("请输入您要研究的主题（输入 'q' 退出）：");
    print!("> ");
    io::stdout().flush()?;

    let mut topic = String::new();
    io::stdin().read_line(&mut topic)?;
    topic = topic.trim().to_string();

    if topic.is_empty() || topic.to_lowercase() == "q" {
        info!("已退出");
        return Ok(());
    }

    info!("\n{}", style(format!("开始研究：{}", topic)).bold());

    // 执行研究
    run_research(provider, &topic, &config)
}

/// 加载或创建配置
fn load_or_create_config() -> anyhow::Result<AppConfig> {
    // 尝试加载现有配置（优先环境变量，其次配置文件）
    if let Some(config) = AppConfig::load() {
        if config.is_complete() {
            // 如果是从环境变量加载的，直接返回
            if AppConfig::from_env().is_some() {
                println!("✓ 已从环境变量加载配置");
                return Ok(config);
            }

            // 从配置文件加载的，询问是否使用
            println!("✓ 已加载现有配置");
            println!(
                "  Provider: {}",
                config.provider.as_ref().unwrap_or(&"未指定".to_string())
            );
            println!(
                "  模型：{}",
                config.model.as_ref().unwrap_or(&"未指定".to_string())
            );

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

    // 输入 base_url（可选，所有 Provider 都允许自定义）
    let mut base_url: Option<String> = None;
    if selected_provider.show_base_url_input() {
        let default_url = selected_provider.default_base_url().unwrap_or("");
        if !default_url.is_empty() {
            let url_input: String = Input::new()
                .with_prompt("输入 Base URL")
                .default(default_url.to_string())
                .interact_text()?;
            base_url = Some(url_input);
        } else {
            let url_input: String = Input::new()
                .with_prompt("输入 Base URL (可选)")
                .allow_empty(true)
                .interact_text()?;
            if !url_input.is_empty() {
                base_url = Some(url_input);
            }
        }
    }

    // 输入模型名称 - 使用 Input 组件，带默认值
    let default_model = selected_provider.default_model();
    let model: String = Input::new()
        .with_prompt("输入模型名称")
        .default(default_model.to_string())
        .interact_text()?;

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

    info!(
        "✓ Provider 初始化成功：{}",
        config.provider.as_ref().unwrap_or(&"OpenAI".to_string())
    );
    info!("  模型：{}", model);
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

    // 创建运行时（必须指定 model）
    let model = config
        .model
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or("gpt-4o-mini");
    info!("✓ 使用模型：{}", model);

    let execution = DefaultExecution::new(provider.clone(), model.to_string(), tools)
        .with_system_prompt(
            "你是一个专业的深度研究助手。请遵循以下研究流程：\n\
         1. 制定详细的研究计划\n\
         2. 使用搜索工具收集相关信息\n\
         3. 抓取网页获取详细内容\n\
         4. 分析整理收集的信息\n\
         5. 生成完整、结构化的研究报告\n\n\
         要求：\n\
         - 信息来源可靠\n\
         - 分析客观全面\n\
         - 报告结构清晰\n\
         - 数据准确有据",
        )
        .with_max_steps(20);

    info!("\n{}", style("【研究开始】").bold());

    // 第 1 轮：背景调研
    info!("\n{}", style("━━━ 第 1 轮：背景调研 ━━━").cyan().bold());

    let background_prompt = format!(
        "请研究以下主题的背景信息：{}\n\n\
         请收集：\n\
         1. 主题的定义和基本概念\n\
         2. 发展历史和时间线\n\
         3. 当前的发展状况\n\
         4. 关键人物和机构",
        topic
    );

    let input = AgentInput::new(background_prompt);
    let background_result: AgentOutput = execution.run(&DummyAgent, input).await?;

    // 第 2 轮：深入分析
    info!("\n{}", style("━━━ 第 2 轮：深入分析 ━━━").cyan().bold());

    let analysis_prompt = format!(
        "基于以下背景信息，深入分析：{}\n\n\
         请分析：\n\
         1. 主要影响因素和驱动力\n\
         2. 相关方和利益关系\n\
         3. 存在的挑战和机遇\n\
         4. 典型案例和实践\n\n\
         背景信息：{}",
        topic,
        background_result.text().unwrap_or("无")
    );

    let input = AgentInput::new(analysis_prompt);
    let analysis_result: AgentOutput = execution.run(&DummyAgent, input).await?;

    // 第 3 轮：未来展望
    info!("\n{}", style("━━━ 第 3 轮：未来展望 ━━━").cyan().bold());

    let future_prompt = format!(
        "基于以下分析，预测未来发展趋势：{}\n\n\
         请预测：\n\
         1. 短期发展趋势（1-2 年）\n\
         2. 中期发展趋势（3-5 年）\n\
         3. 长期发展趋势（5 年以上）\n\
         4. 潜在风险和不确定性\n\n\
         分析信息：{}",
        topic,
        analysis_result.text().unwrap_or("无")
    );

    let input = AgentInput::new(future_prompt);
    let future_result: AgentOutput = execution.run(&DummyAgent, input).await?;

    // 生成报告
    info!("\n{}", style("━━━ 生成研究报告 ━━━").cyan().bold());

    let safe_filename = topic
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c.is_whitespace() {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .replace(" ", "_");

    let report = format!(
        r#"# {} 深度研究报告

**研究日期**: {}
**研究轮数**: 3 轮（背景调研 → 深入分析 → 未来展望）

## 目录

1. [研究背景](#研究背景)
2. [深入分析](#深入分析)
3. [未来展望](#未来展望)
4. [研究结论](#研究结论)
5. [参考资料](#参考资料)

---

## 研究背景

{}

---

## 深入分析

{}

---

## 未来展望

{}

---

## 研究结论

基于以上三轮研究，得出以下结论：

1. **基本情况**：{}是一个重要的研究领域，具有广泛的应用前景。

2. **关键因素**：发展受到多种因素影响，包括技术进步、市场需求、政策支持等。

3. **发展趋势**：未来将呈现持续增长态势，在多个领域产生深远影响。

4. **建议**：相关从业者和研究者应关注最新动态，把握发展机遇。

---

## 参考资料

本研究使用了以下工具收集信息：
- Web 搜索工具
- 网页抓取工具
- SerpAPI 搜索服务（如已配置）

---

*报告生成时间：{}*  
*本研究报告由 AgentKit 深度研究助手自动生成*
"#,
        topic,
        Local::now().format("%Y 年%m 月%d 日"),
        background_result.text().unwrap_or("无背景信息"),
        analysis_result.text().unwrap_or("无分析信息"),
        future_result.text().unwrap_or("无未来展望"),
        topic,
        Local::now().format("%Y-%m-%d %H:%M:%S"),
    );

    // 保存报告
    let filename = format!("research_report_{}.md", safe_filename);
    std::fs::write(&filename, &report)?;

    info!("\n{}", style("✓ 研究完成").green().bold());
    info!("📄 报告已保存到：{}", filename);
    info!("📊 研究轮数：3 轮");
    info!("📝 报告长度：{} 字符", report.len());

    info!("\n{}", style("=== 研究完成 ===").green().bold());

    Ok(())
}

