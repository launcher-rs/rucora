//! 文件处理示例
//!
//! 展示如何使用文件工具读取、写入和编辑文件
//!
//! # 运行方式
//!
//! ```bash
//! cargo run --bin file-processor
//! ```

mod utils;

use agentkit::prelude::*;
use agentkit::runtime::{DefaultRuntime, ToolRegistry};
use agentkit::tools::{FileReadTool, FileWriteTool};
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use utils::MockProvider;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("=== 文件处理示例 ===\n");

    // 创建 Provider
    let provider = MockProvider::with_response("文件操作已完成。");

    // 创建工具注册表
    let tools = ToolRegistry::new()
        .register(FileReadTool::new())
        .register(FileWriteTool::new());

    info!("✓ 已注册 {} 个工具\n", tools.len());

    // 创建运行时
    let runtime = DefaultRuntime::new(Arc::new(provider), tools)
        .with_system_prompt("你是一个文件处理助手，可以帮助用户读取和写入文件");

    // 测试用例
    info!("1. 测试 FileReadTool：");
    let read_tool = FileReadTool::new();
    let read_result = read_tool
        .call(serde_json::json!({
            "path": "Cargo.toml"
        }))
        .await;

    match read_result {
        Ok(result) => {
            info!("   ✓ 读取文件成功");
            if let Some(content) = result.get("content") {
                let content_str = content.as_str().unwrap_or("");
                info!("   文件大小：{} 字节", content_str.len());
                info!(
                    "   前 100 字符：{}",
                    content_str.chars().take(100).collect::<String>()
                );
            }
        }
        Err(e) => {
            info!("   ⚠ 读取失败：{} (这是正常的，文件可能不存在)", e);
        }
    }

    info!("\n2. 测试 FileWriteTool：");
    let write_tool = FileWriteTool::new();
    let write_result = write_tool
        .call(serde_json::json!({
            "path": "test_output.txt",
            "content": "这是一个测试文件\n由 AgentKit 文件处理示例创建\n"
        }))
        .await;

    match write_result {
        Ok(result) => {
            info!("   ✓ 写入文件成功");
            if let Some(bytes) = result.get("bytes_written") {
                info!("   写入字节数：{}", bytes);
            }
        }
        Err(e) => {
            info!("   ⚠ 写入失败：{} (可能是权限问题)", e);
        }
    }

    info!("\n3. 使用 Runtime 进行文件操作：");
    let test_cases = vec![
        ("读取文件", "请读取 Cargo.toml 文件的内容"),
        (
            "写入文件",
            "请创建一个名为 example.txt 的文件，内容为'Hello, AgentKit!'",
        ),
    ];

    for (name, input_text) in test_cases {
        info!("\n--- 测试：{} ---", name);
        info!("输入：{}\n", input_text);

        let input = AgentInput::new(input_text.to_string());

        match runtime.run(input).await {
            Ok(output) => {
                if let Some(content) = output.text() {
                    info!("✓ 回复：{}\n", content);
                }
            }
            Err(e) => {
                info!("❌ 错误：{}\n", e);
            }
        }
    }

    info!("\n=== 示例完成 ===");
    info!("提示：文件操作需要适当的文件系统权限");

    Ok(())
}
