//! rucora 研究库示例
//!
//! 演示如何使用研究库存储和检索研究结果。

use anyhow::Result;
use rucora::deep_research::{InMemoryResearchLibrary, StandardStrategy};
use rucora::provider::OpenAiProvider;
use rucora_core::provider::LlmProvider;
use rucora_core::research::ResearchLibrary;
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

    print_banner();

    let provider = create_provider()?;
    let library = InMemoryResearchLibrary::new();

    println!("{}", console::style("━━━ 研究主题 ━━━").green().bold());
    println!("  {}", console::style("人工智能的发展历史").cyan().bold());

    // 执行研究
    println!("\n{}", console::style("━━━ 执行研究并保存到库 ━━━").blue().bold());
    let report = run_research_with_library(&provider, "人工智能的发展历史").await?;

    // 保存到库
    let id = library.save(&report).await?;
    info!("研究报告已保存，ID: {}", id);

    // 搜索历史研究
    println!(
        "\n{}",
        console::style("━━━ 搜索研究库 ━━━").blue().bold()
    );
    let results = library.search("人工智能").await?;
    println!("找到 {} 条相关研究", results.len());

    // 列出所有研究
    println!(
        "\n{}",
        console::style("━━━ 研究库内容 ━━━").blue().bold()
    );
    let all = library.list(10).await?;
    for r in &all {
        println!("  - {} ({})", r.topic, r.created_at.format("%Y-%m-%d"));
    }

    println!("\n{}", console::style("━━━ 研究报告 ━━━").green().bold());
    println!("{}", report.to_markdown());

    Ok(())
}

fn print_banner() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║          rucora 研究库示例 v1.0                         ║");
    println!("║  存储和检索研究结果，建立个人知识库                     ║");
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

async fn run_research_with_library(
    provider: &Arc<dyn LlmProvider + Send + Sync>,
    topic: &str,
) -> Result<rucora_core::research::ResearchReport> {
    use rucora::deep_research::DefaultResearchEngine;

    let strategy = StandardStrategy::new();
    let engine = DefaultResearchEngine::new(Box::new(strategy));

    let report = engine.research(provider, topic).await?;
    Ok(report)
}