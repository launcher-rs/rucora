//! MCP (Model Context Protocol) 使用示例
//!
//! 展示如何连接 MCP 服务器并使用其提供的工具
//!
//! # 运行方式
//!
//! ```bash
//! # 确保 Ollama 服务运行
//! ollama serve
//!
//! # 拉取模型
//! ollama pull qwen3.5:27b
//!
//! # 设置环境变量（可选）
//! export MCP_URL=http://127.0.0.1:8000/mcp
//! export MCP_BEARER_TOKEN=07PKJS1k0K1Hi7JkDJ0Bk3QDp7vmqRs2e0qI9FmR6vZbtc2ibh5u5SBj8je4OI88
//!
//! # 运行示例（需要 mcp feature）
//! cargo run --example 09_mcp -p agentkit --features mcp
//! ```
use std::{collections::HashMap, sync::Arc};

use agentkit::mcp::{
    ServiceExt,
    protocol::{ClientCapabilities, ClientInfo, Implementation},
    tool::{McpClient, McpTool},
    transport::{
        StreamableHttpClientTransport, streamable_http_client::StreamableHttpClientTransportConfig,
    },
};
use agentkit::runtime::{DefaultRuntime, ToolRegistry};
use agentkit_core::{agent::types::AgentInput, runtime::Runtime};
use reqwest::header::{AUTHORIZATION, HeaderValue};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    println!("╔════════════════════════════════════════════════════════╗");
    println!("║     AgentKit MCP 使用示例                              ║");
    println!("╚════════════════════════════════════════════════════════╝\n");

    // 获取 MCP 配置
    let mcp_url =
        std::env::var("MCP_URL").unwrap_or_else(|_| "http://127.0.0.1:8000/mcp".to_string());
    let bearer = match std::env::var("MCP_BEARER_TOKEN") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => {
            eprintln!("❌ 缺少 MCP_BEARER_TOKEN 环境变量");
            eprintln!("\n请设置：");
            eprintln!("  export MCP_BEARER_TOKEN=your_token_here");
            return Ok(());
        }
    };

    println!("=== 1. 连接 MCP 服务器 ===\n");
    println!("MCP 服务器：{}", mcp_url);
    println!("Token: {}...\n", &bearer[..10]);

    // 创建 HTTP 传输层
    let mut headers = HashMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {bearer}")).unwrap(),
    );
    let config = StreamableHttpClientTransportConfig::with_uri(mcp_url).custom_headers(headers);
    let transport = StreamableHttpClientTransport::from_config(config);

    // 创建客户端信息并连接
    let client_info = ClientInfo::new(
        ClientCapabilities::default(),
        Implementation::new("agentkit", "0.1.0"),
    );
    let service = client_info.serve(transport).await?;
    let mcp_client = McpClient::new(service);

    println!("✓ MCP 服务器连接成功\n");

    // 获取 MCP 工具列表
    println!("=== 2. 获取 MCP 工具列表 ===\n");

    let specs = mcp_client.list_tools().await?;
    println!("找到 {} 个 MCP 工具:", specs.len());
    for spec in &specs {
        println!(
            "  - {}: {}",
            spec.name,
            spec.description.as_deref().unwrap_or("无描述")
        );
    }
    println!("");

    if specs.is_empty() {
        println!("⚠ 没有找到可用的 MCP 工具");
        return Ok(());
    }

    // 注册 MCP 工具
    println!("=== 3. 注册 MCP 工具 ===\n");

    let mut tools = ToolRegistry::new();
    for spec in specs {
        tools = tools.register_arc(Arc::new(McpTool::new(mcp_client.clone(), spec)));
    }
    println!("✓ 已注册 {} 个 MCP 工具\n", tools.len());

    // 创建 Ollama Provider
    println!("=== 4. 创建 Ollama Provider ===\n");

    let provider = agentkit::provider::OllamaProvider::new("http://127.0.0.1:11434")
        .with_default_model("qwen3.5:9b");

    println!("✓ Ollama Provider 初始化成功");

    // 创建运行时
    println!("=== 5. 创建运行时 ===\n");

    let agent = DefaultRuntime::new(Arc::new(provider), tools)
        .with_system_prompt(
            "你是一个严谨的助手。
回答请使用中文。",
        )
        .with_max_steps(6);

    println!("✓ 运行时创建成功\n");

    // 测试对话
    println!("=== 6. 测试对话 ===\n");

    let question = "今天几号了？";
    println!("用户：{}\n", question);

    let out = agent.run(AgentInput::new(question.to_string())).await;

    match out {
        Ok(out) => {
            if let Some(content) = out.text() {
                println!("\n助手：{}", content);
            }
        }
        Err(e) => eprintln!("\n❌ 运行失败：{}", e),
    }

    let question = "今日GitHub趋势榜";
    println!("用户：{}\n", question);

    let out = agent.run(AgentInput::new(question.to_string())).await;

    match out {
        Ok(out) => {
            if let Some(content) = out.text() {
                println!("\n助手：{}", content);
            }
        }
        Err(e) => eprintln!("\n❌ 运行失败：{}", e),
    }

    println!("\n=== MCP 示例完成 ===");

    Ok(())
}
