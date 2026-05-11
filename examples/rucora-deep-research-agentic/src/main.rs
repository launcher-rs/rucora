//! rucora Agentic 自主研究示例
//!
//! 使用 Agentic 策略进行深度研究，LLM 自主决定研究路径。

use anyhow::Result;
use rucora::deep_research::{AgenticStrategy, DefaultResearchEngine};
use rucora::provider::OpenAiProvider;
use rucora_core::provider::LlmProvider;
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

const DEFAULT_TOPIC: &str = "人工智能在医疗诊断中的应用";

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

    println!(
        "\n{}",
        console::style("━━━ 使用 Agentic 自主研究策略 ━━━").blue().bold()
    );
    info!("开始 Agentic 研究: {}", topic);

    let result = run_agentic_research(&provider, topic).await?;

    println!("\n{}", console::style("━━━ 研究结果 ━━━").green().bold());
    println!("{}", result.to_markdown());

    Ok(())
}

fn print_banner() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║       rucora Agentic 自主研究示例 v1.0                   ║");
    println!("║  LLM 自主决策，动态调整研究路径                          ║");
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

async fn run_agentic_research(
    provider: &Arc<dyn LlmProvider + Send + Sync>,
    topic: &str,
) -> Result<rucora_core::research::ResearchReport> {
    let strategy = AgenticStrategy::new();
    let engine = DefaultResearchEngine::new(Box::new(strategy));

    let report = engine.research(provider, topic).await?;
    Ok(report)
}