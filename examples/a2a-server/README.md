# A2A 服务器示例 (a2a-server)

## 原理

A2A 服务器示例展示了如何实现一个支持 A2A (Agent-to-Agent) 协议的服务器。服务器提供一个可以响应客户端请求的 Agent。

## 核心组件

- **ra2a::server::ServerState**: 服务器状态管理
- **a2a_router**: A2A 协议路由
- **AgentExecutor**: Agent 执行器
- **AgentCard**: Agent 能力描述卡片

## 功能

1. **服务发现**: 通过 AgentCard 提供服务能力描述
2. **任务处理**: 接收并处理来自客户端的任务请求
3. **响应返回**: 返回处理结果或状态更新

## 内置 Agent

- **TimeAgent**: 返回当前时间
- **EchoAgent**: 回显用户消息

## 协议支持

- **JSON-RPC**: 基于 JSON-RPC 2.0 规范
- **SSE**: 支持 Server-Sent Events 进行流式更新
- **Task 模型**: 支持任务状态跟踪和更新

## 意义

1. **标准化**: 实现 A2A 协议，便于与其他 Agent 互操作
2. **分布式**: 支持分布式 Agent 架构
3. **解耦**: 服务器和客户端可以独立开发和部署

## 运行

```bash
cargo run --example a2a-server
```

服务器默认监听 `http://localhost:10000`。

## 测试

可以使用 a2a-client 示例测试服务器：

```bash
# 启动服务器后，在另一个终端运行客户端
cargo run --example a2a-client
```

## 环境变量

- `A2A_SERVER_HOST`: 服务器主机（默认 0.0.0.0）
- `A2A_SERVER_PORT`: 服务器端口（默认 10000）