//! MCP (Model Context Protocol) 使用示例
//!
//! 展示如何连接 MCP 服务器并使用 OpenAI Provider 调用 MCP 工具完成实际任务
//!
//! ## 运行方法
//! ```bash
//! # 1. 准备 OpenAI 兼容服务（任选其一）
//!
//! # 方案 A: 使用 Ollama（推荐本地测试）
//! ollama serve
//! ollama pull qwen3.5:9b
//! export OPENAI_BASE_URL=http://127.0.0.1:11434
//!
//! # 方案 B: 使用 OpenAI
//! export OPENAI_API_KEY=sk-your-key
//!
//! # 方案 C: 使用其他 OpenAI 兼容服务
//! export OPENAI_BASE_URL=https://api.openrouter.ai/v1
//! export OPENAI_API_KEY=your-key
//!
//! # 2. 启动 MCP 服务器
//! # 示例：使用官方 filesystem MCP 服务器
//! npx -y @modelcontextprotocol/server-filesystem ~
//!
//! # 3. 设置 MCP 环境变量
//! export MCP_URL=http://127.0.0.1:8000/mcp
//! export MCP_BEARER_TOKEN=your_token_here
//!
//! # 4. 运行示例（需要 mcp feature）
//! cargo run --example 12_mcp -p agentkit --features mcp
//! ```
//!
//! ## 功能演示
//!
//! 1. **连接 MCP 服务器** - 使用 StreamableHttpClientTransport 建立连接
//! 2. **获取工具列表** - 列出 MCP 服务器提供的所有工具
//! 3. **注册 MCP 工具** - 将 MCP 工具注册到 Agent 的工具注册表
//! 4. **测试对话 1** - 使用 MCP 工具查询当前日期
//! 5. **测试对话 2** - 使用 MCP 工具计算距离假期天数
//!
//! ## 架构说明
//!
//! ```text
//! ┌─────────────┐      ┌──────────────┐      ┌─────────────┐
//! │   Agent     │─────▶│  MCP Client  │─────▶│ MCP Server  │
//! │ (qwen3.5)   │      │  (工具调用)   │      │  (工具执行)  │
//! └─────────────┘      └──────────────┘      └─────────────┘
//!        ▲                                         │
//!        │                                         │
//!        └──────────  OpenAiProvider  ─────────────┘
//!                    (qwen3.5:9b / gpt-4o / ...)
//! ```
//!
//! ## MCP 工具示例
//!
//! 本示例使用的 MCP 服务器提供以下工具：
//!
//! | 工具名 | 功能 | 使用场景 |
//! |--------|------|----------|
//! | `get_ai_news` | 获取最新 AI 资讯 | 新闻查询 |
//! | `get_time_info_tools` | 获取当前时间、日期、农历 | 时间查询 |
//! | `github_trending` | 获取 GitHub 趋势榜 | 技术趋势 |
//!
//! ## 环境变量说明
//!
//! ### OpenAI Provider 配置
//!
//! | 变量 | 说明 | 示例 |
//! |------|------|------|
//! | `OPENAI_BASE_URL` | OpenAI 兼容服务地址 | `http://127.0.0.1:11434` |
//! | `OPENAI_API_KEY` | API Key | `sk-xxx` |
//!
//! ### MCP 配置
//!
//! | 变量 | 说明 | 示例 |
//! |------|------|------|
//! | `MCP_URL` | MCP 服务器地址 | `http://127.0.0.1:8000/mcp` |
//! | `MCP_BEARER_TOKEN` | MCP 认证 Token | `your_token` |
//!
//! ## 支持的 MCP 服务器
//!
//! - **官方 MCP 服务器**: https://github.com/modelcontextprotocol/servers
//! - **FileSystem**: 文件读写、目录浏览
//! - **PostgreSQL**: 数据库查询
//! - **Git**: Git 操作
//! - **Fetch**: Web 内容抓取
//! - **自定义 MCP 服务器**: 遵循 MCP 协议的任意服务
//!
//! ## 注意事项
//!
//! 1. **模型选择**: 建议使用支持工具调用的模型，如 qwen3.5:9b、gpt-4o 等
//! 2. **MCP 服务器**: 确保 MCP 服务在 Agent 运行前已启动
//! 3. **网络连接**: MCP 客户端需要能够访问 MCP 服务器地址
//! 4. **认证配置**: 如果 MCP 服务器需要认证，请正确设置 `MCP_BEARER_TOKEN`
//!
//! ## 故障排除
//!
//! ### 问题：找不到 MCP 工具
//! **解决**: 检查 MCP 服务器是否正确启动，访问 `MCP_URL` 确认服务可用
//!
//! ### 问题：模型无法调用工具
//! **解决**: 确认使用的模型支持工具调用功能
//!
//! ### 问题：认证失败
//! **解决**: 检查 `MCP_BEARER_TOKEN` 是否正确，或联系 MCP 服务器管理员

use std::{collections::HashMap, sync::Arc};

use agentkit::agent::ToolAgent;
use agentkit::mcp::{
    ServiceExt,
    protocol::{ClientCapabilities, ClientInfo, Implementation},
    tool::McpTool,
    transport::{
        StreamableHttpClientTransport, streamable_http_client::StreamableHttpClientTransportConfig,
    },
};
use agentkit::prelude::Agent;
use agentkit::provider::OpenAiProvider;
use agentkit_mcp::McpClient;
use reqwest::header::{AUTHORIZATION, HeaderValue};
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔════════════════════════════════════════╗");
    info!("║   AgentKit MCP 使用示例               ║");
    info!("╚════════════════════════════════════════╝\n");

    // ═══════════════════════════════════════════════════════════
    // 获取 MCP 配置
    // ═══════════════════════════════════════════════════════════
    let mcp_url =
        std::env::var("MCP_URL").unwrap_or_else(|_| "http://127.0.0.1:8000/mcp".to_string());
    let bearer = match std::env::var("MCP_BEARER_TOKEN") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => {
            info!("❌ 缺少 MCP_BEARER_TOKEN 环境变量");
            info!("\n请设置：");
            info!("  export MCP_BEARER_TOKEN=your_token_here");
            info!("\n提示：如果使用本地 MCP 服务器 without auth，可设置任意值");
            info!("  export MCP_BEARER_TOKEN=dummy_token");
            return Ok(());
        }
    };

    // ═══════════════════════════════════════════════════════════
    // 步骤 1: 连接 MCP 服务器
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("步骤 1: 连接 MCP 服务器");
    info!("═══════════════════════════════════════\n");

    info!("MCP 服务器：{}", mcp_url);
    info!("Token: {}...\n", &bearer[..10.min(bearer.len())]);

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

    info!("✓ MCP 服务器连接成功\n");

    // ═══════════════════════════════════════════════════════════
    // 步骤 2: 获取 MCP 工具列表
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("步骤 2: 获取 MCP 工具列表");
    info!("═══════════════════════════════════════\n");

    let specs = mcp_client
        .list_tools()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    info!("找到 {} 个 MCP 工具:", specs.len());
    for spec in &specs {
        info!(
            "  - {}: {}",
            spec.name,
            spec.description.as_deref().unwrap_or("无描述")
        );
    }
    info!("");

    if specs.is_empty() {
        info!("⚠ 没有找到可用的 MCP 工具");
        return Ok(());
    }

    // ═══════════════════════════════════════════════════════════
    // 步骤 3: 注册 MCP 工具
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("步骤 3: 注册 MCP 工具");
    info!("═══════════════════════════════════════\n");

    let mut tools = agentkit::agent::ToolRegistry::new();
    for spec in specs {
        tools = tools.register_arc(Arc::new(McpTool::new(mcp_client.clone(), spec)));
    }
    info!("✓ 已注册 {} 个 MCP 工具\n", tools.len());

    // ═══════════════════════════════════════════════════════════
    // 步骤 4: 创建 OpenAI Provider
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("步骤 4: 创建 OpenAI Provider");
    info!("═══════════════════════════════════════\n");

    info!("使用 OpenAI Provider (qwen3.5:9b)...");
    let provider = OpenAiProvider::from_env()?;
    info!("✓ OpenAI Provider 初始化成功\n");

    // ═══════════════════════════════════════════════════════════
    // 步骤 5: 创建带 MCP 工具的 Agent
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("步骤 5: 创建带 MCP 工具的 Agent");
    info!("═══════════════════════════════════════\n");

    let agent = ToolAgent::builder()
        .provider(provider)
        .model("qwen3.5:9b")
        .system_prompt(
            "你是一个严谨的助手，擅长使用各种工具完成任务。\n\
             回答请使用中文。\n\
             如果需要获取实时信息或执行操作，请使用可用的工具。",
        )
        .tool_registry(tools)
        .max_steps(6)
        .build();

    info!("✓ Agent 创建成功\n");

    // ═══════════════════════════════════════════════════════════
    // 步骤 6: 测试对话 1 - 查询当前日期
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("步骤 6: 测试对话 1 - 查询当前日期");
    info!("═══════════════════════════════════════\n");

    let question1 = "今天几号了？";
    info!("用户：{}\n", question1);

    match agent.run(question1.into()).await {
        Ok(output) => {
            if let Some(text) = output.text() {
                info!("助手：{}\n", text);
            }
        }
        Err(e) => {
            info!("❌ 运行失败：{}\n", e);
        }
    }

    // ═══════════════════════════════════════════════════════════
    // 步骤 7: 测试对话 2 - 计算距离假期天数
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("步骤 7: 测试对话 2 - 计算距离假期天数");
    info!("═══════════════════════════════════════\n");

    let question2 = "距离清明假期还有几天？";
    info!("用户：{}\n", question2);

    match agent.run(question2.into()).await {
        Ok(output) => {
            if let Some(text) = output.text() {
                info!("助手：{}\n", text);
            }
        }
        Err(e) => {
            info!("❌ 运行失败：{}\n", e);
        }
    }

    // ═══════════════════════════════════════════════════════════
    // 步骤 8: 优雅关闭 MCP 连接
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("步骤 8: 关闭 MCP 连接");
    info!("═══════════════════════════════════════\n");

    // 显式丢弃客户端以触发清理
    drop(mcp_client);

    // 等待一小段时间让 RMCP 优雅关闭连接
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    info!("✓ MCP 连接已关闭\n");

    // ═══════════════════════════════════════════════════════════
    // 总结
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例完成！");
    info!("═══════════════════════════════════════\n");

    info!("📝 MCP 使用总结：\n");

    info!("1. 连接 MCP 服务器:");
    info!("   - 使用 StreamableHttpClientTransport");
    info!("   - 配置认证头（Bearer Token）");
    info!("   - 创建 McpClient\n");

    info!("2. 获取工具列表:");
    info!("   - 调用 list_tools() 获取工具规格");
    info!("   - 遍历工具列表了解可用功能\n");

    info!("3. 注册工具:");
    info!("   - 使用 McpTool 包装工具规格");
    info!("   - 注册到 ToolRegistry");
    info!("   - 传递给 Agent 使用\n");

    info!("4. 使用工具:");
    info!("   - Agent 自动决定调用工具");
    info!("   - MCP 服务器执行实际逻辑");
    info!("   - 返回结果给 Agent\n");

    info!("5. 环境变量:");
    info!("   - OPENAI_BASE_URL: OpenAI 兼容服务地址（如 Ollama）");
    info!("   - OPENAI_API_KEY: OpenAI API Key");
    info!("   - MCP_URL: MCP 服务器地址");
    info!("   - MCP_BEARER_TOKEN: 认证 Token\n");

    Ok(())
}
