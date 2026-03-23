//! Tool 使用示例
//!
//! 展示如何使用各种内置工具
//!
//! # 运行方式
//!
//! ```bash
//! cargo run --example 03_tools -p agentkit
//! ```

use agentkit::prelude::*;
use agentkit::tools::{EchoTool, FileReadTool, GitTool, ShellTool};
use agentkit_core::tool::Tool;
use serde_json::json;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔════════════════════════════════════════════════════════╗");
    info!("║         AgentKit Tool 使用示例                         ║");
    info!("╚════════════════════════════════════════════════════════╝\n");

    // 示例 1: Echo Tool
    info!("=== Echo Tool ===");
    let echo_tool = EchoTool;
    info!("工具名称：{}", echo_tool.name());
    info!("工具描述：{:?}", echo_tool.description());
    let result = echo_tool.call(json!({"text": "Hello, AgentKit!"})).await?;
    info!("✓ 结果：{}\n", result);

    // 示例 2: FileRead Tool
    info!("=== FileRead Tool ===");
    let file_tool = FileReadTool::new();
    info!("工具名称：{}", file_tool.name());
    info!("工具描述：{:?}", file_tool.description());
    // 尝试读取 Cargo.toml
    match file_tool.call(json!({"path": "agentkit/Cargo.toml"})).await {
        Ok(result) => {
            if let Some(content) = result.get("content") {
                let preview: String = content.as_str().unwrap_or("").chars().take(200).collect();
                info!("✓ 文件内容预览：{}...\n", preview);
            }
        }
        Err(e) => {
            info!("⚠ 读取失败（可能是文件不存在）：{}\n", e);
        }
    }

    // 示例 3: Git Tool
    info!("=== Git Tool ===");
    let git_tool = GitTool::new();
    info!("工具名称：{}", git_tool.name());
    info!("工具描述：{:?}", git_tool.description());
    // 查看 Git 状态
    match git_tool.call(json!({"command": "status"})).await {
        Ok(result) => {
            info!("✓ Git 状态：{}\n", result);
        }
        Err(e) => {
            info!("⚠ Git 命令失败（可能不在 Git 仓库中）：{}\n", e);
        }
    }

    // 示例 4: Shell Tool
    info!("=== Shell Tool ===");
    let shell_tool = ShellTool::new();
    info!("工具名称：{}", shell_tool.name());
    info!("工具描述：{:?}", shell_tool.description());
    // 执行简单命令
    match shell_tool
        .call(json!({"command": "echo 'Hello from Shell'"}))
        .await
    {
        Ok(result) => {
            info!("✓ Shell 输出：{}\n", result);
        }
        Err(e) => {
            info!("❌ 错误：{}\n", e);
        }
    }

    info!("\n=== 所有 Tool 测试完成 ===");

    Ok(())
}
