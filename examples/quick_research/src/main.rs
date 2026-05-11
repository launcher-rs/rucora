//! rucora 快速研究示例
//!
//! 快速获取研究摘要，适合简单的事实查询问题。

use anyhow::Result;
use rucora::agent::execution::DefaultExecution;
use rucora::agent::{Agent, ToolRegistry};
use rucora::provider::OpenAiProvider;
use rucora_core::agent::{AgentContext, AgentDecision, AgentInput, AgentOutput};
use rucora_core::provider::LlmProvider;
use rucora_tools::{BrowseTool, DatetimeTool, TavilyTool, WebFetchTool};
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

const DEFAULT_TOPIC: &str = "什么是量子计算?";

struct QuickAgent;

#[async_trait::async_trait]
impl Agent for QuickAgent {
    async fn think(&self, context: &AgentContext) -> AgentDecision {
        AgentDecision::Chat {
            request: Box::new(context.default_chat_request()),
        }
    }

    fn name(&self) -> &str {
        "quick_research"
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .compact()
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    print_banner();

    let provider = create_provider()?;
    let topic = DEFAULT_TOPIC;

    println!("{}", console::style("━━━ 研究主题 ━━━").green().bold());
    println!("  {}", console::style(topic).cyan().bold());

    println!("\n{}", console::style("━━━ 执行快速研究 ━━━").blue().bold());
    info!("开始快速研究: {}", topic);

    let result = run_quick_research(&provider, topic).await?;

    println!("\n{}", console::style("━━━ 研究结果 ━━━").green().bold());
    println!("{}", result);

    Ok(())
}

fn print_banner() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║         rucora 快速研究示例 v1.0                          ║");
    println!("║  适用于简单事实查询，30秒-3分钟快速获取摘要              ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");
}

fn create_provider() -> Result<Arc<dyn LlmProvider + Send + Sync>> {
    let api_key = std::env::var("OPENAI_API_KEY")
        .or_else(|_| std::env::var("API_KEY"))
        .expect("请设置 OPENAI_API_KEY 或 API_KEY 环境变量");

    let model = std::env::var("OPENAI_DEFAULT_MODEL")
        .or_else(|_| std::env::var("MODEL"))
        .unwrap_or_else(|_| "gpt-4o-mini".to_string());

    let base_url = std::env::var("OPENAI_BASE_URL")
        .or_else(|_| std::env::var("BASE_URL"))
        .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());

    Ok(Arc::new(
        OpenAiProvider::new(&base_url, &api_key).with_default_model(&model),
    ))
}

async fn run_quick_research(
    provider: &Arc<dyn LlmProvider + Send + Sync>,
    topic: &str,
) -> Result<String> {
    let model = std::env::var("OPENAI_DEFAULT_MODEL")
        .or_else(|_| std::env::var("MODEL"))
        .unwrap_or_else(|_| "gpt-4o-mini".to_string());

    let mut registry = ToolRegistry::new()
        .register(DatetimeTool::new())
        .register(WebFetchTool::new())
        .register(BrowseTool::new());

    if let Ok(tavily_key) = std::env::var("TAVILY_API_KEY") {
        registry = registry.register(TavilyTool::with_keys(vec![tavily_key])?);
        info!("✓ Tavily 搜索已启用");
    }

    let system_prompt = format!(
        r#"你是一名专业研究助手，负责快速回答用户的问题。

要求：
1. 使用 search/web_fetch 工具搜索相关信息（必须搜索，不能直接回答）
2. 优先从 Tavily 获取摘要，或从网页抓取关键信息
3. 用 2-4 个段落清晰回答问题
4. 每个观点必须标注来源 URL

当前时间: {{datetime}}

请开始研究主题：**{}**"#,
        topic
    );

    let execution = DefaultExecution::new(provider.clone(), &model, registry)
        .with_system_prompt(system_prompt)
        .with_max_steps(15);

    let agent = QuickAgent;
    let output = execution
        .run(&agent, AgentInput::new(topic.to_string()))
        .await?;

    Ok(output.text().unwrap_or("无结果").to_string())
}