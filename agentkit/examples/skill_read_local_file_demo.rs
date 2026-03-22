//! 文件读取技能示例
//!
//! 演示如何使用 FileReadTool 读取本地文件。
//!
//! 运行方式：
//! - `cargo run -p agentkit --example skill_read_local_file_demo`

use agentkit::runtime::{DefaultRuntime, ToolRegistry};
use agentkit::tools::FileReadTool;
use agentkit_core::agent::AgentInput;
use agentkit_core::runtime::Runtime;
use agentkit_core::tool::Tool;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    println!("=== 文件读取技能示例 ===\n");

    // 创建工具注册表，包含文件读取工具
    let tools = ToolRegistry::new().register(FileReadTool::new());

    println!("✓ 工具注册表创建成功");
    println!("  可用工具：FileReadTool\n");

    // 注意：此示例需要配置 Provider
    // 如果没有配置，将显示提示信息
    let provider_result = agentkit::provider::OpenAiProvider::from_env();

    match provider_result {
        Ok(provider) => {
            // 创建 Runtime
            let runtime = DefaultRuntime::new(Arc::new(provider), tools)
                .with_system_prompt("你是一个有用的助手，可以读取和总结文件内容。")
                .with_max_steps(5);

            // 测试：读取当前文件
            let prompt = "请读取并总结 C:\\code\\agentkit\\readme.md 文件的主要内容";

            println!("📝 请求：{}\n", prompt);

            match runtime.run(AgentInput::new(prompt)).await {
                Ok(out) => {
                    if let Some(content) = out.text() {
                        println!("✓ 回复：{}\n", content);
                    } else {
                        let out: agentkit_core::agent::AgentOutput = out;
                        println!("✓ 回复：{:?}\n", out.value);
                    }
                }
                Err(e) => {
                    eprintln!("❌ 运行失败：{}\n", e);
                    println!("提示：请确保已配置 OPENAI_API_KEY 环境变量");
                }
            }
        }
        Err(_) => {
            println!("⚠ 未配置 Provider，跳过实际运行");
            println!("\n使用方法:");
            println!("  1. 设置环境变量: export OPENAI_API_KEY=sk-...");
            println!("  2. 重新运行：cargo run -p agentkit --example skill_read_local_file_demo");

            // 演示工具调用（不依赖 Provider）
            println!("\n=== 演示工具直接调用 ===\n");

            let file_tool = FileReadTool::new();
            match file_tool
                .call(serde_json::json!({
                    "path": "C:\\code\\agentkit\\readme.md",
                    "max_bytes": 500
                }))
                .await
            {
                Ok(result) => {
                    println!("✓ 文件读取成功");
                    println!("  结果：{:?}\n", result);
                }
                Err(e) => {
                    println!("⚠ 文件读取失败：{}\n", e);
                }
            }
        }
    }

    println!("=== 示例完成 ===");
}
