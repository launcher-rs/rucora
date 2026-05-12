//! rucora Agentic 自主研究示例
//!
//! 使用 Agentic 策略进行深度研究，LLM 自主决定研究路径。

use anyhow::Result;
use rucora::deep_research::{AgenticStrategy, DefaultResearchEngine};
use rucora::provider::OpenAiProvider;
use rucora_core::provider::LlmProvider;
use rucora_core::research::DeepResearchEngine;
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

    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║       rucora Agentic 自主研究示例 v1.0            ║");
    println!("║  LLM 自主决定研究路径，多轮迭代深化分析            ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    let provider = create_provider()?;
    let topic = DEFAULT_TOPIC;

    println!("【研究主题】");
    println!("  {}", topic);

    println!("\n【执行 Agentic 自主研究】");
    info!("开始自主研究: {}", topic);

    let report = run_agentic_research(&provider, topic).await?;

    println!("\n【研究完成】");
    println!("\n{}", report.summary);

    println!("\n=== 完成 ===");
    Ok(())
}

fn create_provider() -> Result<Arc<dyn LlmProvider>> {
    let api_key = std::env::var("OPENAI_API_KEY")
        .or_else(|_| std::env::var("OPENAI_KEY"))?;
    let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());

    let base_url = std::env::var("OPENAI_BASE_URL")
        .or_else(|_| std::env::var("BASE_URL"))
        .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());

    Ok(Arc::new(
        OpenAiProvider::new(&base_url, &api_key).with_default_model(&model),
    ))
}

async fn run_agentic_research(
    provider: &Arc<dyn LlmProvider>,
    topic: &str,
) -> Result<rucora_core::research::ResearchReport> {
    let strategy = AgenticStrategy::new();
    let engine = DefaultResearchEngine::new(Box::new(strategy));

    let report = engine.research(provider, topic).await?;
    Ok(report)
}