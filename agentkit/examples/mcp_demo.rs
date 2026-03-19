#[cfg(not(feature = "mcp"))]
fn main() {
    eprintln!(
        "This example requires feature 'mcp'. Try: cargo run -p agentkit --example mcp_demo --features mcp"
    );
}

#[cfg(feature = "mcp")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::{collections::HashMap, sync::Arc};

    use agentkit::mcp::{
        ServiceExt,
        protocol::{ClientCapabilities, ClientInfo, Implementation},
        tool::{McpClient, McpTool},
        transport::{
            StreamableHttpClientTransport,
            streamable_http_client::StreamableHttpClientTransportConfig,
        },
    };
    use agentkit_core::{
        agent::Agent,
        provider::types::{ChatMessage, Role},
    };
    use agentkit_runtime::{ToolCallingAgent, ToolRegistry};
    use reqwest::header::{AUTHORIZATION, HeaderValue};
    use tracing_subscriber::EnvFilter;

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug")),
        )
        .init();

    let mcp_url =
        std::env::var("MCP_URL").unwrap_or_else(|_| "http://127.0.0.1:8000/mcp".to_string());
    let bearer = match std::env::var("MCP_BEARER_TOKEN") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => {
            eprintln!("missing MCP_BEARER_TOKEN env var");
            return Ok(());
        }
    };

    let mut headers = HashMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {bearer}")).unwrap(),
    );
    let config = StreamableHttpClientTransportConfig::with_uri(mcp_url).custom_headers(headers);
    let transport = StreamableHttpClientTransport::from_config(config);

    let client_info = ClientInfo::new(
        ClientCapabilities::default(),
        Implementation::new("agentkit", "0.1.0"),
    );
    let service = client_info.serve(transport).await?;
    let mcp_client = McpClient::new(service);

    let specs = mcp_client.list_tools().await?;
    let mut tools = ToolRegistry::new();
    for spec in specs {
        tools = tools.register_arc(Arc::new(McpTool::new(mcp_client.clone(), spec)));
    }

    let provider = agentkit::provider::OpenAiProvider::new("http://127.0.0.1:11434/v1", "ollama")
        .with_default_model("qwen3.5:27b");

    let agent = ToolCallingAgent::new(provider, tools)
        .with_system_prompt(
            "你是一个严谨的助手。你可以调用外部工具获取真实信息。\n\
当用户询问今天几号、日期、农历等信息时，请优先调用 MCP 的日期工具获取真实结果后再回答。\n\
回答请使用中文。",
        )
        .with_max_steps(6);

    let out = agent
        .run(agentkit_core::agent::types::AgentInput {
            messages: vec![ChatMessage {
                role: Role::User,
                content: "今天几号了？".to_string(),
                name: None,
            }],
            metadata: None,
        })
        .await;

    match out {
        Ok(out) => println!("{}", out.message.content),
        Err(e) => eprintln!("运行失败：{}", e),
    }

    Ok(())
}
