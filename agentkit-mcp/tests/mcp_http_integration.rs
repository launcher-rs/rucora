use std::collections::HashMap;

use agentkit_mcp::tool::McpClient;
use reqwest::header::{AUTHORIZATION, HeaderValue};
use rmcp::{
    ServiceExt,
    model::{ClientCapabilities, ClientInfo, Implementation},
    transport::{
        StreamableHttpClientTransport, streamable_http_client::StreamableHttpClientTransportConfig,
    },
};

#[tokio::test]
async fn mcp_http_bearer_token_list_tools_and_call_date_tool() {
    let mcp_url = match std::env::var("MCP_URL") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => return,
    };

    let bearer = match std::env::var("MCP_BEARER_TOKEN") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => return,
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
        Implementation::new("agentkit-mcp", "0.1.0"),
    );

    let service = client_info.serve(transport).await.unwrap();
    let client = McpClient::new(service);

    let tools = client.list_tools().await.unwrap();
    assert!(!tools.is_empty());

    let maybe_date_tool = tools.iter().find(|t| {
        let name = t.name.as_ref();
        let desc = t.description.as_deref().unwrap_or("");
        let name_lc = name.to_lowercase();
        let desc_lc = desc.to_lowercase();

        name_lc.contains("date")
            || name_lc.contains("today")
            || name_lc.contains("calendar")
            || name.contains("日期")
            || name.contains("今天")
            || name.contains("农历")
            || desc_lc.contains("lunar")
            || desc.contains("日期")
            || desc.contains("今天")
            || desc.contains("农历")
    });

    let Some(date_tool) = maybe_date_tool else {
        return;
    };

    let out = client
        .call_tool(date_tool.name.as_ref(), serde_json::json!({}))
        .await
        .unwrap();

    let v = out.structured_content.unwrap_or(serde_json::Value::Null);
    assert_ne!(v, serde_json::Value::Null);
}
