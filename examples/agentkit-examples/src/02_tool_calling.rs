//! 工具调用示例
//!
//! 展示如何使用各种内置工具
//!
//! # 运行方式
//!
//! ```bash
//! cargo run --bin tool-calling
//! ```

mod utils;

use agentkit::prelude::*;
use agentkit::runtime::{DefaultRuntime, ToolRegistry};
use agentkit::tools::{EchoTool, FileReadTool, GitTool, ShellTool};
use futures_util::StreamExt;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use utils::create_provider_or_mock;

fn truncate(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        &s[..max_len]
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("=== 工具调用示例 ===\n");

    let provider = create_provider_or_mock();

    let tools = ToolRegistry::new()
        .register(EchoTool)
        .register(FileReadTool::new())
        .register(GitTool::new())
        .register(ShellTool::new());

    info!("✓ 已注册 {} 个工具\n", tools.len());

    let runtime = DefaultRuntime::new(provider, tools)
        .with_system_prompt("你是一个有用的助手，可以使用工具帮助用户")
        .with_max_steps(5);

    let test_cases = vec![
        ("回显测试", "请回显这句话：Hello, Tools!"),
        ("Git 测试", "当前 Git 仓库的状态如何？"),
        ("文件测试", "读取当前目录下的 Cargo.toml 文件"),
        ("Shell 测试", "执行命令：echo 'Hello from Shell'"),
    ];

    for (name, input_text) in test_cases {
        info!("--- 测试：{} ---", name);
        info!("输入：{}\n", input_text);

        let input = AgentInput::new(input_text.to_string());
        let mut stream = runtime.run_stream(input);

        while let Some(event) = stream.next().await {
            match event? {
                ChannelEvent::TokenDelta(delta) => print!("{}", delta.delta),
                ChannelEvent::ToolCall(call) => {
                    info!("\n🔧 工具调用：{} {:?}", call.name, call.input)
                }
                ChannelEvent::ToolResult(result) => info!(
                    "\n✓ 工具结果：{}",
                    truncate(&result.output.to_string(), 200)
                ),
                ChannelEvent::Message(msg) => info!("\n✓ 最终回复：{}\n", msg.content),
                _ => {}
            }
        }
    }

    Ok(())
}
