//! AgentKit 流式输出示例
//!
//! 展示如何使用 Agent 的流式输出能力，包括：
//! 1. 使用 `run_stream()` 逐帧处理事件（打字机效果）
//! 2. 使用 `run_stream_text()` 直接获取拼接后的最终文本
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example 25_streaming_agent
//! ```

use agentkit::agent::{SimpleAgent, ToolAgent};
use agentkit::prelude::*;
use agentkit::provider::OpenAiProvider;
use agentkit::tools::DatetimeTool;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

fn make_provider() -> anyhow::Result<OpenAiProvider> {
    Ok(OpenAiProvider::from_env()?)
}

fn model_name() -> String {
    std::env::var("MODEL_NAME").expect("没有设置环境变量MODEL_NAME")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    // 初始化日志（仅显示 ERROR，避免干扰流式输出）
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::ERROR)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // 检查配置
    if std::env::var("OPENAI_API_KEY").is_err() && std::env::var("OPENAI_BASE_URL").is_err() {
        println!("⚠ 未设置 API 配置");
        println!("   使用 OpenAI: export OPENAI_API_KEY=sk-your-key");
        println!("   使用 Ollama: export OPENAI_BASE_URL=http://localhost:11434");
        return Ok(());
    }

    // ========================================
    // 示例 1：使用 AgentStream.next() 逐帧处理事件
    // ========================================
    println!("\n=== 示例 1: AgentStream.next() - 逐帧流式输出 ===\n");

    let agent = SimpleAgent::builder()
        .provider(make_provider()?)
        .model(model_name())
        .system_prompt("你是一个简洁的助手。")
        .build();

    println!("用户: 用一句话介绍你自己，要幽默一些");
    println!("\n助手: ");

    let mut stream = agent.run_stream("用一句话介绍你自己，要幽默一些".into());
    while let Some(event) = stream.next().await {
        match event? {
            ChannelEvent::TokenDelta(delta) => {
                print!("{}", delta.delta);
                use std::io::Write;
                std::io::stdout().flush().ok();
            }
            ChannelEvent::Error(err) => {
                eprintln!("\n错误: {}", err.message);
            }
            _ => {}
        }
    }
    println!("\n");

    // ========================================
    // 示例 2：使用 run_stream_text() 获取最终文本（最简洁）
    // ========================================
    println!("\n=== 示例 2: run_stream_text() - 获取最终文本 ===\n");

    let agent2 = SimpleAgent::builder()
        .provider(make_provider()?)
        .model(model_name())
        .system_prompt("你是一个翻译助手，只输出翻译结果。")
        .build();

    println!("用户: 将 'The quick brown fox jumps over the lazy dog' 翻译成中文");

    let text = agent2
        .run_stream_text("将 'The quick brown fox jumps over the lazy dog' 翻译成中文")
        .await?;
    println!("\n助手: {text}\n");

    // ========================================
    // 示例 3：ToolAgent 流式输出（含工具调用事件）
    // ========================================
    println!("\n=== 示例 3: ToolAgent AgentStream - 含工具调用事件 ===\n");

    let tool_agent = ToolAgent::builder()
        .provider(make_provider()?)
        .model(model_name())
        .system_prompt("你是有用的助手。当被问到时间相关问题时，使用 datetime 工具。")
        .tool(DatetimeTool)
        .max_steps(5)
        .build();

    println!("用户: 现在几点了？");
    println!("\n[事件流]");

    let mut tool_stream = tool_agent.run_stream("现在几点了？".into());
    while let Some(event) = tool_stream.next().await {
        match event? {
            ChannelEvent::TokenDelta(delta) => {
                // 直接打印 token，实现打字机效果
                print!("{}", delta.delta);
                use std::io::Write;
                std::io::stdout().flush().ok();
            }
            ChannelEvent::ToolCall(call) => {
                println!("\n>>> 调用工具: {} (参数: {})", call.name, call.input);
            }
            ChannelEvent::ToolResult(result) => {
                println!("\n>>> 工具返回: {}", result.output);
            }
            ChannelEvent::Message(_msg) => {
                println!("\n>>> 助手回复完成");
            }
            ChannelEvent::Debug(debug) => {
                // 忽略 debug 事件，避免干扰输出
                let _ = debug;
            }
            ChannelEvent::Error(err) => {
                println!("\n[错误] {}", err.message);
            }
            _ => {}
        }
    }
    println!();

    // ========================================
    // 示例 4：ToolAgent run_stream_text()
    // ========================================
    println!("\n=== 示例 4: ToolAgent run_stream_text() ===\n");

    println!("用户: 用一句话回复我，不要调用工具");

    let text2 = tool_agent
        .run_stream_text("用一句话回复我，不要调用工具")
        .await?;
    println!("\n助手: {text2}\n");

    Ok(())
}
