# Agent 异步输出研究报告与实施计划

## 1. 现状分析

### 1.1 当前架构

项目已具备完整的流式输出基础设施，但存在以下问题：

#### 已具备的能力

| 层级 | 流式方法 | 状态 |
|------|----------|------|
| `LlmProvider` | `stream_chat()` → `BoxStream<ChatStreamChunk>` | ✅ 已实现，各 Provider 有具体实现 |
| `Agent` trait | `run_stream()` → `BoxStream<ChannelEvent>` | ⚠️ 默认实现返回"不支持流式输出"错误 |
| `AgentExecutor` trait | `run_stream()` → `BoxStream<ChannelEvent>` | ⚠️ 仅有 trait 定义 |
| `DefaultExecution` | `run_stream_simple()` → `BoxStream<ChannelEvent>` | ✅ 已实现完整流式逻辑 |
| `ChannelEvent` | `TokenDelta` / `ToolCall` / `ToolResult` 等事件 | ✅ 已定义完整事件类型 |

#### 核心问题

1. **Agent 的 `run_stream()` 默认实现不可用**：`agentkit-core/src/agent/mod.rs:570-578` 的默认实现直接返回错误，任何未覆写此方法的 Agent 都无法使用流式输出。

2. **具体 Agent 类型未覆写 `run_stream()`**：`ToolAgent`、`ReActAgent`、`ReflectAgent`、`ChatAgent`、`SimpleAgent` 均未在各自的 `Agent` trait 实现中覆写 `run_stream()` 方法。

3. **`DefaultExecution::run_stream_simple()` 存在但未被桥接**：`execution.rs:1001-1176` 已实现了完整的流式执行逻辑，包括：
   - 调用 `provider.stream_chat()` 获取 token 流
   - 逐帧发射 `ChannelEvent::TokenDelta`
   - 处理流中的 `tool_calls`，并发执行工具
   - 发射 `ChannelEvent::ToolCall` / `ToolResult` 事件

   但没有任何 Agent 将此方法桥接到自己的 `run_stream()` 中。

4. **用户无便捷的高层流式 API**：当前 `run_stream()` 返回的是原始 `ChannelEvent` 流，用户需要手动匹配事件类型。缺少类似 `run()` 那样返回最终 `AgentOutput` 的便捷高层 API。

### 1.2 事件流模型

`ChannelEvent` 枚举定义了完整的流式事件类型：

```
TokenDelta(delta)  → 逐字文本输出
ToolCall(call)     → 工具调用开始
ToolResult(result) → 工具调用结果
Message(msg)       → 完整消息（助手回复完成）
Error(err)         → 错误事件
```

流的生命周期：
```
[TokenDelta*] → [ToolCall → ToolResult]* → [TokenDelta*] → Message → 结束
```

## 2. 设计目标

1. **让所有 Agent 类型天然支持流式输出**：通过 `DefaultExecution` 桥接，使 `ToolAgent` 等 Agent 的 `run_stream()` 返回真实的流。

2. **提供高层便捷 API**：除了底层的 `run_stream()` 事件流，提供更高层的 `run_stream_text()` 方法，直接输出拼接后的文本，降低使用门槛。

3. **保持向后兼容**：不破坏现有 `run()` 方法的行为。

## 3. 实施方案

### Phase 1: 桥接 DefaultExecution 到各 Agent

**核心思路**：让每个 Agent 在 `Agent::run_stream()` 实现中调用自身的 `DefaultExecution::run_stream_simple()`。

#### 3.1 ToolAgent 添加 run_stream

**文件**: `agentkit/src/agent/tool.rs`

在 `ToolAgent` 的 `Agent` impl 中添加：

```rust
fn run_stream(
    &self,
    input: AgentInput,
) -> BoxStream<'static, Result<ChannelEvent, AgentError>> {
    self.execution.run_stream_simple(input)
}
```

#### 3.2 ReActAgent 添加 run_stream

**文件**: `agentkit/src/agent/react.rs`

ReActAgent 同样组合了 `DefaultExecution`，添加相同的桥接：

```rust
fn run_stream(
    &self,
    input: AgentInput,
) -> BoxStream<'static, Result<ChannelEvent, AgentError>> {
    self.execution.run_stream_simple(input)
}
```

#### 3.3 ReflectAgent 添加 run_stream

**文件**: `agentkit/src/agent/reflect.rs`

ReflectAgent 的"生成-反思-改进"循环中，每一轮生成阶段都可以流式输出。添加桥接：

```rust
fn run_stream(
    &self,
    input: AgentInput,
) -> BoxStream<'static, Result<ChannelEvent, AgentError>> {
    self.execution.run_stream_simple(input)
}
```

#### 3.4 ChatAgent 添加 run_stream

**文件**: `agentkit/src/agent/chat.rs`

ChatAgent 需要处理对话历史的流式输出：

```rust
fn run_stream(
    &self,
    input: AgentInput,
) -> BoxStream<'static, Result<ChannelEvent, AgentError>> {
    self.execution.run_stream_simple(input)
}
```

#### 3.5 SimpleAgent 添加 run_stream

**文件**: `agentkit/src/agent/simple.rs`

SimpleAgent 是最简单的 Agent，它不调用 `DefaultExecution`（因为它不需要工具循环），需要单独实现：

```rust
fn run_stream(
    &self,
    input: AgentInput,
) -> BoxStream<'static, Result<ChannelEvent, AgentError>> {
    // 构建请求
    let mut messages = Vec::new();
    if let Some(ref prompt) = self.system_prompt {
        messages.push(ChatMessage::system(prompt.clone()));
    }
    messages.push(ChatMessage::user(input.text));

    let request = ChatRequest {
        messages,
        model: Some(self.model.clone()),
        ..Default::default()
    };

    // 直接流式调用 provider
    let provider = self.provider.clone();
    let stream = try_stream! {
        let mut s = provider.stream_chat(request)?;
        while let Some(item) = s.next().await {
            let chunk = item?;
            if let Some(delta) = chunk.delta {
                yield ChannelEvent::TokenDelta(TokenDeltaEvent { delta });
            }
        }
    };
    Box::pin(stream)
}
```

### Phase 2: 提供高层便捷 API

在 `DefaultExecution` 上添加 `run_stream_text()` 方法，将 token 流拼接为完整文本。

**文件**: `agentkit/src/agent/execution.rs`

```rust
/// 高层流式 API：返回拼接后的最终文本
///
/// 此方法消费事件流，自动拼接 TokenDelta 为完整文本返回。
/// 适用于只需要最终文本、不需要逐帧处理的场景。
pub async fn run_stream_text(&self, input: AgentInput) -> Result<String, AgentError> {
    let mut stream = self.run_stream_simple(input);
    let mut text = String::new();

    while let Some(event) = stream.next().await {
        match event? {
            ChannelEvent::TokenDelta(delta) => {
                text.push_str(&delta.delta);
            }
            ChannelEvent::Error(err) => {
                return Err(AgentError::Message(err.message));
            }
            _ => {}
        }
    }

    Ok(text)
}
```

然后为各 Agent 类型添加对应的便捷方法。以 `ToolAgent` 为例：

```rust
impl<P> ToolAgent<P>
where
    P: LlmProvider + Send + Sync + 'static,
{
    /// 高层流式 API：运行并返回最终拼接的文本
    pub async fn run_stream_text(&self, input: impl Into<AgentInput>) -> Result<String, AgentError> {
        self.execution.run_stream_text(input.into()).await
    }
}
```

### Phase 3: 添加使用示例

**文件**: `agentkit/examples/` 新建 `25_streaming_agent.rs`

演示：
1. 使用 `run_stream()` 逐帧处理事件
2. 使用 `run_stream_text()` 获取最终文本
3. 实时打印 token delta（类似打字机效果）

### Phase 4: 测试

- 为 `SimpleAgent::run_stream()` 添加单元测试
- 为 `ToolAgent::run_stream()` 添加集成测试
- 验证事件流顺序和完整性

## 4. 变更影响评估

| 变更 | 影响范围 | 风险 |
|------|----------|------|
| Agent trait 增加默认 `run_stream()` | 无（已有默认实现） | 低 |
| ToolAgent/ReActAgent/ReflectAgent/ChatAgent 覆写 `run_stream()` | 仅影响这些类型 | 低 |
| SimpleAgent 实现 `run_stream()` | 新增能力 | 低 |
| `DefaultExecution::run_stream_text()` | 新增方法 | 无 |
| 各 Agent 添加 `run_stream_text()` | 新增方法 | 无 |
| 新示例文件 | 仅新增 | 无 |

## 5. 未来扩展方向（不在本次实施范围内）

1. **`run_stream_output()`**：返回 `AgentOutput` 的流式版本，边流边构建最终输出对象
2. **回调式 API**：提供 `on_token`, `on_tool_call`, `on_complete` 回调接口
3. **SSE/HTTP Stream 导出**：将 Agent 流直接导出为 HTTP Server-Sent Events

## 6. 实施顺序总结

```
Step 1: ToolAgent 添加 run_stream() 桥接
Step 2: ReActAgent 添加 run_stream() 桥接
Step 3: ReflectAgent 添加 run_stream() 桥接
Step 4: ChatAgent 添加 run_stream() 桥接
Step 5: SimpleAgent 实现 run_stream()（单独实现）
Step 6: DefaultExecution 添加 run_stream_text() 高层 API
Step 7: 各 Agent 添加 run_stream_text() 便捷方法
Step 8: 创建 streaming_agent.rs 示例
Step 9: 运行 cargo test 和 cargo clippy 验证
```
