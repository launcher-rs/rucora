# AgentKit A2A

A2A (Agent-to-Agent) protocol integration for AgentKit.

## Overview

This crate provides A2A protocol support for AgentKit, enabling:
- Agent-to-agent communication and collaboration
- Task delegation and result return
- Multi-agent system orchestration

## Installation

```toml
[dependencies]
agentkit-a2a = "0.1"
```

Or via the main AgentKit crate:

```toml
[dependencies]
agentkit = { version = "0.1", features = ["a2a"] }
```

## Usage

### Client Usage

```rust
use agentkit_a2a::client::Client;

// Connect to Agent server
let client = Client::connect("http://agent-server:8080").await?;

// Send task
let task = client.send_task("process_data", "input data").await?;

// Wait for result
let result = client.wait_for_result(&task.id).await?;
```

### Server Usage

```rust
use agentkit_a2a::server::Server;

let mut server = Server::new();

server.register_handler("analyze", |task| async move {
    let result = format!("Analysis of: {}", task.input);
    Ok(TaskResult {
        task_id: task.id,
        output: result,
        status: TaskStatus::Completed,
    })
});

server.bind("127.0.0.1:8080").await?.serve().await?;
```

### Multi-Agent Collaboration

```rust
use agentkit_a2a::client::Client;

let agent1 = Client::connect("http://agent1:8080").await?;
let agent2 = Client::connect("http://agent2:8080").await?;

// Delegate task to Agent 1
let task1 = agent1.send_task("fetch_data", "").await?;
let data = agent1.wait_for_result(&task1.id).await?;

// Pass result to Agent 2
let task2 = agent2.send_task("process_data", &data.output).await?;
let result = agent2.wait_for_result(&task2.id).await?;
```

### A2A Tool Adapter

```rust
use agentkit_a2a::A2AToolAdapter;
use agentkit_core::tool::Tool;

let adapter = A2AToolAdapter::new(
    "remote_agent".to_string(),
    "Call remote agent".to_string(),
    serde_json::json!({}),
    client,
);

let result = adapter.call(serde_json::json!({"message": "Hello"})).await?;
```

## Submodules

- `protocol`: A2A protocol model definitions
- `transport`: A2A transport layer
- `types`: A2A protocol types (from `ra2a::types`)

## Dependencies

Built on [`ra2a`](https://crates.io/crates/ra2a) library.

## License

MIT
