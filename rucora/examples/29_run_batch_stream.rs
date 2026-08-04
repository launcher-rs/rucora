//! rucora 批量流式翻译示例（使用 run_batch_stream）
//!
//! 展示如何使用 Agent::run_batch_stream 并发翻译多条文本，核心特性：
//! 1. **逐条产出**：每完成一条立即产出，无需等全部完成，可实时感知进度
//! 2. **原始索引**：每条产出为 `(原始索引, 结果)`，可直接按输入顺序回填，不会乱序
//! 3. **提前终止**：drop 掉流即可停止消费剩余任务（对比 `run_batch` 必须收齐全量结果）
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example 29_run_batch_stream
//! ```

use rucora::agent::SimpleAgent;
use rucora::prelude::{Agent, AgentInput, StreamExt};
use rucora::provider::OpenAiProvider;
use std::sync::Arc;
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

    let agent = SimpleAgent::builder(provider)
        .model(model_name)
        .system_prompt("你是翻译助手。将用户输入翻译成中文，只输出翻译结果。")
        .build();

    let texts = [
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

    info!("开始翻译 {} 条文本（并发 4，逐条产出）\n", inputs.len());

    // run_batch_stream 要求 Self: 'static，故以 Arc<Self> 接收者调用
    let agent = Arc::new(agent);

    // 按原始索引回填翻译结果：流以「完成顺序」产出，但每条自带原始索引，
    // 因此即使任务交错完成，最终数组仍与输入一一对应（重复文本也不会错位）
    let mut zh_results = vec![String::new(); texts.len()];
    let mut done = 0;

    let mut stream = agent.run_batch_stream(inputs, 4).await;
    let start = std::time::Instant::now();
    while let Some((idx, result)) = stream.next().await {
        done += 1;
        match result {
            Ok(output) => {
                let zh = output.text_unwrap().to_string();
                zh_results[idx] = zh.clone();
                info!(
                    "[{idx}] 完成（{done}/{}）{:.1}s：{} -> {}",
                    texts.len(),
                    start.elapsed().as_secs_f64(),
                    texts[idx],
                    zh
                );
            }
            Err(e) => eprintln!("[{idx}] 失败：{e}"),
        }
    }
    let elapsed = start.elapsed();

    info!("\n翻译完成，耗时 {:.2}s\n", elapsed.as_secs_f64());

    // 最终结果已按原始顺序就位，直接遍历输出即可
    for (i, zh) in zh_results.iter().enumerate() {
        info!("[{}] {} -> {}", i + 1, texts[i], zh);
    }

    Ok(())
}
