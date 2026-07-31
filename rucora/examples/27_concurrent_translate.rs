//! rucora 并发翻译示例（使用框架内置 run_batch）
//!
//! 展示如何使用 Agent::run_batch 方法并发翻译多条文本。
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example 27_concurrent_translate
//! ```

use rucora::agent::SimpleAgent;
use rucora::prelude::{Agent, AgentInput};
use rucora::provider::OpenAiProvider;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let model_name = std::env::var("MODEL_NAME").expect("MODEL_NAME 未设置");
    let provider = OpenAiProvider::from_env()?;

    let agent = SimpleAgent::builder()
        .provider(provider)
        .model(model_name)
        .system_prompt("你是翻译助手。将用户输入翻译成中文，只输出翻译结果。")
        .try_build()?;

    let texts = vec![
        "Hello, how are you?",
        "The weather is nice today.",
        "Artificial intelligence is transforming the world.",
        "Rust is a systems programming language.",
        "Concurrent programming can be challenging.",
        "Tokyo is the capital of Japan.",
        "Machine learning requires large amounts of data.",
        "Open source software powers the internet.",
    ];

    let inputs: Vec<AgentInput> = texts
        .iter()
        .map(|s| AgentInput::new(s.to_string()))
        .collect::<Result<Vec<_>, _>>()?;

    info!("开始翻译 {} 条文本（并发 4）\n", inputs.len());

    let start = std::time::Instant::now();
    let results = std::sync::Arc::new(agent).run_batch(inputs, 4).await;
    let elapsed = start.elapsed();

    info!("翻译完成，耗时 {:.2}s\n", elapsed.as_secs_f64());
    for (i, result) in results.iter().enumerate() {
        match result {
            Ok(output) => info!("[{}] {} -> {}", i + 1, texts[i], output.text_unwrap()),
            Err(e) => info!("[{}] 失败：{}", i + 1, e),
        }
    }

    Ok(())
}