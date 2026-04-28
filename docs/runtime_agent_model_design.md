# Runtime/Agent 必须设置 Model 的设计改进

## 设计理念

### 核心原则

```
Provider      = 提供 AI 能力（连接 API），不负责选择模型
Runtime/Agent = 决策和执行单元，必须指定模型
```

### 为什么这样设计？

1. **语义清晰**：Provider 只是通道，Runtime/Agent 才是决策者
2. **职责分离**：Provider 不关心具体业务，Runtime/Agent 根据场景选择模型
3. **灵活性**：同一个 Provider 可以在不同 Runtime 中使用不同模型
4. **显式优于隐式**：强制开发者明确指定模型，避免意外使用错误模型

## 修改内容

### 1. RuntimeConfig

**修改前：**
```rust
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub max_steps: usize,
    pub max_tool_concurrency: usize,
    pub enable_tool_logging: bool,
    pub debug_mode: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self { /* ... */ }
}
```

**修改后：**
```rust
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub model: String,              // ← 必需字段
    pub max_steps: usize,
    pub max_tool_concurrency: usize,
    pub enable_tool_logging: bool,
    pub debug_mode: bool,
}

impl RuntimeConfig {
    /// 必须指定 model
    pub fn new(model: impl Into<String>) -> Self { /* ... */ }
}
```

### 2. DefaultRuntime

**修改前：**
```rust
let runtime = DefaultRuntime::new(provider, tools)
    .with_system_prompt("...");
```

**修改后：**
```rust
// 方式 1：构造函数中指定
let runtime = DefaultRuntime::new(provider, tools, "gpt-4o-mini")
    .with_system_prompt("...");

// 方式 2：使用 RuntimeConfig
let config = RuntimeConfig::new("gpt-4o-mini")
    .with_max_steps(10);
let runtime = DefaultRuntime::with_config(provider, tools, config);
```

### 3. DefaultAgent

**修改前：**
```rust
pub struct DefaultAgent<P> {
    default_model: Option<String>,  // ← 可选
    // ...
}
```

**修改后：**
```rust
pub struct DefaultAgent<P> {
    model: String,                  // ← 必需
    // ...
}

// Builder 必须调用 model() 才能 build()
let agent = DefaultAgent::builder()
    .provider(provider)
    .model("gpt-4o")    // ← 必须调用
    .build();
```

## 使用示例

### 示例 1：Runtime 基础使用

```rust
use rucora::prelude::*;
use rucora::provider::OpenAiProvider;
use rucora::runtime::{DefaultRuntime, ToolRegistry};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Provider 仅提供连接能力
    let provider = OpenAiProvider::from_env()?;
    
    // Runtime 必须指定 model
    let runtime = DefaultRuntime::new(
        Arc::new(provider),
        ToolRegistry::new(),
        "gpt-4o-mini"  // ← 必须指定
    )
    .with_system_prompt("你是有用的助手");
    
    let output = runtime.run(AgentInput::new("你好")).await?;
    println!("{}", output.text().unwrap_or("无回复"));
    
    Ok(())
}
```

### 示例 2：Runtime 使用环境变量

```rust
use rucora::prelude::*;
use rucora::provider::OpenAiProvider;
use rucora::runtime::{DefaultRuntime, RuntimeConfig, ToolRegistry};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let provider = OpenAiProvider::from_env()?;
    
    // 从环境变量读取模型
    let model = std::env::var("OPENAI_DEFAULT_MODEL")
        .unwrap_or_else(|_| "gpt-4o-mini".to_string());
    
    // 使用 RuntimeConfig
    let config = RuntimeConfig::new(model)
        .with_max_steps(10);
    
    let runtime = DefaultRuntime::with_config(
        Arc::new(provider),
        ToolRegistry::new(),
        config
    );
    
    let output = runtime.run(AgentInput::new("你好")).await?;
    println!("{}", output.text().unwrap_or("无回复"));
    
    Ok(())
}
```

### 示例 3：Agent 必须指定 model

```rust
use rucora::prelude::*;
use rucora::provider::AnthropicProvider;
use rucora::agent::DefaultAgent;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let provider = AnthropicProvider::from_env()?;
    
    // Agent 必须指定 model
    let agent = DefaultAgent::builder()
        .provider(provider)
        .model("claude-3-5-sonnet-20241022")  // ← 必须调用
        .system_prompt("你是有用的助手")
        .tool(EchoTool)
        .build();
    
    let output = agent.run("你好").await?;
    println!("{}", output.text().unwrap_or("无回复"));
    
    Ok(())
}
```

### 示例 4：不同场景使用不同模型

```rust
use rucora::prelude::*;
use rucora::provider::OpenAiProvider;
use rucora::runtime::{DefaultRuntime, ToolRegistry};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let provider = Arc::new(OpenAiProvider::from_env()?);
    
    // 快速响应用 Runtime - 使用便宜快速的模型
    let fast_runtime = DefaultRuntime::new(
        provider.clone(),
        ToolRegistry::new(),
        "gpt-4o-mini"
    );
    
    // 复杂分析用 Runtime - 使用高质量模型
    let smart_runtime = DefaultRuntime::new(
        provider.clone(),
        ToolRegistry::new(),
        "gpt-4o"
    );
    
    // 根据场景选择合适的 Runtime
    let simple_task = fast_runtime.run(AgentInput::new("问候")).await?;
    let complex_task = smart_runtime.run(AgentInput::new("分析这篇论文")).await?;
    
    Ok(())
}
```

## 环境变量支持

虽然 Provider 不再负责模型选择，但仍然支持通过环境变量配置默认模型：

```bash
# 设置默认使用的模型
export OPENAI_DEFAULT_MODEL=gpt-4o
export ANTHROPIC_DEFAULT_MODEL=claude-3-5-sonnet-20241022
export GEMINI_DEFAULT_MODEL=gemini-1.5-pro
```

```rust
// 代码中读取环境变量
let model = std::env::var("OPENAI_DEFAULT_MODEL")
    .unwrap_or_else(|_| "gpt-4o-mini".to_string());

let runtime = DefaultRuntime::new(provider, tools, model);
```

## 最佳实践

### 1. 开发环境

```rust
// 使用快速、便宜的模型进行开发
let model = std::env::var("DEV_MODEL")
    .unwrap_or_else(|_| "gpt-4o-mini".to_string());

let runtime = DefaultRuntime::new(provider, tools, model);
```

### 2. 生产环境

```rust
// 使用高质量模型
let model = std::env::var("PROD_MODEL")
    .unwrap_or_else(|_| "gpt-4o".to_string());

let runtime = DefaultRuntime::new(provider, tools, model);
```

### 3. 根据任务类型选择

```rust
// 简单任务
let simple_runtime = DefaultRuntime::new(
    provider.clone(),
    tools.clone(),
    "gpt-4o-mini"
);

// 复杂任务
let complex_runtime = DefaultRuntime::new(
    provider.clone(),
    tools.clone(),
    "gpt-4o"
);
```

## 迁移指南

### 从旧版本迁移

**旧代码：**
```rust
// Provider 有 default_model
let provider = OpenAiProvider::from_env()?;
let runtime = DefaultRuntime::new(provider, tools);
```

**新代码：**
```rust
// Provider 不再有 default_model
let provider = OpenAiProvider::from_env()?;

// Runtime 必须指定 model
let runtime = DefaultRuntime::new(
    provider,
    tools,
    "gpt-4o-mini"  // ← 新增
);
```

### 保持向后兼容

如果希望保持从环境变量读取的灵活性：

```rust
let provider = OpenAiProvider::from_env()?;

// 从环境变量读取
let model = std::env::var("OPENAI_DEFAULT_MODEL")
    .unwrap_or_else(|_| "gpt-4o-mini".to_string());

let runtime = DefaultRuntime::new(provider, tools, model);
```

## 总结

### 改进点

✅ **职责清晰**：Provider 提供连接，Runtime/Agent 负责决策  
✅ **显式配置**：强制指定模型，避免隐式错误  
✅ **灵活切换**：同一 Provider 可在不同 Runtime 中使用不同模型  
✅ **语义准确**：`model` 而不是 `default_model`，强调其必需性  

### API 变更

| 组件 | 变更 |
|------|------|
| `RuntimeConfig` | 添加必需字段 `model: String`，移除 `Default` trait |
| `DefaultRuntime::new()` | 添加 `model` 参数 |
| `DefaultAgent::builder()` | 必须调用 `model()` 才能 `build()` |
| `Provider` | 移除 `default_model()` getter（可选保留） |

这个设计让 rucora 的架构更加清晰，职责分离更加明确。
