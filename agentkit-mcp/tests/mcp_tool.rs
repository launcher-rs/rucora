use std::sync::Arc;

use agentkit_core::tool::Tool;
use agentkit_mcp::tool::{McpClient, McpTool};
use rmcp::{
    handler::server::ServerHandler,
    model::{
        CallToolRequestParams, CallToolResult, ClientCapabilities, ClientInfo, Implementation,
        ListToolsResult, ServerInfo, Tool as RmcpTool,
    },
    service::ServiceExt,
    service::serve_server,
};
use serde_json::json;

#[derive(Clone)]
struct TestServer;

impl ServerHandler for TestServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::default()
    }

    fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, rmcp::ErrorData>> + Send + '_
    {
        async move {
            let schema: rmcp::model::JsonObject =
                serde_json::from_value(json!({"type":"object"})).unwrap();
            let tool = RmcpTool::new_with_raw(
                "echo",
                Some(std::borrow::Cow::Borrowed("echo")),
                Arc::new(schema),
            );
            Ok(ListToolsResult {
                meta: None,
                next_cursor: None,
                tools: vec![tool],
            })
        }
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, rmcp::ErrorData>> + Send + '_
    {
        async move {
            if request.name.as_ref() != "echo" {
                return Ok(CallToolResult::structured_error(
                    json!({"message":"unknown tool"}),
                ));
            }

            let args = request
                .arguments
                .as_ref()
                .map(|a| serde_json::to_value(a).unwrap())
                .unwrap_or(serde_json::Value::Null);
            Ok(CallToolResult::structured(json!({"echo": args})))
        }
    }
}

// #[tokio::test]
// async fn mcp_tool_adapts_to_agentkit_tool() {
//     let (client_io, server_io) = tokio::io::duplex(64 * 1024);
//     let _server = serve_server(TestServer, server_io).await.unwrap();

//     let client_info = ClientInfo::new(
//         ClientCapabilities::default(),
//         Implementation::new("agentkit-mcp-test", "0.1.0"),
//     );
//     let client_service = client_info.serve(client_io).await.unwrap();
//     let client = McpClient::new(client_service);

//     let tools = client.list_tools().await.unwrap();
//     let tool = McpTool::new(client, tools[0].clone());

//     let out = tool.call(json!({"a":1})).await.unwrap();
//     assert_eq!(out, json!({"echo": {"a":1}}));
// }
