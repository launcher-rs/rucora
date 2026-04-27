//! MCP 工具适配器模块
//!
//! # 概述
//!
//! 本模块提供 MCP 工具与 agentkit Tool trait 的适配功能：
//! - [`McpClient`]: MCP 客户端封装
//! - [`McpTool`]: MCP 工具适配器
//!
//! # 核心类型
//!
//! ## McpClient
//!
//! [`McpClient`] 是对 `rmcp` 的 `RunningService` 的封装，提供：
//! - 列出远端工具（`list_tools`）
//! - 调用远端工具（`call_tool`）
//!
//! ```rust,no_run
//! use agentkit::mcp::McpClient;
//!
//! # async fn example(client: McpClient) -> Result<(), Box<dyn std::error::Error>> {
//! // 列出可用工具
//! let tools = client.list_tools().await?;
//!
//! for tool in tools {
//!     println!("工具：{}", tool.name);
//! }
//!
//! // 调用工具
//! let result = client.call_tool("my_tool", serde_json::json!({})).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## McpTool
//!
//! [`McpTool`] 将远端 MCP 工具包装为 [`agentkit_core::tool::Tool`]：
//!
//! ```rust,no_run
//! use agentkit::mcp::{McpClient, McpTool};
//! use agentkit_core::tool::Tool;
//!
//! # async fn example(client: McpClient) -> Result<(), Box<dyn std::error::Error>> {
//! // 获取 MCP 工具定义
//! let tools = client.list_tools().await?;
//! let spec = tools.into_iter().next().unwrap();
//!
//! // 创建适配器
//! let tool = McpTool::new(client, spec);
//!
//! // 作为 agentkit Tool 使用
//! let result = tool.call(serde_json::json!({})).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # 使用流程
//!
//! ```text
//! 1. 创建 MCP 传输层
//!    │
//!    ▼
//! 2. 连接 MCP 服务器
//!    │
//!    ▼
//! 3. 创建 McpClient
//!    │
//!    ▼
//! 4. 列出可用工具
//!    │
//!    ▼
//! 5. 创建 McpTool 适配器
//!    │
//!    ▼
//! 6. 作为 agentkit Tool 调用
//! ```

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

/// MCP 客户端封装
///
/// 该类型对 `rmcp` 的 `RunningService` 做了一层薄封装：
/// - 提供列出远端工具（`list_tools`）
/// - 提供按名称调用工具（`call_tool`）
///
/// 主要目的是让上层不用直接和 `rmcp::service` 的细节打交道。
///
/// # 示例
///
/// ```rust,no_run
/// use agentkit::mcp::{McpClient, StdioTransport};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // 创建传输层并连接
/// let transport = StdioTransport::new("mcp-server");
/// let service = transport.connect().await?;
///
/// // 创建客户端
/// let client = McpClient::new(service);
///
/// // 列出工具
/// let tools = client.list_tools().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct McpClient {
    /// 底层 rmcp 客户端服务
    client: Arc<RunningService<RoleClient, InitializeRequestParams>>,
}

impl McpClient {
    /// 使用已初始化的 `rmcp` client service 构造一个 `McpClient`
    ///
    /// # 参数
    ///
    /// - `client`: 已初始化的 rmcp RunningService
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit::mcp::McpClient;
    /// # use rmcp::service::{RunningService, RoleClient, InitializeRequestParams};
    ///
    /// # fn example(client: RunningService<RoleClient, InitializeRequestParams>) {
    /// let client = McpClient::new(client);
    /// # }
    /// ```
    pub fn new(client: RunningService<RoleClient, InitializeRequestParams>) -> Self {
        Self {
            client: Arc::new(client),
        }
    }

    /// 获取底层 `rmcp` 的 peer 句柄，用于发起 RPC
    fn peer(&self) -> &Peer<RoleClient> {
        self.client.peer()
    }

    /// 列出对端暴露的全部工具定义
    ///
    /// # 返回值
    ///
    /// 返回的是 `rmcp` 的 `Tool` 描述（包含 name/description/schema 等）。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use agentkit::mcp::McpClient;
    ///
    /// # async fn example(client: McpClient) -> Result<(), Box<dyn std::error::Error>> {
    /// let tools = client.list_tools().await?;
    ///
    /// for tool in tools {
    ///     println!("工具：{} - {}", tool.name, tool.description.unwrap_or_default());
    /// }
    /// # Ok(())
    /// # }
    /// ```
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

    /// 调用对端的指定工具
    ///
    /// # 参数
    ///
    /// - `name`: 工具名称
    /// - `input`: 工具输入（将被转换为 MCP 的 `arguments`）
    ///
    /// # 返回值
    ///
    /// 返回值是 MCP 的原始 `CallToolResult`，调用方可以选择读取 structured_content
    /// 或拼接文本内容。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use agentkit::mcp::McpClient;
    /// use serde_json::json;
    ///
    /// # async fn example(client: McpClient) -> Result<(), Box<dyn std::error::Error>> {
    /// let result = client.call_tool(
    ///     "my_tool",
    ///     json!({"param": "value"})
    /// ).await?;
    ///
    /// println!("结果：{:?}", result);
    /// # Ok(())
    /// # }
    /// ```
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
                .map_or_else(|| "<no structured_content>".to_string(), |v| v.to_string());
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

/// MCP 工具适配器
///
/// 将远端 MCP 工具包装为 `agentkit-core` 的 `Tool`。
///
/// 用于把外部工具以统一的 `Tool` 方式接入 agentkit 的 agent/tooling 体系。
///
/// # 示例
///
/// ```rust,no_run
/// use agentkit::mcp::{McpClient, McpTool};
/// use agentkit_core::tool::Tool;
///
/// # async fn example(client: McpClient) -> Result<(), Box<dyn std::error::Error>> {
/// // 获取 MCP 工具定义
/// let tools = client.list_tools().await?;
/// let spec = tools.into_iter().next().unwrap();
///
/// // 创建适配器
/// let tool = McpTool::new(client, spec);
///
/// // 作为 agentkit Tool 使用
/// let result = tool.call(serde_json::json!({})).await?;
/// println!("结果：{}", result);
/// # Ok(())
/// # }
/// ```
pub struct McpTool {
    /// MCP 客户端
    client: McpClient,
    /// MCP 工具定义
    spec: RmcpTool,
}

impl McpTool {
    /// 使用 `McpClient` 与远端工具定义构造 `McpTool`
    ///
    /// # 参数
    ///
    /// - `client`: MCP 客户端
    /// - `spec`: MCP 工具定义
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit::mcp::{McpClient, McpTool};
    /// # use rmcp::model::Tool as RmcpTool;
    ///
    /// # fn example(client: McpClient, spec: RmcpTool) {
    /// let tool = McpTool::new(client, spec);
    /// # }
    /// ```
    pub fn new(client: McpClient, spec: RmcpTool) -> Self {
        Self { client, spec }
    }
}

#[async_trait]
impl Tool for McpTool {
    /// 工具名称
    fn name(&self) -> &str {
        self.spec.name.as_ref()
    }

    /// 工具描述
    fn description(&self) -> Option<&str> {
        self.spec.description.as_deref()
    }

    /// 工具分类（固定为 External）
    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::External]
    }

    /// 输入参数 schema
    fn input_schema(&self) -> Value {
        serde_json::to_value(self.spec.input_schema.as_ref())
            .unwrap_or_else(|_| json!({"type":"object"}))
    }

    /// 执行工具调用
    ///
    /// # 参数
    ///
    /// - `input`: 工具输入参数
    ///
    /// # 返回值
    ///
    /// - `Ok(Value)`: 执行成功，返回结果
    /// - `Err(ToolError)`: 执行失败，返回错误
    ///
    /// # 处理逻辑
    ///
    /// 1. 调用远端 MCP 工具
    /// 2. 如果有 structured_content，直接返回
    /// 3. 否则拼接文本内容返回
    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let result = self
            .client
            .call_tool(self.spec.name.as_ref(), input)
            .await
            .map_err(ToolError::Message)?;

        // 优先返回结构化内容
        if let Some(v) = result.structured_content {
            return Ok(v);
        }

        // 拼接文本内容
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
