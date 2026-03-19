use std::{borrow::Cow, sync::Arc};

use agentkit_core::{
    error::ToolError,
    tool::{Tool, ToolCategory},
};
use async_trait::async_trait;
use rmcp::{
    model::{
        CallToolRequestParams, CallToolResult, InitializeRequestParams, JsonObject, RawContent,
        Tool as RmcpTool,
    },
    service::{Peer, RoleClient, RunningService},
};
use serde_json::{Value, json};
use tracing::{debug, trace};

#[derive(Clone)]
pub struct McpClient {
    client: Arc<RunningService<RoleClient, InitializeRequestParams>>,
}

impl McpClient {
    pub fn new(client: RunningService<RoleClient, InitializeRequestParams>) -> Self {
        Self {
            client: Arc::new(client),
        }
    }

    fn peer(&self) -> &Peer<RoleClient> {
        self.client.peer()
    }

    pub async fn list_tools(&self) -> Result<Vec<RmcpTool>, String> {
        let start = std::time::Instant::now();
        let tools = self
            .peer()
            .list_all_tools()
            .await
            .map_err(|e| e.to_string())?;
        debug!(
            tools_len = tools.len(),
            elapsed_ms = start.elapsed().as_millis() as u64,
            "mcp.list_tools.done"
        );
        trace!(
            tool_names = ?tools.iter().map(|t| t.name.as_ref()).collect::<Vec<_>>(),
            "mcp.list_tools.tools"
        );
        Ok(tools)
    }

    pub async fn call_tool(&self, name: &str, input: Value) -> Result<CallToolResult, String> {
        let input_preview = {
            const MAX: usize = 800;
            let s = input.to_string();
            if s.len() <= MAX {
                s
            } else {
                format!("{}...<truncated:{}>", &s[..MAX], s.len())
            }
        };
        debug!(tool.name = %name, tool.input = %input_preview, "mcp.call_tool.start");

        let arguments = match input {
            Value::Null => None,
            other => Some(serde_json::from_value::<JsonObject>(other).map_err(|e| e.to_string())?),
        };

        let params = match arguments {
            Some(args) => {
                CallToolRequestParams::new(Cow::Owned(name.to_string())).with_arguments(args)
            }
            None => CallToolRequestParams::new(Cow::Owned(name.to_string())),
        };

        let start = std::time::Instant::now();
        let result = self
            .peer()
            .call_tool(params)
            .await
            .map_err(|e| e.to_string())?;

        let structured_len = result
            .structured_content
            .as_ref()
            .map(|v| v.to_string().len());
        debug!(
            tool.name = %name,
            elapsed_ms = start.elapsed().as_millis() as u64,
            has_structured = result.structured_content.is_some(),
            structured_len = structured_len,
            content_items = result.content.len(),
            "mcp.call_tool.done"
        );

        let result_preview = {
            const MAX: usize = 1200;
            let s = result
                .structured_content
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "<no structured_content>".to_string());
            if s.len() <= MAX {
                s
            } else {
                format!("{}...<truncated:{}>", &s[..MAX], s.len())
            }
        };
        trace!(tool.name = %name, result = %result_preview, "mcp.call_tool.result_preview");

        Ok(result)
    }
}

pub struct McpTool {
    client: McpClient,
    spec: RmcpTool,
}

impl McpTool {
    pub fn new(client: McpClient, spec: RmcpTool) -> Self {
        Self { client, spec }
    }
}

#[async_trait]
impl Tool for McpTool {
    fn name(&self) -> &str {
        self.spec.name.as_ref()
    }

    fn description(&self) -> Option<&str> {
        self.spec.description.as_deref()
    }

    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::External]
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(self.spec.input_schema.as_ref())
            .unwrap_or_else(|_| json!({"type":"object"}))
    }

    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let result = self
            .client
            .call_tool(self.spec.name.as_ref(), input)
            .await
            .map_err(ToolError::Message)?;

        if let Some(v) = result.structured_content {
            return Ok(v);
        }

        let mut text = String::new();
        for c in result.content {
            if let RawContent::Text(t) = c.raw {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(&t.text);
            }
        }

        Ok(json!({"content": text}))
    }
}
