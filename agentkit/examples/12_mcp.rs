//! AgentKit MCP（Model Context Protocol）示例
//!
//! 展示如何集成 MCP 服务器并使用其提供的工具。
//!
//! ## 运行方法
//! ```bash
//! # 首先需要启动 MCP 服务器
//! cargo run --example 12_mcp --features mcp
//! ```

#[cfg(feature = "mcp")]
use agentkit::mcp::McpClient;
#[cfg(feature = "mcp")]
use tracing::{Level, info};
#[cfg(feature = "mcp")]
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
#[cfg(feature = "mcp")]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔════════════════════════════════════════╗");
    info!("║   AgentKit MCP 示例                   ║");
    info!("╚════════════════════════════════════════╝\n");

    info!("MCP（Model Context Protocol）是一个开放协议，");
    info!("用于标准化 AI 模型与外部工具和数据的集成。\n");

    info!("═══════════════════════════════════════");
    info!("MCP 主要功能:");
    info!("═══════════════════════════════════════");
    info!("1. 工具发现 - 自动发现 MCP 服务器提供的工具");
    info!("2. 工具调用 - 远程调用 MCP 工具");
    info!("3. 资源访问 - 访问 MCP 服务器管理的资源");
    info!("4. 提示词模板 - 使用 MCP 提供的提示词模板");
    info!("═══════════════════════════════════════\n");

    info!("配置 MCP 客户端:");
    info!("  URL: http://localhost:8000/mcp");
    info!("  Token: Bearer Token 认证（可选）\n");

    // 注意：实际使用需要启动 MCP 服务器
    info!("提示：运行此示例前，需要先启动 MCP 服务器");
    info!("参考：https://github.com/modelcontextprotocol\n");

    info!("示例完成！");

    Ok(())
}

#[cfg(not(feature = "mcp"))]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("MCP feature 未启用");
    println!("请使用以下命令运行:");
    println!("  cargo run --example 12_mcp --features mcp");
    Ok(())
}
