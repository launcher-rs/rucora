# Agent 自动对话历史管理

## 概述

Agent 现在内置了对话历史管理功能，无需手动管理 `ConversationManager`。启用后，Agent 会自动：
1. 保存用户消息到对话历史
2. 保存助手回复到对话历史
3. 在下次调用时自动包含历史记录

## 快速开始

### 启用对话历史

```rust
use agentkit::agent::DefaultAgent;
use agentkit::provider::OpenAiProvider;

let provider = OpenAiProvider::from_env()?;

// 启用对话历史
let agent = DefaultAgent::builder()
    .provider(provider)
    .model("qwen3.5:9b")
    .with_conversation(true)       // ← 启用自动对话历史管理
    .with_max_messages(20)         // ← 可选：保留最近 20 条消息
    .build();

// 第一轮对话
agent.run("你好，我叫小明").await?;

// 第二轮对话（自动记住上一轮）
agent.run("你还记得我叫什么吗？").await?;  // 回答：小明
```

### 不启用对话历史（默认行为）

```rust
// 不启用对话历史（向后兼容）
let agent = DefaultAgent::builder()
    .provider(provider)
    .model("qwen3.5:9b")
    .with_conversation(false)  // 或不调用此方法
    .build();

// 每次对话都是独立的
agent.run("你好").await?;
agent.run("你还记得我吗？").await?;  // 不记得，因为没有启用对话历史
```

## API 说明

### `with_conversation(enabled: bool)`

启用或禁用对话历史管理。

**参数：**
- `enabled`: `true` 启用，`false` 禁用

**示例：**
```rust
let agent = DefaultAgent::builder()
    .provider(provider)
    .model("qwen3.5:9b")
    .with_conversation(true)
    .build();
```

### `with_max_messages(max: usize)`

设置对话历史最大消息数（仅在启用对话时有效）。

**参数：**
- `max`: 最大消息数（0 表示无限制）

**示例：**
```rust
let agent = DefaultAgent::builder()
    .provider(provider)
    .model("qwen3.5:9b")
    .with_conversation(true)
    .with_max_messages(20)  // 保留最近 20 条消息
    .build();
```

### `get_conversation_history() -> Option<Vec<ChatMessage>>`

获取当前对话历史（异步方法）。

**返回：**
- `Some(Vec<ChatMessage>)`: 如果启用了对话历史，返回历史消息的副本
- `None`: 如果没有启用对话历史

**示例：**
```rust
if let Some(history) = agent.get_conversation_history().await {
    println!("历史消息数：{}", history.len());
    for msg in history {
        println!("{}: {}", 
            match msg.role {
                Role::User => "用户",
                Role::Assistant => "助手",
                _ => "",
            },
            msg.content
        );
    }
}
```

### `clear_conversation()`

清空对话历史（异步方法）。

**示例：**
```rust
// 清空对话历史
agent.clear_conversation().await;

// 清空后，Agent 不记得之前的对话
agent.run("你好").await?;
```

## 使用场景

### 场景 1：个人助手（需要记忆）

```rust
// 个人助手需要记住用户信息
let assistant = DefaultAgent::builder()
    .provider(provider)
    .model("qwen3.5:9b")
    .with_conversation(true)
    .with_max_messages(50)  // 保留较多消息
    .build();

// 多轮对话
assistant.run("我叫张三").await?;
assistant.run("我是一名医生").await?;
assistant.run("我平时喜欢打篮球").await?;

// 之后可以询问之前的信息
let output = assistant.run("我是做什么工作的？").await?;
// 回答：医生
```

### 场景 2：客服机器人（不需要记忆）

```rust
// 客服机器人每次回答独立问题
let bot = DefaultAgent::builder()
    .provider(provider)
    .model("qwen3.5:9b")
    .with_conversation(false)  // 不启用对话历史
    .build();

// 每次对话独立
bot.run("如何重置密码？").await?;
bot.run("如何修改邮箱？").await?;  // 与上一轮无关
```

### 场景 3：多轮任务后清空历史

```rust
let agent = DefaultAgent::builder()
    .provider(provider)
    .model("qwen3.5:9b")
    .with_conversation(true)
    .build();

// 完成一个任务
agent.run("帮我写一封邮件给老板").await?;
agent.run("语气要正式一点").await?;
agent.run("主题是请假").await?;

// 任务完成后清空历史
agent.clear_conversation();

// 开始新任务，不受之前影响
agent.run "帮我写一个购物清单").await?;
```

### 场景 4：查看历史进行分析

```rust
let agent = DefaultAgent::builder()
    .provider(provider)
    .model("qwen3.5:9b")
    .with_conversation(true)
    .build();

// 对话...
agent.run("你好").await?;
agent.run("今天天气不错").await?;

// 查看历史进行分析
if let Some(history) = agent.get_conversation_history() {
    // 分析对话内容
    let user_messages: Vec<_> = history
        .iter()
        .filter(|m| m.role == Role::User)
        .collect();
    
    println!("用户发送了 {} 条消息", user_messages.len());
    
    // 或者保存到文件
    let json = serde_json::to_string_pretty(&history)?;
    std::fs::write("conversation.json", json)?;
}
```

## 完整示例

### 示例 1：基础对话

```rust
use agentkit::agent::DefaultAgent;
use agentkit::provider::OpenAiProvider;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let provider = OpenAiProvider::from_env()?;
    
    let agent = DefaultAgent::builder()
        .provider(provider)
        .model("qwen3.5:9b")
        .with_conversation(true)
        .build();
    
    // 第一轮
    println!("用户：你好，我叫小明");
    let output = agent.run("你好，我叫小明").await?;
    println!("助手：{}", output.text().unwrap());
    
    // 第二轮
    println!("\n用户：你还记得我叫什么吗？");
    let output = agent.run("你还记得我叫什么吗？").await?;
    println!("助手：{}", output.text().unwrap());
    // 输出：小明
    
    Ok(())
}
```

### 示例 2：管理对话历史

```rust
use agentkit::agent::DefaultAgent;
use agentkit::provider::OpenAiProvider;
use agentkit_core::provider::types::Role;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let provider = OpenAiProvider::from_env()?;
    
    let agent = DefaultAgent::builder()
        .provider(provider)
        .model("qwen3.5:9b")
        .with_conversation(true)
        .with_max_messages(10)
        .build();
    
    // 对话
    agent.run("问题 1").await?;
    agent.run("问题 2").await?;
    agent.run("问题 3").await?;
    
    // 查看历史
    if let Some(history) = agent.get_conversation_history().await {
        println!("=== 对话历史 ===");
        for msg in history {
            let role = match msg.role {
                Role::User => "用户",
                Role::Assistant => "助手",
                _ => "",
            };
            println!("{}: {}", role, msg.content.chars().take(30).collect::<String>());
        }
    }
    
    // 清空历史
    agent.clear_conversation().await;
    println!("\n已清空对话历史");
    
    Ok(())
}
```

## 工作原理

### 内部流程

```
用户调用 agent.run(input)
    ↓
1. 从 conversation_manager 获取历史消息
2. 将历史消息添加到 messages
3. 添加当前用户消息到 messages
4. 调用 LLM
5. 获取助手回复
6. 保存用户消息到 conversation_manager
7. 保存助手回复到 conversation_manager
8. 返回结果
```

### 线程安全

`ConversationManager` 使用 `Arc<Mutex<>>` 包裹，支持：
- 多线程安全访问
- 异步锁（`lock().await`）
- 同步锁（`blocking_lock()`）

## 最佳实践

### 1. 选择合适的消息限制

```rust
// 短对话场景
let agent = DefaultAgent::builder()
    .with_conversation(true)
    .with_max_messages(10)
    .build();

// 长对话场景
let agent = DefaultAgent::builder()
    .with_conversation(true)
    .with_max_messages(50)
    .build();
```

### 2. 定期清理历史

```rust
// 每 10 轮对话后清理
let mut conversation_count = 0;

loop {
    let input = get_user_input();
    agent.run(input).await?;
    conversation_count += 1;
    
    if conversation_count >= 10 {
        agent.clear_conversation();
        conversation_count = 0;
    }
}
```

### 3. 保存和恢复对话

```rust
use serde_json;

// 保存
if let Some(history) = agent.get_conversation_history() {
    let json = serde_json::to_string(&history)?;
    std::fs::write("conversation.json", json)?;
}

// 恢复（需要手动添加到新的 Agent）
let json = std::fs::read_to_string("conversation.json")?;
let history: Vec<ChatMessage> = serde_json::from_str(&json)?;
// 注意：目前不支持直接恢复，需要手动实现
```

## 与手动管理对比

### 手动管理（旧方式）

```rust
let mut conv = ConversationManager::new();
let agent = DefaultAgent::builder()
    .provider(provider)
    .model("qwen3.5:9b")
    .build();

// 每轮对话需要手动：
conv.add_user_message(input);
let messages = conv.get_messages();
let output = agent.run(AgentInput::from_messages(messages)).await?;
conv.add_assistant_message(output.text().unwrap());
```

### 自动管理（新方式）

```rust
let agent = DefaultAgent::builder()
    .provider(provider)
    .model("qwen3.5:9b")
    .with_conversation(true)
    .build();

// 直接使用
let output = agent.run(input).await?;
```

## 注意事项

1. **向后兼容**：默认不启用对话历史，保持向后兼容
2. **性能考虑**：启用对话历史会增加内存使用
3. **并发安全**：使用 `Arc<Mutex<>>` 保证线程安全
4. **系统提示**：清空历史后会自动重新添加系统提示

## 运行示例

```bash
# 设置环境变量
export OPENAI_API_KEY=sk-your-key
export OPENAI_BASE_URL=http://your-server:11434/v1

# 运行示例
cargo run --example 06_agent_conversation -p agentkit
```
