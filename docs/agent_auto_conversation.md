# Agent 自动对话历史管理

## 概述

启用 `with_conversation(true)` 后，Agent 会自动管理多轮对话历史：

1. 保存用户消息
2. 保存助手回复
3. 下一轮调用时自动带上历史消息

当前支持情况：

- `ChatAgent`：支持 `with_conversation(true)`，并支持 `max_history_messages(...)`
- `ToolAgent`：支持 `with_conversation(true)`
- `ReActAgent`：支持 `with_conversation(true)`
- `ReflectAgent`：支持 `with_conversation(true)`

## 快速开始

### ChatAgent

```rust
use rucora::agent::ChatAgent;
use rucora::provider::OpenAiProvider;

let provider = OpenAiProvider::from_env()?;

let agent = ChatAgent::builder()
    .provider(provider)
    .model("qwen3.5:9b")
    .with_conversation(true)
    .max_history_messages(20)
    .try_build()?;

agent.run("你好，我叫小明").await?;
agent.run("你还记得我叫什么吗？").await?;
```

### ToolAgent

```rust
use rucora::agent::ToolAgent;
use rucora::provider::OpenAiProvider;
use rucora::tools::ShellTool;

let provider = OpenAiProvider::from_env()?;

let agent = ToolAgent::builder()
    .provider(provider)
    .model("qwen3.5:9b")
    .tool(ShellTool)
    .with_conversation(true)
    .try_build()?;
```

## API 说明

### `with_conversation(enabled: bool)`

启用或禁用自动对话历史管理。

```rust
let agent = ChatAgent::builder()
    .provider(provider)
    .model("qwen3.5:9b")
    .with_conversation(true)
    .try_build()?;
```

### `max_history_messages(max: usize)`

仅 `ChatAgentBuilder` 支持，用于限制保留的历史消息条数。

```rust
let agent = ChatAgent::builder()
    .provider(provider)
    .model("qwen3.5:9b")
    .with_conversation(true)
    .max_history_messages(20)
    .try_build()?;
```

### `get_conversation_history().await`

获取当前历史消息副本。未启用对话历史时返回 `None`。

### `clear_conversation().await`

清空历史。若配置了系统提示词，清空后会保留系统提示词。

## 工作原理

```
agent.run(input)
    ↓
1. 读取 conversation_manager 中的历史消息
2. 追加当前用户消息
3. 调用 LLM
4. 将用户消息和助手回复写回 conversation_manager
```

启用 `ConversationManager` 时，系统提示词只会保留一份，不会重复注入。

## 注意事项

1. 默认不启用自动对话历史。
2. `ChatAgent` 可以限制历史条数；其他 Agent 当前仅支持开关，不提供单独的历史条数 builder 配置。
3. `ConversationManager` 本身还支持 `with_max_messages(...)`、`with_max_tokens(...)` 等更细粒度控制；如果你需要完全自定义策略，直接使用 `ConversationManager` 更合适。
