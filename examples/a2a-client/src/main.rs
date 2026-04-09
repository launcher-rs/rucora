//! A2A Client 示例 - 使用 agentkit 调用 A2A server
//!
//! 此示例演示如何使用 agentkit 通过 A2A 协议与远程 Agent 交互：
//! 1. 创建一个 A2A 工具，通过 ra2a 客户端调用 A2A server
//! 2. 直接调用 A2A 工具获取时间
//!
//! 运行方式：
//! ```bash
//! # 首先启动 A2A server (在另一个终端)
//! cargo run --example a2a-server
//!
//! # 然后运行客户端
//! cargo run --example a2a-client
//! ```

use agentkit::a2a::A2AToolAdapter;
use agentkit::agent::ToolRegistry;
use agentkit::core::tool::Tool;
use agentkit_providers::OllamaProvider;
use ra2a::client::Client;
use std::sync::Arc;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    println!("╔════════════════════════════════════════════════════════╗");
    println!("║         AgentKit A2A Client - 调用远程时间助手         ║");
    println!("╚════════════════════════════════════════════════════════╝\n");

    // A2A Server 地址
    let server_url = "http://localhost:8080";

    // 创建 A2A 客户端
    println!("正在连接 A2A Server: {}", server_url);
    let client = Client::from_url(server_url)?;

    // 获取 AgentCard 验证连接
    match client.get_agent_card().await {
        Ok(card) => {
            println!("✓ 已连接到 A2A Server");
            println!("  Agent 名称：{}", card.name);
            println!("  描述：{}", card.description);
        }
        Err(e) => {
            eprintln!("❌ 无法连接到 A2A Server: {}", e);
            eprintln!("\n请确保先启动 A2A Server:");
            eprintln!("  cargo run --example a2a-server\n");
            return Ok(());
        }
    }

    // 创建 A2A 工具适配器
    let a2a_tool = A2AToolAdapter::new(
        "a2a_time_agent".to_string(),
        "通过 A2A 协议调用远程时间助手，可以询问当前时间".to_string(),
        serde_json::json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "要发送给远程 Agent 的消息，例如'现在几点了？'"
                }
            },
            "required": ["message"]
        }),
        client,
    );

    // 创建 Ollama Provider (仅用于演示，实际未使用)
    println!("\n正在初始化 Ollama Provider...");
    let _provider =
        Arc::new(OllamaProvider::new("http://localhost:11434").with_default_model("qwen2.5:7b"));

    // 创建工具注册表（仅用于演示）
    let _tools = ToolRegistry::new().register(a2a_tool);

    println!("✓ Agent 初始化完成\n");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // 询问时间
    let question = "现在几点了？";
    println!("🤔 用户问：{}\n", question);
    println!("📡 正在通过 A2A 协议调用远程 Agent...\n");

    // 直接调用 A2A 工具
    let input = serde_json::json!({
        "message": question
    });

    // 重新创建客户端用于调用（因为之前的 client 已移动到 a2a_tool 中）
    let call_client = Client::from_url(server_url)?;
    let call_tool = A2AToolAdapter::new(
        "a2a_time_agent".to_string(),
        "通过 A2A 协议调用远程时间助手".to_string(),
        serde_json::json!({
            "type": "object",
            "properties": {
                "message": {"type": "string"}
            },
            "required": ["message"]
        }),
        call_client,
    );

    match call_tool.call(input).await {
        Ok(result) => {
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
            let result: serde_json::Value = result;
            if let Some(response) = result.get("response").and_then(|v| v.as_str()) {
                println!("💬 A2A Server 回复：\n{}\n", response);
            }
            println!("✓ 调用完成");
        }
        Err(e) => {
            eprintln!("❌ 调用失败：{}", e);
        }
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("示例结束\n");

    Ok(())
}

