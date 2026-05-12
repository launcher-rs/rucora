//! rucora 研究库示例
//!
//! 演示如何使用研究库存储和检索研究结果。

use anyhow::Result;
use rucora::deep_research::{InMemoryResearchLibrary, StandardStrategy};
use rucora::provider::OpenAiProvider;
use rucora_core::provider::LlmProvider;
use rucora_core::research::{DeepResearchEngine, ResearchLibrary};
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .compact()
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    println!("\n=== 研究库示例 ===\n");

    let provider = create_provider()?;
    let library = InMemoryResearchLibrary::new();

    println!("【研究主题】");
    println!("  人工智能的发展历史");

    println!("\n【执行研究并保存到库】");
    let report = run_research_with_library(&provider, "人工智能的发展历史").await?;

    let id = library.save(&report).await?;
    info!("研究报告已保存，ID: {}", id);

    println!("\n【搜索研究库】");
    let results = library.search("人工智能").await?;
    println!("找到 {} 条相关研究", results.len());

    println!("\n【研究库内容】");
    let all = library.list(10).await?;
    println!("共存储 {} 项研究", all.len());

    for (i, r) in all.iter().enumerate() {
        println!("  {}. {}", i + 1, r.topic);
    }

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

async fn run_research_with_library(
    provider: &Arc<dyn LlmProvider>,
    topic: &str,
) -> Result<rucora_core::research::ResearchReport> {
    use rucora::deep_research::DefaultResearchEngine;

    let strategy = StandardStrategy::new();
    let engine = DefaultResearchEngine::new(Box::new(strategy));

    let report = engine.research(provider, topic).await?;
    Ok(report)
}