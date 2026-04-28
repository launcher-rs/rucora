# rucora Tools

rucora 的内置工具实现。

## 概述

本 crate 包含 rucora 的 16+ 种工具的具体实现。每个工具提供一种特定能力，可以在 Agent 执行期间被调用。

## 可用工具

| 工具 | 分类 | 说明 |
|------|------|------|
| EchoTool | 基础 | 回显输入内容 |
| ShellTool | 系统 | 执行 Shell 命令 |
| CmdExecTool | 系统 | 受限的命令执行 |
| GitTool | 系统 | Git 操作 |
| FileReadTool | 文件 | 读取文件内容 |
| FileWriteTool | 文件 | 写入文件内容 |
| FileEditTool | 文件 | 编辑文件内容 |
| HttpRequestTool | 网络 | HTTP 请求 |
| WebFetchTool | 网络 | 抓取网页内容 |
| WebSearchTool | 网络 | 网络搜索 |
| BrowseTool | 浏览器 | 浏览网页 |
| BrowserOpenTool | 浏览器 | 打开浏览器 |
| MemoryStoreTool | 记忆 | 存储信息 |
| MemoryRecallTool | 记忆 | 检索信息 |
| DatetimeTool | 工具 | 日期时间操作 |
| GithubTrendingTool | GitHub | GitHub 趋势 |
| SerpAPITool | 搜索 | SerpAPI 搜索 |
| TavilyTool | 搜索 | Tavily 搜索 |

## 安装

```toml
[dependencies]
rucora-tools = "0.1"
```

或通过主 rucora crate：

```toml
[dependencies]
rucora = { version = "0.1", features = ["tools"] }
```

## 使用方式

### Shell 工具

```rust
use rucora_tools::ShellTool;
use rucora_core::tool::Tool;

let tool = ShellTool::new();
let result = tool.call(serde_json::json!({
    "command": "echo hello"
})).await?;
```

### 文件工具

```rust
use rucora_tools::{FileReadTool, FileWriteTool};

let read_tool = FileReadTool::new();
let content = read_tool.call(serde_json::json!({
    "path": "/path/to/file.txt"
})).await?;
```

### HTTP 请求工具

```rust
use rucora_tools::HttpRequestTool;

let http_tool = HttpRequestTool::new();
let response = http_tool.call(serde_json::json!({
    "method": "GET",
    "url": "https://api.example.com/data"
})).await?;
```

### 记忆工具

```rust
use rucora_tools::{MemoryStoreTool, MemoryRecallTool};
use std::sync::Arc;
use rucora_core::memory::InMemoryMemory;

let memory = Arc::new(InMemoryMemory::new());
let store_tool = MemoryStoreTool::new(memory.clone());
let recall_tool = MemoryRecallTool::new(memory);
```

## Feature 配置

| Feature | 说明 |
|---------|------|
| `basic` | EchoTool（默认） |
| `system` | Shell、CmdExec、Git 工具 |
| `file` | 文件读/写/编辑工具 |
| `network` | HTTP 请求工具 |
| `browser` | 浏览和打开浏览器工具 |
| `memory` | 记忆存储和检索工具 |
| `datetime` | 日期时间工具 |
| `github` | GitHub 趋势工具 |
| `web_search` | 网络搜索工具 |
| `all` | 启用所有工具 |

## 安全说明

### ShellTool
- 命令通过 Shell 执行（Windows 使用 `cmd /C`，Linux/macOS 使用 `sh -c`）
- 默认阻止危险的 Shell 操作符
- 环境变量会被清除，只保留安全变量

### 文件工具
- 检测路径遍历攻击
- 可配置路径验证

### HTTP 工具
- URL 验证（必须以 http:// 或 https:// 开头）
- 可配置最大重定向次数
- 支持超时设置

## 许可证

MIT
