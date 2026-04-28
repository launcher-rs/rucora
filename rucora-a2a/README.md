# rucora A2A

rucora 的 A2A（Agent-to-Agent）协议集成。

## 概述

本 crate 为 rucora 提供 A2A 协议支持，用于：
- Agent 之间的通信与协作
- 任务委托与结果返回
- 多 Agent 系统编排

## 安装

```toml
[dependencies]
rucora-a2a = "0.1"
```

或通过主 rucora crate：

```toml
[dependencies]
rucora = { version = "0.1", features = ["a2a"] }
```

## 使用方式

### 客户端使用

```rust
use rucora_a2a::client::Client;

// 连接到 Agent 服务器
let client = Client::connect("http://agent-server:8080").await?;

// 发送任务
let task = client.send_task("process_data", "input data").await?;

// 等待结果
let result = client.wait_for_result(&task.id).await?;
```

### 服务端使用

```rust
use rucora_a2a::server::Server;

let mut server = Server::new();

server.register_handler("analyze", |task| async move {
    let result = format!("分析结果：{}", task.input);
    Ok(TaskResult {
        task_id: task.id,
        output: result,
        status: TaskStatus::Completed,
    })
});

server.bind("127.0.0.1:8080").await?.serve().await?;
```

### 多 Agent 协作

```rust
use rucora_a2a::client::Client;

let agent1 = Client::connect("http://agent1:8080").await?;
let agent2 = Client::connect("http://agent2:8080").await?;

// 将任务委托给 Agent 1
let task1 = agent1.send_task("fetch_data", "").await?;
let data = agent1.wait_for_result(&task1.id).await?;

// 将结果传递给 Agent 2 处理
let task2 = agent2.send_task("process_data", &data.output).await?;
let result = agent2.wait_for_result(&task2.id).await?;
```

### A2A 工具适配器

```rust
use rucora_a2a::A2AToolAdapter;
use rucora_core::tool::Tool;

let adapter = A2AToolAdapter::new(
    "remote_agent".to_string(),
    "调用远程 Agent".to_string(),
    serde_json::json!({}),
    client,
);

let result = adapter.call(serde_json::json!({"message": "你好"})).await?;
```

## 子模块

- `protocol`：A2A 协议模型定义
- `transport`：A2A 传输层
- `types`：A2A 协议类型（来自 `ra2a::types`）

## 依赖

基于 [`ra2a`](https://crates.io/crates/ra2a) 库构建。

## 许可证

MIT
