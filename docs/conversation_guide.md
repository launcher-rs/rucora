# Conversation 使用指南

## 概述

`ConversationManager` 是对话历史管理工具，用于：
- 管理多轮对话的上下文
- 自动添加消息到历史
- 窗口限制（保留最近 N 条消息）
- 与 Agent/Runtime 配合实现连续对话

## 核心概念

```
ConversationManager = 对话历史管理器（负责存储和检索）
Agent/Runtime       = 推理和执行单元（使用历史进行对话）
```

## 基本使用

### 创建对话管理器

```rust
use rucora::conversation::ConversationManager;

// 基础创建
let mut conv = ConversationManager::new();

// 带系统提示
let mut conv = ConversationManager::new()
    .with_system_prompt("你是一个有帮助的助手");

// 带消息限制
let mut conv = ConversationManager::new()
    .with_max_messages(20);  // 保留最近 20 条消息
```

### 添加消息

```rust
// 添加用户消息
conv.add_user_message("你好".to_string());

// 添加助手回复
conv.add_assistant_message("你好！有什么可以帮助你的？".to_string());

// 添加系统消息（一般不手动调用）
conv.add_system_message("系统提示".to_string());
```

### 获取消息历史

```rust
// 获取所有消息（用于 API 调用）
let messages = conv.get_messages();

for msg in messages {
    println!("{}: {}", 
        match msg.role {
            Role::System => "系统",
            Role::User => "用户",
            Role::Assistant => "助手",
            Role::Tool => "工具",
        },
        msg.content
    );
}
```

## 与 Runtime 配合使用

### 方式 1：每次手动传递历史

```rust
use rucora::conversation::ConversationManager;
use rucora::prelude::*;
use rucora::provider::OpenAiProvider;
use rucora::runtime::DefaultRuntime;

let provider = OpenAiProvider::from_env()?;
let runtime = DefaultRuntime::new(
    Arc::new(provider),
    ToolRegistry::new(),
    "qwen3.5:9b",
);

let mut conv = ConversationManager::new()
    .with_system_prompt("你是项目助手");

// 第 1 轮
conv.add_user_message("rucora 是什么？".to_string());
let input = AgentInput::new("rucora 是什么？");
let output = runtime.run(input).await?;
conv.add_assistant_message(output.text().unwrap().to_string());

// 第 2 轮（带上下文）
conv.add_user_message("它支持哪些工具？".to_string());
let input = AgentInput::new("它支持哪些工具？");
let output = runtime.run(input).await?;
```

**注意**：Runtime 本身不维护对话历史，每次请求是独立的。如需多轮对话，需要手动管理历史。

## 与 Agent 配合使用

### 方式 1：简单对话（无历史）

```rust
use rucora::prelude::*;
use rucora::provider::OpenAiProvider;
use rucora::agent::DefaultAgent;

let provider = OpenAiProvider::from_env()?;
let agent = DefaultAgent::builder()
    .provider(provider)
    .model("qwen3.5:9b")
    .build();

// 每次对话是独立的
let output = agent.run("你好").await?;
```

### 方式 2：配合 ConversationManager（推荐）

```rust
use rucora::conversation::ConversationManager;
use rucora::prelude::*;
use rucora::provider::OpenAiProvider;
use rucora::agent::DefaultAgent;
use rucora_core::provider::types::Role;

let provider = OpenAiProvider::from_env()?;
let mut conv = ConversationManager::new()
    .with_system_prompt("你是个人助手");

let agent = DefaultAgent::builder()
    .provider(provider)
    .model("qwen3.5:9b")
    .build();

// 多轮对话
let conversations = vec![
    "你好，我叫小明",
    "我今年 25 岁",
    "你还记得我叫什么吗？",  // 应该回答：小明
];

for user_input in conversations {
    // 1. 添加用户消息到历史
    conv.add_user_message(user_input.to_string());
    
    // 2. 构建包含历史的输入
    let messages = conv.get_messages();
    let mut full_input = String::new();
    
    for msg in messages.iter().filter(|m| m.role != Role::System) {
        let role = match msg.role {
            Role::User => "用户",
            Role::Assistant => "助手",
            _ => "",
        };
        if !role.is_empty() {
            full_input.push_str(&format!("{}: {}\n", role, msg.content));
        }
    }
    full_input.push_str("助手：");
    
    // 3. 运行 Agent
    let output = agent.run(AgentInput::new(full_input)).await?;
    if let Some(content) = output.text() {
        println!("助手：{}", content);
        // 4. 添加助手回复到历史
        conv.add_assistant_message(content.to_string());
    }
}
```

## 完整示例

### 示例 1：基础对话管理

```rust
use rucora::conversation::ConversationManager;
use rucora_core::provider::types::Role;

let mut conv = ConversationManager::new()
    .with_system_prompt("你是有帮助的助手")
    .with_max_messages(20);

// 添加对话
conv.add_user_message("你好".to_string());
conv.add_assistant_message("你好！有什么可以帮助你的？".to_string());

// 查看历史
let messages = conv.get_messages();
println!("总消息数：{}", messages.len());

for msg in messages {
    let role = match msg.role {
        Role::System => "系统",
        Role::User => "用户",
        Role::Assistant => "助手",
        Role::Tool => "工具",
    };
    println!("{}: {}", role, msg.content.chars().take(30).collect::<String>());
}
```

### 示例 2：Runtime 多轮对话

```rust
use rucora::conversation::ConversationManager;
use rucora::prelude::*;
use rucora::provider::OpenAiProvider;
use rucora::runtime::DefaultRuntime;

let provider = OpenAiProvider::from_env()?;
let runtime = DefaultRuntime::new(
    Arc::new(provider),
    ToolRegistry::new(),
    "qwen3.5:9b",
);

let mut conv = ConversationManager::new()
    .with_system_prompt("你是项目助手");

// 第 1 轮
println!("用户：rucora 是什么？");
conv.add_user_message("rucora 是什么？".to_string());
let output = runtime.run(AgentInput::new("rucora 是什么？")).await?;
if let Some(content) = output.text() {
    println!("助手：{}", content);
    conv.add_assistant_message(content.to_string());
}

// 第 2 轮
println!("\n用户：它支持哪些 Provider？");
conv.add_user_message("它支持哪些 Provider？".to_string());
let output = runtime.run(AgentInput::new("它支持哪些 Provider？")).await?;
if let Some(content) = output.text() {
    println!("助手：{}", content);
    conv.add_assistant_message(content.to_string());
}
```

### 示例 3：Agent 多轮对话（推荐方式）

```rust
use rucora::conversation::ConversationManager;
use rucora::prelude::*;
use rucora::provider::OpenAiProvider;
use rucora::agent::DefaultAgent;
use rucora_core::provider::types::Role;

let provider = OpenAiProvider::from_env()?;
let mut conv = ConversationManager::new()
    .with_system_prompt("你是贴心的个人助手")
    .with_max_messages(20);

let agent = DefaultAgent::builder()
    .provider(provider)
    .model("qwen3.5:9b")
    .build();

// 多轮对话
let inputs = vec![
    "你好，我叫张三",
    "我是一名软件工程师",
    "我平时喜欢用 Rust 编程",
    "你还记得我叫什么吗？",
    "我是做什么工作的？",
];

for (i, user_input) in inputs.iter().enumerate() {
    println!("\n【第 {} 轮】", i + 1);
    println!("用户：{}", user_input);
    
    // 添加用户消息
    conv.add_user_message(user_input.to_string());
    
    // 构建带历史的输入
    let messages = conv.get_messages();
    let mut context = String::new();
    
    for msg in messages.iter().filter(|m| m.role != Role::System) {
        let role = match msg.role {
            Role::User => "用户",
            Role::Assistant => "助手",
            _ => "",
        };
        if !role.is_empty() {
            context.push_str(&format!("{}: {}\n", role, msg.content));
        }
    }
    context.push_str("助手：");
    
    // 运行 Agent
    let output = agent.run(AgentInput::new(context)).await?;
    if let Some(content) = output.text() {
        println!("助手：{}", content);
        conv.add_assistant_message(content.to_string());
    }
}
```

## 高级功能

### 消息窗口限制

```rust
// 保留最近 20 条消息
let mut conv = ConversationManager::new()
    .with_max_messages(20);

// 当超过限制时，自动删除最早的消息
for i in 0..30 {
    conv.add_user_message(format!("消息 {}", i));
    conv.add_assistant_message(format!("回复 {}", i));
}

// 总消息数不会超过 20 + 1（系统提示）
println!("消息数：{}", conv.get_messages().len());
```

### Token 限制

```rust
// 保留最近 4000 个 token
let mut conv = ConversationManager::new()
    .with_max_tokens(4000);
```

### 清空历史

```rust
conv.clear();  // 清空所有消息
```

### 获取系统提示

```rust
if let Some(prompt) = conv.system_prompt() {
    println!("系统提示：{}", prompt);
}
```

## 最佳实践

### 1. 选择合适的消息限制

```rust
// 短对话：10-20 条消息
let conv = ConversationManager::new().with_max_messages(15);

// 长对话：50-100 条消息
let conv = ConversationManager::new().with_max_messages(50);
```

### 2. 使用系统提示定义角色

```rust
let conv = ConversationManager::new()
    .with_system_prompt("你是专业的客服助手，回答要简洁专业");
```

### 3. 定期清理历史

```rust
// 每 10 轮对话后清理一次
if conversation_count % 10 == 0 {
    conv.clear();
    conv.ensure_system_prompt("你是...");
}
```

### 4. 保存和加载对话

```rust
use serde_json;

// 保存
let json = serde_json::to_string(&conv)?;
std::fs::write("conversation.json", json)?;

// 加载
let json = std::fs::read_to_string("conversation.json")?;
let conv: ConversationManager = serde_json::from_str(&json)?;
```

## 运行示例

```bash
# 设置环境变量
export OPENAI_API_KEY=sk-your-key
export OPENAI_BASE_URL=http://your-server:11434/v1

# 运行示例
cargo run --example 05_conversation -p rucora
```

## 常见问题

### Q: Runtime 和 Agent 本身不维护对话历史吗？

A: 不维护。Runtime 和 Agent 是无状态的，每次请求独立。需要使用 `ConversationManager` 来管理历史。

### Q: 如何避免 token 超限？

A: 使用 `with_max_tokens()` 或 `with_max_messages()` 限制历史长度。

### Q: 可以在不同 Agent 之间共享对话历史吗？

A: 可以。`ConversationManager` 是独立的，可以传递给多个 Agent 使用。

### Q: 系统提示会被发送到 API 吗？

A: 会。`get_messages()` 返回的消息包含系统提示（如果有）。
