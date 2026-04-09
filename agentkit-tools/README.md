# AgentKit Tools

Built-in tools for AgentKit.

## Overview

This crate contains 16+ concrete tool implementations for AgentKit. Each tool provides a specific capability that can be called by agents during execution.

## Available Tools

| Tool | Category | Description |
|------|----------|-------------|
| EchoTool | Basic | Echo input content |
| ShellTool | System | Execute shell commands |
| CmdExecTool | System | Restricted command execution |
| GitTool | System | Git operations |
| FileReadTool | File | Read file contents |
| FileWriteTool | File | Write to files |
| FileEditTool | File | Edit file contents |
| HttpRequestTool | Network | HTTP requests |
| WebFetchTool | Network | Fetch web content |
| WebSearchTool | Network | Web search |
| BrowseTool | Browser | Browse web pages |
| BrowserOpenTool | Browser | Open browser |
| MemoryStoreTool | Memory | Store information |
| MemoryRecallTool | Memory | Recall information |
| DatetimeTool | Utility | Date and time operations |
| GithubTrendingTool | GitHub | GitHub trending |
| SerpAPITool | Search | SerpAPI search |
| TavilyTool | Search | Tavily search |

## Installation

```toml
[dependencies]
agentkit-tools = "0.1"
```

Or via the main AgentKit crate:

```toml
[dependencies]
agentkit = { version = "0.1", features = ["tools"] }
```

## Usage

### Shell Tool

```rust
use agentkit_tools::ShellTool;
use agentkit_core::tool::Tool;

let tool = ShellTool::new();
let result = tool.call(serde_json::json!({
    "command": "echo hello"
})).await?;
```

### File Tools

```rust
use agentkit_tools::{FileReadTool, FileWriteTool};

let read_tool = FileReadTool::new();
let content = read_tool.call(serde_json::json!({
    "path": "/path/to/file.txt"
})).await?;
```

### HTTP Request Tool

```rust
use agentkit_tools::HttpRequestTool;

let http_tool = HttpRequestTool::new();
let response = http_tool.call(serde_json::json!({
    "method": "GET",
    "url": "https://api.example.com/data"
})).await?;
```

### Memory Tools

```rust
use agentkit_tools::{MemoryStoreTool, MemoryRecallTool};
use std::sync::Arc;
use agentkit_core::memory::InMemoryMemory;

let memory = Arc::new(InMemoryMemory::new());
let store_tool = MemoryStoreTool::new(memory.clone());
let recall_tool = MemoryRecallTool::new(memory);
```

## Features

| Feature | Description |
|---------|-------------|
| `basic` | EchoTool (default) |
| `system` | Shell, CmdExec, Git tools |
| `file` | File read/write/edit tools |
| `network` | HTTP request tools |
| `browser` | Browse and browser open tools |
| `memory` | Memory store and recall tools |
| `datetime` | Datetime tool |
| `github` | GitHub trending tool |
| `web_search` | Web search tools |
| `all` | Enable all tools |

## Security Notes

### ShellTool
- Commands are executed through shell (`cmd /C` on Windows, `sh -c` on Linux/macOS)
- Dangerous shell operators are blocked by default
- Environment variables are cleared except for safe ones

### File Tools
- Path traversal attacks are detected
- Path validation against allowed directories (if configured)

### HTTP Tools
- URL validation (must start with http:// or https://)
- Configurable max redirects
- Timeout support

## License

MIT
