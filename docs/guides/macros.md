# rucora 宏系统文档

本目录包含 rucora 框架的宏系统实现与使用示例。

## 快速开始

```rust
use rucora::prelude::*;
use rucora::{agent, rucora_tool, messages, chat_request, tool_params};
use serde_json::{Value, json};
use rucora_core::error::ToolError;

// 1. 使用 #[rucora_tool] 快速定义工具
#[rucora_tool(name = "add", description = "两数相加")]
async fn add(a: f64, b: f64) -> Result<Value, ToolError> {
    Ok(json!({ "result": a + b }))
}

// 2. 使用 agent! 宏构建 Agent
let agent = agent!(
    ToolAgent,
    provider: provider,
    model: "gpt-4o-mini",
    system_prompt: "你是数学助手",
    tools: [AddTool],
    max_steps: 5,
)?;

// 3. 使用 messages! 构建对话
let msgs = messages![
    system("你是数学助手"),
    user("1+1等于几？"),
];
```

## 宏参考

### `#[rucora_tool]` 过程宏

从异步函数自动生成 `Tool` trait 实现，包括参数结构体、JSON Schema 和工具注册逻辑。

**属性：**
- `name` (必需): 工具名称（暴露给 LLM）
- `description` (必需): 工具描述

**示例：**
```rust
#[rucora_tool(name = "get_weather", description = "获取城市天气")]
async fn get_weather(city: String) -> Result<Value, ToolError> {
    Ok(json!({ "city": city, "temp": 22 }))
}
// 自动生成: GetWeatherParams + GetWeatherTool + impl Tool
```

**参数文档：**
函数参数的 `///` 文档注释会自动转换为 JSON Schema 的 `description` 字段。

### `agent!` 宏

声明式构建各类 Agent（ToolAgent, SimpleAgent, ChatAgent, ReActAgent, ReflectAgent）。

**语法：**
```text
agent!(
    AgentType,
    provider: provider_instance,
    model: "model_name",
    system_prompt: "prompt",
    tools: [tool1, tool2],
    max_steps: 10,
    temperature: 0.7,
    // 其他 builder 方法...
)
```

**支持的键：**
| 键 | 说明 |
|---|---|
| `provider` | LLM Provider 实例（必需） |
| `model` | 模型名称（必需） |
| `system_prompt` | 系统提示词 |
| `tools` | 工具数组 `[Tool1, Tool2]` |
| `max_steps` / `max_iterations` | 最大步骤数 |
| `temperature` | 温度参数 (0.0-2.0) |
| `max_tokens` | 最大输出 token |
| `top_p` / `top_k` | 采样参数 |
| `frequency_penalty` | 频率惩罚 |
| `presence_penalty` | 存在惩罚 |
| `with_conversation` | 启用对话历史 |
| `stop` | 停止序列数组 |
| `extra_params` | 额外参数 |

### `messages!` 宏

快速构建 `Vec<ChatMessage>`。

```rust
let msgs = messages![
    system("系统提示"),
    user("用户输入"),
    assistant("助手回复"),
    tool("tool_name", "工具输出"),
];
```

### `chat_request!` 宏

快速构建 `ChatRequest`。

```rust
let req = chat_request!(
    messages: [system("提示"), user("输入")],
    model: "gpt-4o",
    temperature: 0.7,
    max_tokens: 4096,
);
```

### `tool_params!` 宏

快速构建工具参数 JSON Schema。

```rust
let schema = tool_params! {
    "query" => (string, required, "搜索关键词"),
    "limit" => (number, "返回数量限制"),
};
```

## 类型增强

### `ToolRiskLevel`

工具风险等级枚举，用于标记工具操作的潜在风险。

```rust
use rucora_core::tool::types::ToolRiskLevel;

// 变体
ToolRiskLevel::Safe       // 安全操作（查询、读取）
ToolRiskLevel::Caution    // 需谨慎（写入、HTTP 请求）
ToolRiskLevel::Dangerous  // 危险操作（删除、执行命令）

// 方法
risk_level.requires_approval() // 是否需要人工审批
risk_level.as_str()            // 字符串表示
```

在实现 `Tool` trait 时覆盖 `risk_level()` 方法：
```rust
fn risk_level(&self) -> ToolRiskLevel {
    ToolRiskLevel::Dangerous
}
```

### `ToolResult` 增强

新增字段支持更丰富的结果类型：

```rust
// 成功结果
let res = ToolResult::success("call_1", json!({ "data": 42 }));

// 失败结果
let res = ToolResult::failure("call_2", "网络连接超时");

// 带结构化数据
let res = ToolResult::success("call_3", json!({ "summary": "处理完成" }))
    .with_data(json!({ "rows": 100, "columns": 5 }));

// 带二进制数据
let res = ToolResult::success("call_4", json!({}))
    .with_bytes(image_bytes);

// 检查状态
res.is_success() // true/false
```

## 运行示例

```bash
export OPENAI_API_KEY=sk-your-key
export MODEL_NAME=gpt-4o-mini
cargo run --example 26_macros
```
