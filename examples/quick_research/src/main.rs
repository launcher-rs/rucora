//! rucora 快速研究示例
//!
//! 快速获取研究摘要，适合简单的事实查询问题。

use anyhow::Result;
use rucora::agent::ToolAgent;
use rucora::prelude::Agent;
use rucora::provider::OpenAiProvider;
use rucora_core::provider::LlmProvider;
use rucora_tools::{DatetimeTool, TavilyTool};
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

const DEFAULT_TOPIC: &str = "什么是量子计算?";

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .compact()
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    println!("\n=== 快速研究示例 ===\n");

    let provider = create_provider()?;
    let topic = DEFAULT_TOPIC;

    println!("【研究主题】");
    println!("  {}\n", topic);

    println!("【执行快速研究】");
    info!("开始快速研究: {}", topic);

    let result = run_quick_research(&provider, topic).await?;

    println!("【研究结果】\n");
    println!("{}", result);

    println!("\n=== 完成 ===");
    Ok(())
}

fn create_provider() -> Result<Arc<dyn LlmProvider>> {
    let provider = OpenAiProvider::from_env()?;
    Ok(Arc::new(provider))
}

async fn run_quick_research(provider: &Arc<dyn LlmProvider>, topic: &str) -> Result<String> {
    let system_prompt = r#"你是一名专业研究助手，负责快速收集和总结信息。
请根据用户的主题进行简要研究，提供清晰的回答并标注信息来源。
注意：回答要简洁准确，尽量引用可靠的来源。"#;

    let agent = ToolAgent::builder()
        .provider(provider.clone())
        .model("gpt-4o-mini")
        .system_prompt(system_prompt)
        .tool(TavilyTool::from_env()?)
        .tool(DatetimeTool)
        .max_steps(5)
        .try_build()?;

    let query = format!(
        "请帮我研究以下主题 '{}'，提供简要介绍和主要信息来源。",
        topic
    );

    let output = agent.run(query.into()).await?;

    let text = output.text().unwrap_or("无结果");
    Ok(text.to_string())
}