# rucora MCP

rucora 的 MCP（Model Context Protocol）集成。

## 概述

本 crate 为 rucora 提供 MCP 协议集成支持，用于：
- 连接 MCP 服务器
- 将 MCP 工具转换为 rucora 的 `Tool` trait
- 统一的 MCP 工具调用接口

## 安装

```toml
[dependencies]
rucora-mcp = "0.1"
```

或通过主 rucora crate：

```toml
[dependencies]
rucora = { version = "0.1", features = ["mcp"] }
```

## 使用方式

### 连接 MCP 服务器

```rust
use rucora_mcp::{McpClient, StdioTransport};

// 创建传输层
let transport = StdioTransport::new("mcp-server");

// 创建客户端
let client = McpClient::connect(transport).await?;

// 列出可用工具
let tools = client.list_tools().await?;

for tool in tools {
    println!("工具：{}", tool.name);
}
```

### 调用 MCP 工具

```rust
use rucora_mcp::{McpClient, StdioTransport};
use serde_json::json;

let transport = StdioTransport::new("mcp-server");
let client = McpClient::connect(transport).await?;

let result = client.call_tool(
    "my_tool",
    json!({"param": "value"})
).await?;

println!("结果：{}", result);
```

### 作为 rucora 工具使用

```rust
use rucora_mcp::{McpClient, McpToolAdapter, StdioTransport};
use rucora_core::tool::Tool;

let transport = StdioTransport::new("mcp-server");
let client = McpClient::connect(transport).await?;

let tools = client.list_tools().await?;
let mcp_tool = tools.into_iter().next().unwrap();

let adapter = McpToolAdapter::new(client.clone(), mcp_tool);

// 现在可以作为 rucora 工具使用
let result = adapter.call(serde_json::json!({})).await?;
```

## 子模块

- `protocol`：MCP 协议模型类型
- `tool`：MCP 工具适配器
- `transport`：MCP 传输层（Stdio、HTTP）

## 依赖

基于 [`rmcp`](https://crates.io/crates/rmcp) 库构建。

## 许可证

MIT
