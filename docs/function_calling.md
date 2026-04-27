# 函数调用（Function Calling）原理

## 概述

AgentKit 中所有 Agent 与外部世界的交互，无论来源是内置工具、MCP 服务器、A2A 远程 Agent 还是 Skill 脚本，**最终都统一走 LLM 的 Function Calling / Tool Calling 机制**。本文档详解这一核心机制的工作原理。

## 1. 什么是 Function Calling

Function Calling 是 LLM 提供的一种能力：开发者向 LLM 注册一组"工具"（函数名 + 参数描述），LLM 在理解用户意图后，**不直接生成回复文本**，而是生成一个结构化的工具调用请求，告诉宿主程序"请使用某某参数调用某某函数"。

宿主程序执行该函数后，将结果以 `role: "tool"` 的消息形式返回给 LLM，LLM 再基于工具输出生成最终回复。

### 标准对话流程

```
User: "今天星期几？"

Assistant (LLM 回复):
  {
    "role": "assistant",
    "content": "",
    "tool_calls": [{
      "id": "call_abc123",
      "type": "function",
      "function": {
        "name": "get_datetime",
        "arguments": "{\"format\":\"text\"}"
      }
    }]
  }

→ 宿主程序执行 get_datetime(format="text")，得到结果 "2025年1月15日，星期三"

Tool (返回结果):
  {
    "role": "tool",
    "tool_call_id": "call_abc123",
    "content": "2025年1月15日，星期三"
  }

Assistant (最终回复):
  {
    "role": "assistant",
    "content": "今天是2025年1月15日，星期三。"
  }
```

## 2. Tool 定义的结构

AgentKit 内部使用 `ToolDefinition` 结构表示一个工具：

```rust
pub struct ToolDefinition {
    pub name: String,              // 工具名称，必须唯一
    pub description: Option<String>, // 工具描述，帮助 LLM 理解
    pub input_schema: Value,        // JSON Schema，定义输入参数
}
```

### 发送给 LLM 的完整格式

不同 Provider 会将其转换为各自的 API 格式：

**OpenAI 格式：**
```json
{
  "type": "function",
  "function": {
    "name": "get_datetime",
    "description": "获取当前日期时间信息",
    "parameters": {
      "type": "object",
      "properties": {
        "format": {
          "type": "string",
          "enum": ["text", "json"],
          "default": "text",
          "description": "输出格式"
        }
      }
    }
  }
}
```

**Anthropic 格式：**
```json
{
  "name": "get_datetime",
  "description": "获取当前日期时间信息",
  "input_schema": {
    "type": "object",
    "properties": {
      "format": { ... }
    }
  }
}
```

## 3. 为什么工具参数 Schema 不能精简

你看到的冗长输出是 `serde_json::Value` 的 **Debug 格式打印**，每个 JSON 节点都会展开为 `Object {...}`、`String("...")`、`Array [...]` 等 Rust 调试格式。实际发送给 LLM 的是紧凑的 JSON 格式，并不冗长。

但更重要的是，**JSON Schema 本身就不能精简**，原因如下：

### 3.1 这是 OpenAI Function Calling API 的强制规范

OpenAI 要求 `parameters` 字段必须是**完整的 JSON Schema 对象**。LLM 依赖以下字段来正确生成工具调用：

| 字段 | 作用 |
|------|------|
| `type: "object"` | 告知 LLM 参数是一个对象 |
| `properties` | 每个参数的定义 |
| `type` (每个参数) | 告知类型（string/number/boolean/object/array） |
| `description` (每个参数) | **关键**：告诉 LLM 该参数的用途，直接影响调用准确性 |
| `enum` | 约束可选值，防止 LLM 生成非法值 |
| `required` | 告知哪些参数是必须的 |
| `default` | 提供默认值参考 |

### 3.2 LLM 不使用工具代码，只使用 Schema 描述

LLM **不会执行你的工具代码**。它看到的只有：
- 工具名称
- 工具描述
- 参数 Schema（类型、描述、约束）

如果精简掉 `description`，LLM 就不知道该在什么场景下调用这个工具；如果去掉 `type` 约束，LLM 可能生成类型错误的参数值。

### 3.3 每个工具平均 3-5 个参数

一个典型工具的 schema 大小约 200-500 字符（JSON 格式），这在 LLM 的上下文窗口（通常 8K-128K tokens）中占比极小。**10 个工具也仅消耗约 5000 字符**，对 token 成本的影响可忽略。

### 3.4 调试输出 ≠ 实际传输格式

你在日志中看到的冗长 `ToolDefinition { name: ..., input_schema: Object {...} }` 是 **Rust Debug 格式**，仅用于开发调试。实际发送给 LLM 的是紧凑 JSON，不会这么冗长。

## 4. 四种工具来源，同一套机制

AgentKit 支持四种工具来源，但**全部通过 Tool trait 统一**，最终都走 Function Calling 机制：

```
┌──────────────────────────────────────────────────────────────────┐
│                      工具来源                                      │
│                                                                  │
│  内置工具         MCP 服务器      A2A 远程 Agent    Skill 脚本    │
│  ShellTool       (rmcp client)   (ra2a client)    (SKILL.py/js)  │
│  FileReadTool    McpTool         A2AToolAdapter   SkillTool      │
│  HttpTool        │               │                │              │
│       │          │               │                │              │
│       └──────────┴───────────────┴────────────────┘              │
│                          │                                       │
│                 全部实现 Tool trait                               │
│                 name(), description(),                            │
│                 input_schema(), call()                            │
│                          │                                       │
│                          ▼                                       │
│                   ┌─────────────┐                                │
│                   │ToolRegistry │  追踪来源 (BuiltIn/Mcp/A2A/Skill)│
│                   └──────┬──────┘                                │
│                          │ definitions()                         │
│                          ▼                                       │
│                   ┌──────────────┐                               │
│                   │ToolDefinition│  {name, description, schema}   │
│                   └──────┬───────┘                               │
│                          │ ChatRequest.tools                     │
│                          ▼                                       │
│                   ┌─────────────┐                                │
│                   │ LlmProvider │  转换为各 Provider API 格式      │
│                   │ (OpenAI/    │                                │
│                   │  Anthropic) │                                │
│                   └──────┬──────┘                                │
│                          │                                       │
│                          ▼                                       │
│                   ┌─────────────┐                                │
│                   │    LLM      │  返回 ToolCall {name, input}    │
│                   └──────┬──────┘                                │
│                          │                                       │
│                          ▼                                       │
│                   ┌─────────────┐                                │
│                   │  Execution  │  ToolRegistry.call(name, input) │
│                   │   Loop      │                                │
│                   └──────┬──────┘                                │
│                          │                                       │
│                          ▼                                       │
│                   ┌─────────────┐                                │
│                   │ Tool Result │  添加到消息历史 (role: "tool")   │
│                   │  Message    │  LLM 基于结果生成最终回复        │
│                   └─────────────┘                                │
└──────────────────────────────────────────────────────────────────┘
```

### 4.1 MCP（Model Context Protocol）

MCP 是 Anthropic 提出的开放协议，允许 LLM 应用通过标准化接口连接外部工具和服务。

**核心流程：**
1. 通过 MCP 传输层（Stdio/HTTP）连接到 MCP Server
2. 调用 `list_tools()` 获取服务器提供的工具列表（`rmcp::model::Tool`）
3. 用 `McpTool` 适配器包装，实现 `Tool` trait
4. 注册到 `ToolRegistry`，与其他工具无异

**关键代码：**
```rust
// McpTool 实现 Tool trait
impl Tool for McpTool {
    fn name(&self) -> &str { self.spec.name.as_ref() }
    fn description(&self) -> Option<&str> { self.spec.description.as_deref() }
    fn input_schema(&self) -> Value {
        serde_json::to_value(self.spec.input_schema.as_ref())
            .unwrap_or_else(|_| json!({"type":"object"}))
    }
    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        // 通过 McpClient → rmcp peer → MCP Server 执行
    }
}
```

### 4.2 A2A（Agent-to-Agent）

A2A 将**另一个远程 Agent 当作工具**来调用。你向远程 Agent 发送消息，获取其回复。

**核心流程：**
1. 创建 `A2AToolAdapter`，封装对远程 Agent 的 HTTP 连接
2. 定义参数 schema（通常只需一个 `message` 字段）
3. 调用时通过 `ra2a` 客户端发送消息请求
4. 解析响应并返回

**关键代码：**
```rust
impl Tool for A2AToolAdapter {
    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let message_text = input.get("message").and_then(|v| v.as_str()).unwrap_or("");
        let msg = Message::user(vec![Part::text(message_text)]);
        let req = SendMessageRequest::new(msg);
        let result = self.client.send_message(&req).await?;
        // 解析响应返回
    }
}
```

### 4.3 Skills

Skills 是可复用的**外部脚本**（Python/Node.js/Bash），通过 JSON Schema 定义接口。

**核心流程：**
1. 从 YAML/目录结构加载 Skill 定义（`SkillDefinition`）
2. 用 `SkillTool` 适配器包装，实现 `Tool` trait
3. 调用时找到对应脚本文件（`.py` > `.js` > `.sh`）
4. 通过 stdin 传入 JSON 参数，从 stdout 读取 JSON 结果
5. 支持超时控制、参数验证

**关键代码：**
```rust
impl Tool for SkillTool {
    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        // 找脚本文件
        // 执行外部进程，JSON 通过 stdin 传入
        // 从 stdout 读取 JSON 结果
    }
}
```

### 4.4 内置工具

ShellTool、FileReadTool、HttpTool 等直接实现 `Tool` trait，在进程内执行逻辑。

## 5. 完整调用链路

以 MCP 工具 `get_time_info_tools` 为例：

```
1. Agent.run("今天几号？")
   ↓
2. ToolAgent.think() → 返回 AgentDecision::Chat { request }
   request.tools = Some(Vec<ToolDefinition>)  // 包含所有已注册工具
   ↓
3. LlmProvider.chat(request)
   → OpenAI: POST /v1/chat/completions { tools: [{type:"function", ...}] }
   → Anthropic: POST /v1/messages { tools: [{name, ...}] }
   ↓
4. LLM 回复（流式或一次性）
   → assistant.content = ""
   → assistant.tool_calls = [{id:"call_xxx", name:"get_time_info_tools", input:{}}]
   ↓
5. Execution Loop 解析 tool_calls
   → ToolRegistry.call("get_time_info_tools", {})
   → McpTool.call({})
   → McpClient → MCP Server → 执行 → 返回结果
   ↓
6. 将结果添加到消息历史
   messages.push(ChatMessage { role: "tool", content: "2025-01-15 ..." })
   ↓
7. 再次调用 LLM（带 tool result）
   → LLM 看到工具结果，生成最终文本回复
   ↓
8. Agent.run() 返回 AgentOutput { content: "今天是2025年1月15日..." }
```

## 6. 关键设计决策

| 决策 | 原因 |
|------|------|
| 所有工具统一走 `Tool` trait | MCP/A2A/Skill/内置工具的调用链路完全一致，代码复用率高 |
| `ToolDefinition` 使用 JSON Schema | LLM 依赖 Schema 理解参数类型和用途，无法简化 |
| `input_schema: Value` 而非强类型结构体 | 不同工具的参数差异巨大，JSON Schema 是最灵活的表示方式 |
| Provider 负责格式转换 | OpenAI/Anthropic/Gemini 的 tool 格式不同，统一在 Provider 层适配 |

## 7. 常见误解

### 误解 1："Schema 太冗长，可以精简"
**事实**：JSON Schema 是 LLM 理解工具的唯一依据。去掉 `description`、`type`、`enum` 等字段后，LLM 可能生成错误的工具调用或根本不调用。

### 误解 2："MCP/A2A 是不同的机制"
**事实**：MCP 和 A2A 只是**工具来源不同**，最终都通过 `Tool` trait 统一，走同一套 Function Calling 机制。

### 误解 3："Debug 输出就是实际传输格式"
**事实**：`ToolDefinition { ... }` 是 Rust Debug 格式，实际发送给 LLM 的是紧凑 JSON。调试输出看起来冗长是因为 `serde_json::Value` 的 Debug 实现会展开每个节点。
