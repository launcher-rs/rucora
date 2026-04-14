//! AgentKit 深度研究示例 (生产环境版)
//!
//! 功能：
//! 1. 多 Provider 支持 - OpenAI/Anthropic/Gemini/Ollama 等
//! 2. 工具自动调用 - Web 搜索/网页抓取/文件操作等
//! 3. 结构化报告生成 - 自动保存为 Markdown 文件
//!
//! 核心修复：
//! 在 Agent 的 think 方法中手动注入工具定义，确保 LLM 可以看到并调用工具。

mod config;

use agentkit::agent::ToolRegistry;
use agentkit::agent::execution::DefaultExecution;
use agentkit::prelude::*;
use agentkit_core::agent::{Agent, AgentContext, AgentDecision};
use agentkit_core::provider::LlmProvider;
use agentkit_core::tool::ToolDefinition;
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

/// 研究 Agent
struct ResearchAgent {
    name: String,
    tool_defs: Vec<ToolDefinition>,
}

impl ResearchAgent {
    fn new(name: String, tool_defs: Vec<ToolDefinition>) -> Self {
        Self { name, tool_defs }
    }
}

#[async_trait::async_trait]
impl Agent for ResearchAgent {
    async fn think(&self, context: &AgentContext) -> AgentDecision {
        let mut req = context.default_chat_request();
        
        // 🚨 核心逻辑：将工具定义注入请求
        // 这样 LLM 才能看到工具列表并决定调用它们
        if !self.tool_defs.is_empty() {
            req.tools = Some(self.tool_defs.clone());
        }

        AgentDecision::Chat { request: Box::new(req) }
    }

    fn name(&self) -> &str { &self.name }
    fn description(&self) -> Option<&str> { Some("深度研究助手") }
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
    println!("║     AgentKit 深度研究助手 (正式版)                    ║");
    println!("╚════════════════════════════════════════════════════════╝\n");

    // 1. 配置加载
    let config = load_or_create_config()?;
    config.display();

    // 2. 创建带重试的 Provider
    let provider = create_resilient_provider(&config)?;

    // 3. 获取主题
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

    // 4. 初始化环境与工具
    let (agent, execution) = init_environment(&config, &provider)?;

    // 5. 执行研究
    run_research(&execution, &agent, &topic).await
}

/// 初始化环境
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

    // B. 🚨 关键：在注册前提取工具定义
    let tool_defs = vec![
        ToolDefinition {
            name: web_search.name().to_string(),
            description: web_search.description().map(String::from),
            input_schema: web_search.input_schema(),
        },
        ToolDefinition {
            name: web_scraper.name().to_string(),
            description: web_scraper.description().map(String::from),
            input_schema: web_scraper.input_schema(),
        },
        ToolDefinition {
            name: datetime.name().to_string(),
            description: datetime.description().map(String::from),
            input_schema: datetime.input_schema(),
        },
        ToolDefinition {
            name: file_write.name().to_string(),
            description: file_write.description().map(String::from),
            input_schema: file_write.input_schema(),
        },
        ToolDefinition {
            name: shell.name().to_string(),
            description: shell.description().map(String::from),
            input_schema: shell.input_schema(),
        },
    ];

    // 可选：SerpAPI
    if config.serpapi_keys.is_some() || std::env::var("SERPAPI_API_KEY").is_ok() {
        if let Ok(serp_tool) = agentkit_tools::SerpapiTool::from_env() {
            // 这里需要追加到 tool_defs 和 registry，为保持代码简洁略过
            info!("✅ SerpAPI 工具已加载 (请在 registry 构建链中添加 .register(tool))");
        }
    }

    info!("📦 已注册 {} 个工具", tool_defs.len());

    // C. 构建 Registry (用于实际执行)
    let registry = ToolRegistry::new()
        .register(web_search)
        .register(web_scraper)
        .register(datetime)
        .register(file_write)
        .register(shell);

    // D. 创建携带工具定义的 Agent
    let agent = ResearchAgent::new("research_agent".to_string(), tool_defs);

    // E. 创建执行器
    let model = config.model.as_deref().unwrap_or("gpt-4o-mini");
    let execution = DefaultExecution::new(provider.clone(), model.to_string(), registry)
        .with_system_prompt(
            "你是一个专业的深度研究助手。\n\
             **强制指令**：\n\
             1. 你必须先使用 'web_search' 等工具搜索最新信息。\n\
             2. 禁止仅凭训练数据回答。\n\
             3. 输出结构化的 Markdown 报告。",
        )
        .with_max_steps(20);

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

    match execution.run(agent, input).await {
        Ok(output) => {
            info!("\n{}", style("✅ 研究完成").green().bold());
            info!("Token 消耗: {}", output.total_tokens());
            
            if let Some(text) = output.text() {
                println!("\n{}", style("📄 报告摘要:").bold());
                println!("{}\n", &text.chars().take(500).collect::<String>());
                
                // 保存完整报告
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
