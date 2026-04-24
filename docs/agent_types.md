# Agent 类型详解

AgentKit 提供 5 种内置 Agent 类型，覆盖从简单对话到复杂推理的各种场景。

## Agent 类型总览

| Agent 类型 | 适用场景 | 工具调用 | 对话历史 | 推理能力 |
|-----------|---------|---------|---------|---------|
| `SimpleAgent` | 单次问答 | ❌ | ❌ | 基础 |
| `ChatAgent` | 多轮对话 | ❌ | ✅ | 基础 |
| `ToolAgent` | 工具调用任务 | ✅ | ✅ | 中等 |
| `ReActAgent` | 复杂推理 + 工具 | ✅ | ✅ | 强（ReAct 模式） |
| `ReflectAgent` | 自省优化 | ✅ | ✅ | 最强（反思模式） |

## SimpleAgent

最简单的 Agent 类型，适合单次问答场景。

### 特点
- 无对话历史，每次请求独立
- 不支持工具调用
- 最轻量、最快

### 使用示例

```rust
use agentkit::agent::SimpleAgent;
use agentkit::prelude::Agent;
use agentkit::provider::OpenAiProvider;

let provider = OpenAiProvider::from_env()?;

let agent = SimpleAgent::builder()
    .provider(provider)
    .model("gpt-4o-mini")
    .system_prompt("你是友好的 AI 助手。")
    .temperature(0.7)
    .build();

let output = agent.run("用一句话介绍 Rust").await?;
println!("{}", output.text().unwrap_or("无回复"));
```

### Builder 方法

| 方法 | 说明 | 默认值 |
|------|------|--------|
| `.provider()` | LLM Provider | **必需** |
| `.model()` | 模型名称 | **必需** |
| `.system_prompt()` | 系统提示词 | 无 |
| `.temperature()` | 温度 | 0.7 |
| `.top_p()` | Top-p 采样 | None |
| `.top_k()` | Top-k 采样 | None |
| `.max_tokens()` | 最大 token 数 | None |
| `.frequency_penalty()` | 频率惩罚 | None |
| `.presence_penalty()` | 存在惩罚 | None |
| `.stop()` | 停止序列 | None |
| `.extra_params()` | 额外参数 | None |
| `.llm_params()` | 完整 LLM 参数 | 默认 |

---

## ChatAgent

支持多轮对话的 Agent，内置对话历史管理。

### 特点
- 自动管理对话历史
- 可配置保留消息数量
- 不支持工具调用

### 使用示例

```rust
use agentkit::agent::ChatAgent;
use agentkit::prelude::Agent;
use agentkit::provider::OpenAiProvider;

let agent = ChatAgent::builder()
    .provider(provider)
    .model("gpt-4o-mini")
    .system_prompt("你是友好的 AI 助手。")
    .with_conversation(true)       // 启用对话历史
    .max_history_messages(20)      // 保留最近 20 条消息
    .build();

// 第一轮
agent.run("你好，我叫小明").await?;

// 第二轮（自动记住上一轮）
let output = agent.run("你还记得我叫什么吗？").await?;
// 回答：小明
```

### 独有 Builder 方法

| 方法 | 说明 | 默认值 |
|------|------|--------|
| `.with_conversation(bool)` | 启用/禁用对话历史 | false |
| `.max_history_messages(usize)` | 最大保留消息数 | 0（无限制） |

### 运行时方法

| 方法 | 说明 |
|------|------|
| `.get_conversation_history().await` | 获取对话历史 |
| `.clear_conversation().await` | 清空对话历史 |

---

## ToolAgent

支持工具调用的 Agent，适合需要执行具体任务的场景。

### 特点
- 自动工具选择和调用
- 支持多步工具调用
- 内置循环检测（防止无限循环）
- 孤儿工具消息自动清理

### 使用示例

```rust
use agentkit::agent::ToolAgent;
use agentkit::prelude::Agent;
use agentkit::provider::OpenAiProvider;
use agentkit::tools::{DatetimeTool, EchoTool};

let agent = ToolAgent::builder()
    .provider(provider)
    .model("gpt-4o-mini")
    .system_prompt("你是有用的智能助手。")
    .tool(DatetimeTool)
    .tool(EchoTool)
    .max_steps(10)
    .temperature(0.7)
    .top_p(0.9)
    .build();

let output = agent.run("现在几点了？").await?;
println!("{}", output.text().unwrap_or("无回复"));
```

### 独有 Builder 方法

| 方法 | 说明 | 默认值 |
|------|------|--------|
| `.tool(T)` | 注册单个工具 | - |
| `.tool_registry(registry)` | 注册工具表 | 空 |
| `.max_steps(usize)` | 最大执行步数 | 20 |
| `.max_tool_concurrency(usize)` | 工具并发数 | 1 |
| `.with_conversation(bool)` | 启用对话历史 | false |
| `.max_history_messages(usize)` | 最大保留消息数 | 0 |

---

## ReActAgent

基于 ReAct（Reasoning + Acting）模式的 Agent，适合复杂推理任务。

### 特点
- 显式的"思考-行动-观察"循环
- 更强的推理能力
- 支持工具调用
- 自动构建 ReAct 格式提示

### ReAct 模式流程

```
用户输入
  ↓
Thought（思考）→ 决定下一步行动
  ↓
Action（行动）→ 调用工具 或 给出最终答案
  ↓
Observation（观察）→ 工具执行结果
  ↓
循环直到得出最终答案
```

### 使用示例

```rust
use agentkit::agent::ReActAgent;
use agentkit::prelude::Agent;
use agentkit::provider::OpenAiProvider;
use agentkit::tools::{CalculatorTool, ShellTool};

let agent = ReActAgent::builder()
    .provider(provider)
    .model("gpt-4o-mini")
    .system_prompt("你是一个擅长推理的助手。")
    .tool(CalculatorTool)
    .tool(ShellTool::new())
    .max_steps(15)
    .build();

let output = agent.run("计算 123 * 456 的结果").await?;
```

### 独有 Builder 方法

| 方法 | 说明 | 默认值 |
|------|------|--------|
| `.tool(T)` | 注册工具 | - |
| `.max_steps(usize)` | 最大推理轮次 | 20 |
| `.with_conversation(bool)` | 启用对话历史 | false |

---

## ReflectAgent

基于反思模式的 Agent，在 ReAct 基础上增加了自我验证和优化能力。

### 特点
- 先给出初步答案
- 自我反思和改进答案
- 最高质量的输出
- 适合需要高准确度的场景

### Reflect 模式流程

```
用户输入
  ↓
Draft（草稿）→ 生成初步答案
  ↓
Reflect（反思）→ 审查和改进
  ↓
Final（最终）→ 输出优化后的答案
```

### 使用示例

```rust
use agentkit::agent::ReflectAgent;
use agentkit::prelude::Agent;
use agentkit::provider::OpenAiProvider;

let agent = ReflectAgent::builder()
    .provider(provider)
    .model("gpt-4o")  // 建议使用更强的模型
    .system_prompt("你是一个追求完美答案的助手。")
    .max_steps(10)
    .build();

let output = agent.run("解释量子纠缠现象").await?;
```

---

## Extractor

信息提取专用 Agent，用于从文本中提取结构化信息。

### 特点
- 固定 temperature=0.0（确定性输出）
- 支持 JSON Schema 约束
- 适合信息抽取、分类、格式化等任务

### 使用示例

```rust
use agentkit::agent::Extractor;
use agentkit::prelude::Agent;
use agentkit::provider::OpenAiProvider;

let extractor = Extractor::builder()
    .provider(provider)
    .model("gpt-4o-mini")
    .schema(serde_json::json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "age": {"type": "integer"},
            "email": {"type": "string"}
        },
        "required": ["name", "age", "email"]
    }))
    .build();

let output = extractor.run("张三，今年28岁，邮箱 zhangsan@example.com").await?;
let data: Value = output.value;
```

---

## ToolRegistry

工具注册表，用于管理和查询可用工具。

### 功能

| 方法 | 说明 |
|------|------|
| `.register(tool)` | 注册单个工具 |
| `.register_arc(tool)` | 注册 Arc 包装的工具 |
| `.definitions()` | 获取所有工具定义 |
| `.call(name, input)` | 调用指定工具 |
| `.enabled_len()` | 获取可用工具数量 |

---

## LoopDetector

循环检测器，防止 Agent 陷入无限重复调用同一工具。

### 配置

```rust
use agentkit::agent::LoopDetectorConfig;

let config = LoopDetectorConfig {
    enabled: true,
    window_size: 5,       // 滑动窗口大小
    max_repeats: 2,       // 最大重复次数
};
```

### 检测结果

| 结果 | 说明 |
|------|------|
| `Ok` | 正常，继续执行 |
| `Warning` | 注入系统消息提醒 LLM 调整策略 |
| `Block` | 替换输出，阻止继续调用 |
| `Break` | 终止循环，返回错误 |

---

## 选择指南

| 场景 | 推荐 Agent |
|------|-----------|
| 简单问答 | `SimpleAgent` |
| 多轮对话（无工具） | `ChatAgent` |
| 工具调用任务 | `ToolAgent` |
| 复杂推理 + 工具 | `ReActAgent` |
| 需要高质量答案 | `ReflectAgent` |
| 信息提取/分类 | `Extractor` |
