//! AgentKit 深度研究示例 (API 改进验证版)
//!
//! 验证库缺陷修复：
//! 1. Tool trait 增加了 definition() 方法。
//! 2. ToolRegistry 提供了 definitions() 方法。
//! 3. DefaultExecution 现在会自动注入工具定义，Agent 无需手动处理。

mod config;

use agentkit::agent::ToolRegistry;
use agentkit::agent::execution::DefaultExecution;
use agentkit::prelude::*;
use agentkit_core::agent::{Agent, AgentContext, AgentDecision};
use agentkit_core::provider::LlmProvider;
use agentkit::provider::resilient::{ResilientProvider, RetryConfig};
use agentkit_tools::{
    DatetimeTool, FileWriteTool, ShellTool, WebScraperTool, WebSearchTool,
};
use chrono::Local;
use config::{AppConfig, ProviderType};
use console::style;
use dialoguer::{Input, Select};
use std::io::{self, Write};
use std::sync::Arc;
use tracing::{Level, info, warn};
use tracing_subscriber::FmtSubscriber;

// ====================== 核心组件 ======================

/// 研究 Agent (简化版：不再需要手动管理工具定义)
struct ResearchAgent {
    name: String,
}

impl ResearchAgent {
    fn new(name: String) -> Self {
        Self { name }
    }
}

#[async_trait::async_trait]
impl Agent for ResearchAgent {
    async fn think(&self, context: &AgentContext) -> AgentDecision {
        info!("🧠 [ResearchAgent] 正在思考...");
        // 🚨 改进验证：Agent 不再需要手动注入工具定义！
        // DefaultExecution 会在执行前自动从 ToolRegistry 获取定义并注入。
        AgentDecision::Chat {
            request: Box::new(context.default_chat_request()),
        }
    }

    fn name(&self) -> &str { &self.name }
    fn description(&self) -> Option<&str> { Some("深度研究助手 (自动工具注入)") }
}

// ====================== 主流程 ======================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    println!("╔════════════════════════════════════════════════════════╗");
    println!("║     AgentKit 深度研究 (API 改进验证版)                ║");
    println!("║  特性：自动工具注入 / 简化 Agent 实现                 ║");
    println!("╚════════════════════════════════════════════════════════╝\n");

    // 1. 加载配置
    let config = load_or_create_config()?;
    config.display();

    // 2. 创建带重试的 Provider
    let provider = create_resilient_provider(&config)?;

    // 3. 获取研究主题
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

    // 4. 初始化环境 (验证新 API)
    let (agent, execution) = init_environment(&config, &provider)?;

    // 5. 执行研究
    run_research(&execution, &agent, &topic).await
}

/// 初始化环境 (展示如何使用 Registry 获取定义)
fn init_environment(
    config: &AppConfig,
    provider: &Arc<dyn LlmProvider + Send + Sync>,
) -> anyhow::Result<(ResearchAgent, DefaultExecution)> {
    
    // A. 实例化工具
    let web_search = WebSearchTool::new().with_max_results(5);
    let web_scraper = WebScraperTool::new();
    let datetime = DatetimeTool::new();
    let file_write = FileWriteTool::new();
    let shell = ShellTool::new();

    // B. 注册工具到 Registry (用于实际执行)
    let registry = ToolRegistry::new()
        .register(web_search)
        .register(web_scraper)
        .register(datetime)
        .register(file_write)
        .register(shell);

    info!("📦 ToolRegistry 已注册 {} 个工具", registry.len());

    // C. 🚨 验证改进 1：使用 definition() 方法
    // 以前需要手动拼接，现在可以直接调用 tool.definition()
    // 这里仅做演示，实际执行中 DefaultExecution 会自动处理
    let sample_tool = registry.get("web_search").unwrap();
    let def = sample_tool.definition();
    info!("✅ 验证 Tool.definition() 成功: {}", def.name);

    // D. 🚨 验证改进 2：使用 Registry 获取所有定义
    // DefaultExecution 内部将调用此方法自动注入
    let all_defs = registry.definitions();
    info!("✅ 验证 Registry.definitions() 成功: 共 {} 个工具定义", all_defs.len());

    // E. 创建简化版 Agent (无需传递工具定义)
    let agent = ResearchAgent::new("research_agent".to_string());

    // F. 创建执行器 (它会自动负责工具注入)
    let model = config.model.as_deref().unwrap_or("qwen3.5:9b");
    let execution = DefaultExecution::new(provider.clone(), model.to_string(), registry)
        .with_system_prompt(
            "你是一个专业的深度研究助手。\n\
             **强制指令**：\n\
             1. 你必须先使用 'web_search' 等工具搜索最新信息。\n\
             2. 禁止仅凭训练数据回答。\n\
             3. 输出结构化的 Markdown 格式。",
        )
        .with_max_steps(15);

    Ok((agent, execution))
}

/// 执行研究阶段
async fn run_research(
    execution: &DefaultExecution,
    agent: &ResearchAgent,
    topic: &str,
) -> anyhow::Result<()> {
    
    info!("\n{}", style("【开始研究】").bold());

    let prompt = format!(
        "请研究以下主题：{}\n\n\
         请使用搜索工具获取最新信息并生成报告。",
        topic
    );

    let input = AgentInput::new(prompt);

    // 执行！Agent 将自动获得工具列表
    match execution.run(agent, input).await {
        Ok(output) => {
            info!("\n{}", style("✅ 研究完成").green().bold());
            info!("Token 消耗: {}", output.total_tokens());
            
            if let Some(text) = output.text() {
                println!("\n{}", style("📄 报告摘要:").bold());
                println!("{}\n", &text.chars().take(500).collect::<String>());
                
                let filename = format!("research_{}.md", Local::now().format("%Y%m%d_%H%M%S"));
                std::fs::write(&filename, text)?;
                info!("💾 完整报告已保存至: {}", filename);
            }
        }
        Err(e) => {
            warn!("❌ 研究失败: {}", e);
        }
    }

    Ok(())
}

// ====================== 辅助函数 ======================

fn load_or_create_config() -> anyhow::Result<AppConfig> {
    if let Some(config) = AppConfig::load() && config.is_complete() {
        println!("✓ 已加载现有配置");
        return Ok(config);
    }

    println!("\n{}", style("━━━ 配置向导 ━━━").blue().bold());
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
    let api_key: String = Input::new().with_prompt("输入 API Key").interact_text()?;
    let model: String = Input::new()
        .with_prompt("输入模型名称")
        .default(selected_provider.default_model().to_string())
        .interact_text()?;

    let config = AppConfig {
        provider: Some(selected_provider.name().to_string()),
        api_key: Some(api_key),
        model: Some(model),
        base_url: selected_provider.default_base_url().map(String::from),
        serpapi_keys: None,
    };
    config.save()?;
    Ok(config)
}

fn create_resilient_provider(config: &AppConfig) -> anyhow::Result<Arc<dyn LlmProvider + Send + Sync>> {
    let api_key = config.api_key.as_ref().ok_or_else(|| anyhow::anyhow!("缺少 API Key"))?;
    let model = config.model.as_ref().ok_or_else(|| anyhow::anyhow!("缺少模型配置"))?;
    let base_url = config.base_url.as_deref().unwrap_or("https://api.openai.com/v1");

    let inner = Arc::new(agentkit_providers::OpenAiProvider::new(base_url, api_key.clone())
        .with_default_model(model.clone()));

    Ok(Arc::new(
        ResilientProvider::new(inner).with_config(RetryConfig::default())
    ))
}
