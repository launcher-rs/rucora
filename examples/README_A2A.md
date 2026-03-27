# A2A 示例 - AgentKit

本目录包含两个 A2A（Agent-to-Agent）协议示例：

## 示例说明

### a2a-server

A2A 服务器示例，实现一个简单的时间助手 Agent：
- 回答当前时间
- 回应用户消息

**功能特点：**
- 使用 `ra2a` 库实现 A2A 协议服务器
- 内置时间查询功能
- 支持通过 A2A 协议接收和处理消息

### a2a-client

A2A 客户端示例，演示如何使用 agentkit 调用远程 A2A Agent：
- 通过 A2A 协议连接远程时间助手
- 使用 agentkit 的 Agent 调用 A2A 工具
- 询问当前时间并显示结果

**功能特点：**
- 使用 `A2AToolAdapter` 将远程 Agent 适配为本地工具
- 集成 Ollama (qwen2.5:7b) 作为 LLM
- 自动调用远程时间助手

## 运行方式

### 1. 启动 A2A Server

在第一个终端窗口运行：

```bash
cd agentkit/agentkit/examples/a2a-server
cargo run
```

服务器将在 `http://localhost:8080` 启动。

### 2. 运行 A2A Client

在第二个终端窗口运行：

```bash
cd agentkit/agentkit/examples/a2a-client
cargo run
```

客户端将：
1. 连接到 A2A Server
2. 询问"现在几点了？"
3. 通过 A2A 协议获取时间并显示

## 前提条件

### Ollama 设置

客户端需要 Ollama 服务：

```bash
# 安装 Ollama
# 访问 https://ollama.ai 下载安装

# 拉取模型
ollama pull qwen2.5:7b

# 启动服务（通常自动启动）
ollama serve
```

### 环境变量

无需特殊环境变量配置。

## 技术栈

- **ra2a**: A2A 协议实现
- **agentkit**: Agent 开发框架
- **tokio**: 异步运行时
- **axum**: Web 框架（服务器）
- **Ollama**: 本地 LLM 运行

## 项目结构

```
a2a-server/
├── Cargo.toml
└── src/
    └── main.rs          # 服务器实现

a2a-client/
├── Cargo.toml
└── src/
    └── main.rs          # 客户端实现
```

## 通信流程

```
┌─────────────┐      ┌─────────────┐      ┌─────────────┐
│   用户问    │ ───> │   Agent     │ ───> │  A2A Tool   │
│ "现在几点了？" │      │ (qwen2.5)   │      │  Adapter    │
└─────────────┘      └─────────────┘      └──────┬──────┘
                                                  │
                                                  │ A2A 协议
                                                  │
                                         ┌────────▼────────┐
                                         │   A2A Server    │
                                         │  (时间助手)      │
                                         │                 │
                                         │  返回当前时间    │
                                         └─────────────────┘
```

## 扩展示例

### 添加新的 A2A 功能

在 server 中添加新的消息处理逻辑：

```rust
impl TimeAgent {
    fn process_message(input: &str) -> String {
        let lower = input.to_lowercase();
        
        if lower.contains("时间") || lower.contains("几点") {
            format!("现在的时间是：{}", Self::get_current_time())
        } else if lower.contains("日期") {
            format!("今天的日期是：{}", Local::now().format("%Y 年%m 月%d 日"))
        } else {
            format!("收到：{}", input)
        }
    }
}
```

### 添加多个 A2A 工具

在 client 中注册多个 A2A 工具：

```rust
// 时间助手
let time_client = Client::from_url("http://localhost:8080")?;
let time_tool = A2AToolAdapter::new(
    "time_agent".to_string(),
    "查询当前时间".to_string(),
    parameters,
    time_client,
);

// 日期助手
let date_client = Client::from_url("http://localhost:8081")?;
let date_tool = A2AToolAdapter::new(
    "date_agent".to_string(),
    "查询当前日期".to_string(),
    parameters,
    date_client,
);

// 注册两个工具
let tools = ToolRegistry::new()
    .register(time_tool)
    .register(date_tool);
```

## 故障排除

### 无法连接到 A2A Server

确保服务器已启动并监听正确端口：

```bash
# 检查服务器是否运行
netstat -an | findstr :8080
```

### Ollama 连接失败

确保 Ollama 服务正在运行：

```bash
# 检查服务状态
curl http://localhost:11434/api/tags
```

### 模型未找到

拉取所需的模型：

```bash
ollama pull qwen2.5:7b
```

## 相关资源

- [ra2a 库文档](https://github.com/qntx/ra2a)
- [AgentKit 文档](../../readme.md)
- [A2A 协议规范](https://github.com/google/A2A)
