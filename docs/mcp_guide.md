# MCP (Model Context Protocol) 集成指南

AgentKit 提供了 MCP (Model Context Protocol) 集成，允许 Agent 通过标准协议调用远程工具。

## 概述

MCP 是一个开放协议，标准化了 AI 模型如何与外部工具交互。AgentKit 的 `agentkit-mcp` crate 提供了以下功能：

- **连接 MCP 服务器** - 远程或本地服务，通过 MCP 协议暴露工具
- **转换 MCP 工具为 AgentKit 工具** - 远程工具可以无缝与 Agent 系统一起使用
- **统一的工具调用接口** - Agent 无需知道工具是本地还是远程

该模块基于 [`rmcp`](https://crates.io/crates/rmcp) crate（版本 1.3）构建，这是 MCP 协议的 Rust 实现。

## 架构

```
+------------------+      +-------------------+      +------------------+
|   ToolAgent      |      |   ToolRegistry    |      |   McpTool        |
| (qwen3.5 / gpt)  |----->| (manages tools)   |----->| (adapts MCP tool |
|                  |      |                   |      |  to Tool trait)  |
+------------------+      +-------------------+      +--------+---------+
                                                          |
                                                          v
                                                  +------------------+
                                                  |   McpClient      |
                                                  | (RPC over rmcp)  |
                                                  +--------+---------+
                                                           |
                                      +--------------------+--------------------+
                                      |                                         |
                                      v                                         v
                          +------------------------+              +------------------------+
                          | StdioTransport         |              | StreamableHttpClient   |
                          | (local subprocess)     |              | (HTTP/SSE remote)      |
                          +------------------------+              +------------------------+
                                      |                                         |
                                      v                                         v
                          +------------------------+              +------------------------+
                          | MCP Server (local)     |              | MCP Server (remote)    |
                          | e.g. filesystem, git   |              | e.g. custom MCP API    |
                          +------------------------+              +------------------------+
```

## 核心类型

### McpClient

```rust
#[derive(Clone)]
pub struct McpClient {
    client: Arc<RunningService<RoleClient, InitializeRequestParams>>,
}
```

`rmcp` 的 `RunningService` 的轻量包装器。

**方法：**

| 方法 | 签名 | 用途 |
|------|------|------|
| `new` | `pub fn new(service: RunningService<RoleClient, InitializeRequestParams>) -> Self` | 从已初始化的服务构建 |
| `list_tools` | `pub async fn list_tools(&self) -> Result<Vec<RmcpTool>, String>` | 列出远程 MCP 服务器的所有工具 |
| `call_tool` | `pub async fn call_tool(&self, name: &str, input: Value) -> Result<CallToolResult, String>` | 调用特定的远程工具 |

**实现细节：**
- `list_tools()` 调用 `self.peer().list_all_tools()` 并通过 `tracing` 记录计时和工具名称
- `call_tool()` 将 `serde_json::Value` 输入转换为 MCP 的 `CallToolRequestParams`，处理 `Null` 输入时发送 `None` 参数
- 两个方法都在跟踪日志中截断大型输入/结果（输入 800 字符，结果 1200 字符）

### McpTool（适配器）

```rust
pub struct McpTool {
    client: McpClient,
    spec: RmcpTool,
}
```

**适配器模式**实现。包装远程 MCP 工具定义并实现 `agentkit_core::tool::Tool` trait，使远程工具与本地工具无法区分。

**Tool trait 实现：**

| 方法 | 实现 |
|------|------|
| `fn name(&self) -> &str` | 返回 `self.spec.name` |
| `fn description(&self) -> Option<&str>` | 返回 `self.spec.description` |
| `fn categories(&self) -> &'static [ToolCategory]` | 始终返回 `[ToolCategory::External]` |
| `fn input_schema(&self) -> Value` | 将 `self.spec.input_schema` 转换为 `serde_json::Value` |
| `async fn call(&self, input: Value) -> Result<Value, ToolError>` | 委托给 `McpClient::call_tool()` |

**`call()` 方法逻辑：**
1. 通过 `self.client.call_tool()` 调用远程 MCP 工具
2. 如果 `structured_content` 存在，直接作为 JSON 返回
3. 否则，遍历 `result.content`，提取所有 `Text` 片段，用换行符连接，返回 `{"content": text}`

## 传输机制

MCP 支持多种传输方式：

| 传输 | 用途 | 构造函数 |
|------|------|---------|
| `StdioTransport` | 本地子进程通信 | `StdioTransport::new("command")` |
| `StreamableHttpTransport` | HTTP 流（基于 SSE） | `StreamableHttpTransport::new("http://host:port")` |
| `StreamableHttpClientTransport` | 带自定义配置的 HTTP 客户端 | `StreamableHttpClientTransport::from_config(config)` |
| `HttpClientTransport` | 基本 HTTP 客户端 | 通过 rmcp::transport 可用 |

## 连接流程

```
1. 创建传输（StdioTransport 或 StreamableHttpClientTransport）
   |
   v
2. 创建带 ClientCapabilities + Implementation 的 ClientInfo
   |
   v
3. 调用 client_info.serve(transport).await  -- 返回 RunningService
   |
   v
4. 包装到 McpClient::new(service)
   |
   v
5. 调用 mcp_client.list_tools().await  -- 获取 Vec<RmcpTool>
   |
   v
6. 对每个 RmcpTool：McpTool::new(mcp_client.clone(), spec)
   |
   v
7. 注册到 ToolRegistry：registry.register_arc(Arc::new(mcp_tool))
   |
   v
8. 将 ToolRegistry 传递给 ToolAgent
```

## 使用示例

### 示例 1：HTTP 流传输

```rust
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 步骤 1：配置 MCP 传输（带认证头）
    let mcp_url = std::env::var("MCP_URL").unwrap_or("http://127.0.0.1:8000/mcp".to_string());
    let bearer = std::env::var("MCP_BEARER_TOKEN").unwrap_or("dummy_token".to_string());

    let mut headers = HashMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {bearer}")).unwrap(),
    );
    let config = StreamableHttpClientTransportConfig::with_uri(mcp_url).custom_headers(headers);
    let transport = StreamableHttpClientTransport::from_config(config);

    // 步骤 2：创建客户端信息并连接
    let client_info = ClientInfo::new(
        ClientCapabilities::default(),
        Implementation::new("agentkit", "0.1.0"),
    );
    let service = client_info.serve(transport).await?;
    let mcp_client = McpClient::new(service);

    // 步骤 3：列出可用工具
    let specs = mcp_client.list_tools().await.map_err(|e| anyhow::anyhow!(e))?;

    // 步骤 4：注册 MCP 工具到 ToolRegistry
    let mut tools = agentkit::agent::ToolRegistry::new();
    for spec in specs {
        tools = tools.register_arc(Arc::new(McpTool::new(mcp_client.clone(), spec)));
    }

    // 步骤 5：创建 LLM 提供者
    let provider = OpenAiProvider::from_env()?;

    // 步骤 6：创建带 MCP 工具的 Agent
    let agent = ToolAgent::builder()
        .provider(provider)
        .model("qwen3.5:9b")
        .system_prompt("你是一个严谨的助手，擅长使用各种工具完成任务。")
        .tool_registry(tools)
        .max_steps(6)
        .build();

    // 步骤 7：运行 Agent
    let output = agent.run("今天几号？".into()).await?;
    println!("{}", output.text().unwrap_or("No response"));

    // 步骤 8：清理
    drop(mcp_client);
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    Ok(())
}
```

### 示例 2：Stdio 传输（简化版）

```rust
use agentkit::mcp::{McpClient, McpTool, StdioTransport, ServiceExt};
use agentkit::protocol::{ClientCapabilities, ClientInfo, Implementation};
use agentkit_core::tool::Tool;
use std::sync::Arc;

#[tokio::main]
async fn example() -> Result<(), Box<dyn std::error::Error>> {
    // 创建传输
    let transport = StdioTransport::new("mcp-server");

    // 创建客户端信息
    let client_info = ClientInfo::new(
        ClientCapabilities::default(),
        Implementation::new("agentkit", "0.1.0"),
    );

    // 连接
    let service = client_info.serve(transport).await?;
    let client = McpClient::new(service);

    // 列出工具
    let tools = client.list_tools().await?;
    let mcp_tool_spec = tools.into_iter().next().unwrap();

    // 创建适配器
    let adapter = McpTool::new(client.clone(), mcp_tool_spec);

    // 作为 agentkit 工具调用
    let result = adapter.call(serde_json::json!({})).await?;
    println!("结果：{}", result);

    Ok(())
}
```

## 与 Agent 系统集成

### ToolRegistry 集成

MCP 工具通过 `ToolSource::Mcp` 注册：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolSource {
    BuiltIn,
    Skill,
    Mcp,       // <-- MCP 工具使用此来源
    A2A,
    Custom,
}
```

集成流程：

1. `McpTool` 实现 `agentkit_core::tool::Tool`，因此可以包装在 `Arc<dyn Tool>` 中
2. `ToolRegistry::register_arc()` 接受 Arc 包装的工具
3. `ToolRegistry::definitions()` 为每个启用的工具提取 `ToolDefinition`（名称、描述、输入模式）- 这些发送给 LLM 的函数调用机制
4. `ToolRegistry::enabled_tools()` 返回所有启用的工具用于执行
5. `ToolAgent` 使用注册表：
   - 向 LLM 提供者发送工具定义
   - 从 LLM 接收工具调用请求
   - 查找并执行工具（包括 `McpTool` 实例）
   - 将结果返回给 LLM

### 在 Agent 中使用

```rust
use agentkit::agent::{ToolAgent, ToolRegistry};
use agentkit::mcp::tool::McpTool;
use std::sync::Arc;

// 假设已有 mcp_client 和工具规范
let mut registry = ToolRegistry::new();

for spec in mcp_tools {
    registry = registry.register_arc(Arc::new(McpTool::new(mcp_client.clone(), spec)));
}

let agent = ToolAgent::builder()
    .provider(provider)
    .model("gpt-4o-mini")
    .tool_registry(registry)
    .build();
```

## 协议类型

`agentkit-mcp` 重新导出了 `rmcp::model` 中的所有类型：

```rust
pub use rmcp::model::*;
```

**常用类型：**

| 类型 | 用途 |
|------|------|
| `Tool` (RmcpTool) | MCP 工具定义，包含 `name`、`description`、`input_schema` |
| `CallToolResult` | 调用 MCP 工具的结果，包含 `content` 和 `structured_content` |
| `CallToolRequest` | 调用工具的请求 |
| `InitializeRequest` | MCP 初始化请求 |
| `ClientInfo` | 初始化期间发送的客户端身份 |
| `ClientCapabilities` | 客户端能力声明 |
| `Implementation` | 客户端实现信息（名称、版本） |
| `RawContent` | 内容包装器（Text、Image 等） |
| `JsonObject` | MCP 的 JSON 对象类型 |

## 启用 MCP 功能

MCP 模块在 `mcp` 功能标志后面：

```toml
[dependencies]
agentkit = { version = "0.1", features = ["mcp"] }
```

或使用所有功能：

```toml
[dependencies]
agentkit = { version = "0.1", features = ["full"] }
```

运行示例：

```bash
cargo run --example 12_mcp --features mcp
```

## 最佳实践

1. **为每个工具克隆 McpClient** - 由于 `McpClient` 是 `Clone`（包装 `Arc`），在创建多个 `McpTool` 实例时克隆它，以便它们都共享相同的连接。

2. **使用 `ToolCategory::External`** - `McpTool` 自动将工具分类为 `External`。在注册表中使用 `ToolSource::Mcp` 跟踪来源。

3. **优雅关闭** - 调用 `drop(mcp_client)` 然后短暂 `tokio::time::sleep`（100ms）以允许 rmcp 正确清理连接。

4. **错误处理** - `list_tools()` 和 `call_tool()` 都返回 `Result<..., String>`。转换为 `anyhow::Error` 或显式处理。

5. **输入截断感知** - `call_tool` 方法在跟踪日志中截断大型输入。实际工具调用发送完整输入。

6. **结果处理** - `McpTool::call()` 优先使用 `structured_content` 而非原始文本内容。如果需要原始文本，直接使用 `McpClient::call_tool()`。

7. **功能门控** - MCP 模块在 `mcp` 功能标志后面。构建/运行 MCP 相关代码时始终使用 `--features mcp`。

8. **验证连接** - 连接后调用 `list_tools()` 验证与 MCP 服务器的连接。

## 相关文件

- `agentkit-mcp/src/lib.rs` - 模块入口
- `agentkit-mcp/src/tool.rs` - McpClient 和 McpTool 适配器
- `agentkit-mcp/src/transport.rs` - 传输机制
- `agentkit-mcp/src/protocol.rs` - 协议类型
- `agentkit-mcp/Cargo.toml` - Crate 依赖
- `agentkit/examples/12_mcp.rs` - MCP 集成示例
- `agentkit/src/agent/tool_registry.rs` - 工具注册表（含 ToolSource::Mcp）
