# AgentKit MCP

MCP (Model Context Protocol) integration for AgentKit.

## Overview

This crate provides MCP protocol integration for AgentKit, enabling:
- Connecting to MCP servers
- Converting MCP tools to AgentKit's `Tool` trait
- Unified MCP tool invocation interface

## Installation

```toml
[dependencies]
agentkit-mcp = "0.1"
```

Or via the main AgentKit crate:

```toml
[dependencies]
agentkit = { version = "0.1", features = ["mcp"] }
```

## Usage

### Connect to MCP Server

```rust
use agentkit_mcp::{McpClient, StdioTransport};

// Create transport layer
let transport = StdioTransport::new("mcp-server");

// Create client
let client = McpClient::connect(transport).await?;

// List available tools
let tools = client.list_tools().await?;

for tool in tools {
    println!("Tool: {}", tool.name);
}
```

### Call MCP Tool

```rust
use agentkit_mcp::{McpClient, StdioTransport};
use serde_json::json;

let transport = StdioTransport::new("mcp-server");
let client = McpClient::connect(transport).await?;

let result = client.call_tool(
    "my_tool",
    json!({"param": "value"})
).await?;

println!("Result: {}", result);
```

### Use as AgentKit Tool

```rust
use agentkit_mcp::{McpClient, McpToolAdapter, StdioTransport};
use agentkit_core::tool::Tool;

let transport = StdioTransport::new("mcp-server");
let client = McpClient::connect(transport).await?;

let tools = client.list_tools().await?;
let mcp_tool = tools.into_iter().next().unwrap();

let adapter = McpToolAdapter::new(client.clone(), mcp_tool);

let result = adapter.call(serde_json::json!({})).await?;
```

## Submodules

- `protocol`: MCP protocol model types
- `tool`: MCP tool adapter
- `transport`: MCP transport layers (Stdio, HTTP)

## Dependencies

Built on [`rmcp`](https://crates.io/crates/rmcp) library.

## License

MIT
