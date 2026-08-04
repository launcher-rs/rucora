# Provider 默认模型配置指南

## 概述

rucora 的所有 Provider 现在都支持**三层优先级**的默认模型配置方案，让您无需在每次使用时都手动指定模型。

## 默认模型优先级

```
1. 手动设置（最高优先级）
   ↓
2. 环境变量
   ↓
3. Provider 内置默认值（最低优先级）
```

## 各 Provider 默认模型

| Provider | 内置默认模型 | 环境变量 |
|----------|-------------|----------|
| OpenAI | `gpt-4o-mini` | `OPENAI_DEFAULT_MODEL` |
| Anthropic | `claude-3-5-sonnet-20241022` | `ANTHROPIC_DEFAULT_MODEL` |
| Google Gemini | `gemini-1.5-flash` | `GEMINI_DEFAULT_MODEL` |
| DeepSeek | `deepseek-chat` | `DEEPSEEK_DEFAULT_MODEL` |
| Moonshot | `moonshot-v1-8k` | `MOONSHOT_DEFAULT_MODEL` |
| Ollama | `llama3.1:8b` | `OLLAMA_DEFAULT_MODEL` |
| OpenRouter | `anthropic/claude-3-5-sonnet` | `OPENROUTER_DEFAULT_MODEL` |
| Azure OpenAI | `gpt-4` | `AZURE_OPENAI_DEFAULT_DEPLOYMENT` |

## 使用方式

### 方式 1：使用内置默认模型（最简单）

无需任何配置，直接使用 Provider 的内置默认模型：

```rust
use rucora::provider::OpenAiProvider;

// 自动使用 gpt-4o-mini
let provider = OpenAiProvider::from_env()?;
```

### 方式 2：通过环境变量配置（推荐）

通过环境变量覆盖内置默认值，适合全局配置：

```bash
# 设置 OpenAI 默认模型
export OPENAI_API_KEY=sk-your-key
export OPENAI_DEFAULT_MODEL=gpt-4o

# 设置 Anthropic 默认模型
export ANTHROPIC_API_KEY=sk-ant-your-key
export ANTHROPIC_DEFAULT_MODEL=claude-3-opus-20240229

# 设置 Gemini 默认模型
export GOOGLE_API_KEY=your-key
export GEMINI_DEFAULT_MODEL=gemini-1.5-pro
```

```rust
use rucora::provider::OpenAiProvider;

// 使用环境变量中指定的 gpt-4o
let provider = OpenAiProvider::from_env()?;
```

### 方式 3：手动指定（最高优先级）

通过代码显式指定模型，优先级最高：

```rust
use rucora::provider::OpenAiProvider;

let provider = OpenAiProvider::from_env()?
    .with_default_model("gpt-4o-turbo");

// 或者使用 with_model 构造函数
let provider = OpenAiProvider::with_model(
    "https://api.openai.com/v1",
    "sk-your-key",
    "gpt-4o-turbo"
);
```

## 完整示例

### 示例 1：Runtime 中使用默认模型

```rust
use rucora::prelude::*;
use rucora::provider::OpenAiProvider;
use rucora::runtime::{DefaultRuntime, ToolRegistry};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Provider 会自动使用默认模型
    // 优先级：with_default_model() > OPENAI_DEFAULT_MODEL > gpt-4o-mini
    let provider = OpenAiProvider::from_env()?
        .with_default_model("gpt-4o");
    
    let runtime = DefaultRuntime::new(
        Arc::new(provider),
        ToolRegistry::new(),
    )
    .with_system_prompt("你是有用的助手");
    
    let output = runtime.run(AgentInput::new("你好")).await?;
    println!("{}", output.text().unwrap_or("无回复"));
    
    Ok(())
}
```

### 示例 2：Agent 中使用默认模型

```rust
use rucora::prelude::*;
use rucora::provider::AnthropicProvider;
use rucora::agent::DefaultAgent;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Provider 会自动使用默认模型
    // 优先级：with_default_model() > ANTHROPIC_DEFAULT_MODEL > claude-3-5-sonnet-20241022
    let provider = AnthropicProvider::from_env()?
        .with_default_model("claude-3-opus-20240229");
    
    let agent = DefaultAgent::builder()
        .provider(provider)
        .system_prompt("你是有用的助手")
        .tool(EchoTool)
        .build();
    
    let output = agent.run("你好").await?;
    println!("{}", output.text().unwrap_or("无回复"));
    
    Ok(())
}
```

### 示例 3：在请求中临时覆盖模型

即使 Provider 设置了默认模型，也可以在单次请求中临时指定不同的模型：

```rust
use rucora::prelude::*;
use rucora::provider::OpenAiProvider;
use rucora_core::provider::types::{ChatMessage, ChatRequest, Role};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Provider 默认使用 gpt-4o-mini
    let provider = OpenAiProvider::from_env()?;
    
    // 但这次请求临时使用 gpt-4o
    let request = ChatRequest {
        model: Some("gpt-4o".to_string()), // 临时指定
        messages: vec![
            ChatMessage::new(Role::User, "你好")
        ],
        ..Default::default()
    };
    
    let response = provider.chat(request).await?;
    println!("{}", response.message.content);
    
    Ok(())
}
```

## 最佳实践

### 1. 开发环境

```bash
# .env 文件
OPENAI_API_KEY=sk-dev-key
OPENAI_DEFAULT_MODEL=gpt-4o-mini  # 使用便宜快速的模型进行开发
```

### 2. 生产环境

```bash
# .env.production 文件
OPENAI_API_KEY=sk-prod-key
OPENAI_DEFAULT_MODEL=gpt-4o      # 使用高质量模型
```

### 3. 多 Provider 配置

```bash
# 主要使用 OpenAI
OPENAI_API_KEY=sk-key
OPENAI_DEFAULT_MODEL=gpt-4o

# 备用 Anthropic
ANTHROPIC_API_KEY=sk-ant-key
ANTHROPIC_DEFAULT_MODEL=claude-3-5-sonnet-20241022

# 测试用 Gemini
GOOGLE_API_KEY=gemini-key
GEMINI_DEFAULT_MODEL=gemini-1.5-flash
```

```rust
// 代码中自动 fallback
fn create_provider() -> anyhow::Result<Arc<dyn LlmProvider>> {
    if let Ok(p) = OpenAiProvider::from_env() {
        return Ok(Arc::new(p));
    }
    if let Ok(p) = AnthropicProvider::from_env() {
        return Ok(Arc::new(p));
    }
    if let Ok(p) = GeminiProvider::from_env() {
        return Ok(Arc::new(p));
    }
    anyhow::bail!("没有可用的 Provider");
}
```

## 常见问题

### Q: 为什么我设置的默认模型没有生效？

A: 检查优先级顺序：
1. 是否代码中有 `with_default_model()` 覆盖了环境变量
2. 环境变量是否正确设置（`echo $OPENAI_DEFAULT_MODEL`）
3. 环境变量是否在创建 Provider 之前设置

### Q: 如何在运行时切换模型？

A: 有两种方式：
1. 创建新的 Provider 实例并指定不同的模型
2. 在 `ChatRequest` 中临时指定 `model` 字段

### Q: 默认模型会影响流式请求吗？

A: 不会。默认模型同时适用于 `chat()` 和 `stream_chat()` 方法。

### Q: Azure OpenAI 为什么配置不同？

A: Azure OpenAI 使用 `deployment_id` 而不是模型名称，但配置逻辑相同：
```bash
export AZURE_OPENAI_DEFAULT_DEPLOYMENT=gpt-4-deployment
```

## 迁移指南

### 从旧版本迁移

**旧代码：**
```rust
let provider = OpenAiProvider::new(base_url, api_key);
// 必须手动指定模型
let request = ChatRequest {
    model: Some("gpt-4o".to_string()),
    ..Default::default()
};
```

**新代码：**
```rust
// 自动使用默认模型，无需每次指定
let provider = OpenAiProvider::from_env()?;
let request = ChatRequest::default(); // model 会自动使用默认值
```

### 保持向后兼容

旧代码仍然有效，只是现在可以省略模型指定：

```rust
// 仍然可以这样用
let request = ChatRequest {
    model: Some("custom-model".to_string()),
    ..Default::default()
};
```

## 总结

- ✅ **内置默认值**：开箱即用，无需配置
- ✅ **环境变量**：灵活配置，适合不同环境
- ✅ **手动指定**：精确控制，优先级最高
- ✅ **请求级覆盖**：单次请求可临时使用不同模型

这种三层优先级设计让您可以根据场景选择最合适的配置方式。
