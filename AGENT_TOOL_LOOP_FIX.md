# Agent 工具执行死循环问题修复

## 问题描述

Agent 在执行工具调用时陷入死循环，不断调用同一个工具（如 `weather-query`），直到达到最大步骤数限制。

## 根本原因

工具执行的结果没有被正确添加到对话历史中，导致 LLM 认为工具没有被调用，所以不断重试。

### 原始代码问题

```rust
// 原始代码：使用 ChatMessage::tool 添加工具结果
messages.push(ChatMessage::tool(
    tool_call.name.clone(),
    result.to_string(),  // 直接序列化 JSON 结果
));
```

问题：
1. `ChatMessage::tool` 使用 `name` 字段，但 OpenAI API 需要 `tool_call_id` 字段
2. `result.to_string()` 直接序列化 JSON，LLM 可能不理解

## 修复方案

将工具结果以 LLM 能理解的文本格式添加到对话历史中：

```rust
// 修复后：使用 ChatMessage::assistant 添加格式化的工具结果
let tool_result_content = if result.is_object() {
    // 如果是 JSON 对象，提取关键信息
    if let Some(msg) = result.get("message").and_then(|v| v.as_str()) {
        msg.to_string()
    } else if let Some(error) = result.get("error").and_then(|v| v.as_str()) {
        format!("工具执行失败：{}", error)
    } else if let Some(success) = result.get("success").and_then(|v| v.as_bool()) {
        if success {
            format!("工具 {} 执行成功", tool_call.name)
        } else {
            format!("工具 {} 执行失败", tool_call.name)
        }
    } else {
        result.to_string()
    }
} else {
    result.to_string()
};

messages.push(ChatMessage::assistant(
    format!("[工具 {} 执行结果]: {}", tool_call.name, tool_result_content),
));
```

## 修复效果

修复后，工具执行结果会以清晰的文本格式添加到对话历史中，例如：

```
[工具 weather-query 执行结果]: 工具 weather-query 执行成功
```

或者：

```
[工具 weather-query 执行结果]: 北京的天气：Sunny +25°C
```

这样 LLM 就能正确理解工具已经执行，并生成最终回复。

## 修改的文件

- `agentkit/agentkit/src/agent/mod.rs` - 修复 Agent 的工具执行结果处理逻辑

## 测试建议

运行示例测试修复效果：

```bash
cd D:\Desktop\ocr\agentkit
cargo run -p agentkit-skills-example
```

预期输出：
1. LLM 调用 `read_skill` 工具读取技能说明
2. LLM 调用 `weather-query` 工具查询天气
3. **工具执行成功，LLM 看到结果**
4. LLM 生成最终回复："北京现在晴朗，气温 25 摄氏度。"

## 后续优化建议

1. **添加 `tool_call_id` 支持** - 修改 `ChatMessage` 结构，添加 `tool_call_id` 字段
2. **标准化工具结果格式** - 定义统一的工具结果格式
3. **添加工具执行日志** - 记录工具执行的详细信息
