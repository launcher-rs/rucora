# A2A 客户端示例 (a2a-client)

## 原理

A2A (Agent-to-Agent) 客户端示例展示了如何通过 A2A 协议与远程 Agent 服务器通信。A2A 是 Google 提出的代理间通信协议，允许不同 Agent 之间进行互操作。

## 核心组件

- **ra2a::client::Client**: A2A 协议客户端
- **A2AToolAdapter**: 将 A2A 服务适配为 rucora 工具
- **ToolRegistry**: 工具注册表

## 通信流程

1. **连接服务器**: 通过 HTTP 连接到 A2A 服务器
2. **发送请求**: 使用 JSON-RPC 格式发送任务请求
3. **接收响应**: 接收 Agent 返回的结果
4. **工具调用**: 将 A2A 服务包装为工具供 rucora Agent 使用

## 意义

1. **互操作性**: 与其他实现 A2A 协议的 Agent 通信
2. **分布式**: 支持分布式 Agent 架构
3. **标准化**: 基于 JSON-RPC 的标准化通信

## 运行

```bash
# 首先启动 A2A server
cargo run --example a2a-server

# 然后运行客户端
cargo run --example a2a-client
```

## 环境变量

- `OLLAMA_BASE_URL`: Ollama 服务器地址（默认 http://localhost:11434）
- `A2A_SERVER_URL`: A2A 服务器地址（默认 http://localhost:10000）

## 扩展

可以扩展支持更多 A2A 能力，如流式响应、任务队列等。