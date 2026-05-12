//! rucora 迭代深化研究示例
//!
//! 通过多轮迭代逐步深化研究，每轮基于前一轮结果进行更深层次的搜索和分析。

use anyhow::Result;
use rucora::agent::ToolAgent;
use rucora::prelude::Agent;
use rucora::provider::OpenAiProvider;
use rucora_core::provider::LlmProvider;
use rucora_tools::{DatetimeTool, TavilyTool};
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

const DEFAULT_TOPIC: &str = "人工智能在医疗领域的最新应用";
const DEFAULT_ITERATIONS: usize = 3;

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
    println!("║       rucora 迭代深化研究示例 v1.0                ║");
    println!("║  多轮迭代逐步深化，每轮基于前一轮结果深入分析      ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    let provider = create_provider()?;
    let topic = DEFAULT_TOPIC;
    let iterations = DEFAULT_ITERATIONS;

    println!("【研究主题】");
    println!("  {}", topic);
    println!("\n【迭代轮数】");
    println!("  {}", iterations);

    println!("\n【执行迭代研究】");
    info!("开始迭代研究: {} ({} 轮)", topic, iterations);

    let results = run_iterative_research(&provider, topic, iterations).await?;

    println!("\n【研究结果】");
    for (i, result) in results.iter().enumerate() {
        println!("\n━━━ 第 {} 轮 ━━━", i + 1);
        println!("{}", result);
    }

    println!("\n=== 完成 ===");
    Ok(())
}

fn create_provider() -> Result<Arc<dyn LlmProvider>> {
    let provider = OpenAiProvider::from_env()?;
    Ok(Arc::new(provider))
}

async fn run_iterative_research(
    provider: &Arc<dyn LlmProvider>,
    topic: &str,
    iterations: usize,
) -> Result<Vec<String>> {
    let mut results = Vec::new();
    let mut previous_summary = String::new();

    for iteration in 0..iterations {
        println!("\n  → 第 {} 轮研究...", iteration + 1);

        let system_prompt = if iteration == 0 {
            format!(
                r#"你是一名专业研究助手，负责研究主题 "{}"。
请进行全面的信息收集和研究。
注意：提供准确的信息并标注来源。"#,
                topic
            )
        } else {
            format!(
                r#"你是一名专业研究助手，正在对以下主题进行第 {} 轮深度调研。

## 研究主题: {}

## 之前的研究结果:
{}

请在前一轮研究的基础上进行更深入的分析和补充。
关注新的角度、细节和未解决的问题。"#,
                iteration + 1,
                topic,
                previous_summary
            )
        };

        let agent = ToolAgent::builder()
            .provider(provider.clone())
            .model("gpt-4o-mini")
            .system_prompt(&system_prompt)
            .tool(TavilyTool::from_env()?)
            .tool(DatetimeTool)
            .max_steps(5)
            .try_build()?;

        let query = if iteration == 0 {
            format!("请研究主题 '{}'，提供详细介绍和主要信息来源。", topic)
        } else {
            format!(
                "基于之前的研究结果 '{}'，请进行更深入的调查和补充。",
                previous_summary
            )
        };

        let output = agent.run(query.into()).await?;
        let text = output.text().unwrap_or("无结果").to_string();
        
        results.push(text.clone());
        previous_summary = text;
    }

    Ok(results)
}