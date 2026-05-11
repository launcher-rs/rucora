//! rucora 迭代深化研究示例
//!
//! 通过多轮迭代逐步深化研究，每轮基于前一轮结果进行更深层次的搜索和分析。

use anyhow::Result;
use async_trait::async_trait;
use rucora::agent::execution::DefaultExecution;
use rucora::agent::{Agent, ToolRegistry};
use rucora::provider::OpenAiProvider;
use rucora_core::agent::{AgentContext, AgentDecision, AgentInput, AgentOutput};
use rucora_core::provider::LlmProvider;
use rucora_tools::{BrowseTool, DatetimeTool, TavilyTool, WebFetchTool};
use std::sync::Arc;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

const DEFAULT_TOPIC: &str = "人工智能在医疗领域的最新应用";
const DEFAULT_ITERATIONS: usize = 3;

struct IterationAgent {
    name: String,
}

#[async_trait::async_trait]
impl Agent for IterationAgent {
    async fn think(&self, context: &AgentContext) -> AgentDecision {
        AgentDecision::Chat {
            request: Box::new(context.default_chat_request()),
        }
    }

    fn name(&self) -> &str {
        &self.name
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
    let iterations = DEFAULT_ITERATIONS;

    println!("{}", console::style("━━━ 研究主题 ━━━").green().bold());
    println!("  {}", console::style(topic).cyan().bold());
    println!("\n{}", console::style("━━━ 迭代轮数 ━━━").green().bold());
    println!("  {}", console::style(iterations.to_string()).cyan().bold());

    println!("\n{}", console::style("━━━ 执行迭代研究 ━━━").blue().bold());
    info!("开始迭代研究: {} ({} 轮)", topic, iterations);

    let results = run_iterative_research(&provider, topic, iterations).await?;

    println!("\n{}", console::style("━━━ 研究结果 ━━━").green().bold());
    for (i, result) in results.iter().enumerate() {
        println!("\n━━━ 第 {} 轮 ━━━", i + 1);
        println!("{}", result);
    }

    Ok(())
}

fn print_banner() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║       rucora 迭代深化研究示例 v1.0                ║");
    println!("║  多轮迭代逐步深化，每轮基于前一轮结果深入分析      ║");
    println!("╚══════════════════════════════════════════════════╝\n");
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

async fn run_iterative_research(
    provider: &Arc<dyn LlmProvider + Send + Sync>,
    topic: &str,
    iterations: usize,
) -> Result<Vec<String>> {
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

    let mut all_findings = String::new();
    let mut results = Vec::new();

    for iteration in 0..iterations {
        info!("迭代 {}/{}", iteration + 1, iterations);

        let context = if iteration == 0 {
            format!(
                r#"你是一名专业研究助手，正在对以下主题进行深度调研。

## 研究主题: {}

## 当前轮次: 第 {} 轮（共 {} 轮）

## 任务要求:
1. 使用 search/web_fetch 工具搜索相关信息
2. 将主题分解为多个子问题，分别搜索
3. 整理搜索结果，包含核心发现和来源 URL

当前时间: {{datetime}}

请开始第 1 轮研究。"#,
                topic,
                iteration + 1,
                iterations
            )
        } else {
            format!(
                r#"你是一名专业研究助手，正在对以下主题进行第 {} 轮深度调研。

## 研究主题: {}

## 前面轮次的研究发现:
{}

## 当前轮次: 第 {} 轮（共 {} 轮）

## 任务要求:
1. 基于前一轮的发现，进行更深入的搜索
2. 寻找更多信息来源，验证已有信息
3. 补充遗漏的重要方面
4. 整理新的研究发现

请开始第 {} 轮研究。"#,
                topic,
                all_findings,
                iteration + 1,
                iterations,
                iteration + 1
            )
        };

        let execution = DefaultExecution::new(provider.clone(), &model, registry.clone())
            .with_system_prompt(context)
            .with_max_steps(20);

        let agent = IterationAgent {
            name: format!("迭代研究_{}", iteration + 1),
        };

        let output = execution
            .run(&agent, AgentInput::new(topic.to_string()))
            .await?;

        let text = output.text().unwrap_or_default().to_string();
        results.push(text.clone());
        all_findings = text;

        if text.trim().is_empty() {
            warn!("第 {} 轮无输出", iteration + 1);
            break;
        }
    }

    Ok(results)
}