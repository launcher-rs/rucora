# Provider 默认模型改进总结

## 问题描述

在之前的设计中，用户在使用 Provider、Agent 或 Runtime 时，很容易忘记设置 `model` 参数，导致运行时错误。主要问题：

1. **Provider 没有内置默认模型**：`default_model` 初始化为 `None`
2. **必须手动指定**：每次创建 Provider 后需要调用 `with_default_model()`
3. **缺少环境变量支持**：无法通过环境变量全局配置
4. **示例代码不完整**：示例中没有展示如何设置 model

## 解决方案

实现了**三层优先级**的默认模型配置方案：

```
1. 手动设置（最高优先级）
   ↓
2. 环境变量
   ↓
3. Provider 内置默认值（最低优先级）
```

## 修改内容

### 1. 修改的 Provider 文件

| Provider | 默认模型 | 环境变量 | 文件 |
|----------|---------|----------|------|
| OpenAI | `gpt-4o-mini` | `OPENAI_DEFAULT_MODEL` | `rucora/src/provider/openai.rs` |
| Anthropic | `claude-3-5-sonnet-20241022` | `ANTHROPIC_DEFAULT_MODEL` | `rucora/src/provider/anthropic.rs` |
| Google Gemini | `gemini-1.5-flash` | `GEMINI_DEFAULT_MODEL` | `rucora/src/provider/gemini.rs` |
| DeepSeek | `deepseek-chat` | `DEEPSEEK_DEFAULT_MODEL` | `rucora/src/provider/deepseek.rs` |
| Moonshot | `moonshot-v1-8k` | `MOONSHOT_DEFAULT_MODEL` | `rucora/src/provider/moonshot.rs` |
| Ollama | `llama3.1:8b` | `OLLAMA_DEFAULT_MODEL` | `rucora/src/provider/ollama.rs` |
| OpenRouter | `anthropic/claude-3-5-sonnet` | `OPENROUTER_DEFAULT_MODEL` | `rucora/src/provider/openrouter.rs` |
| Azure OpenAI | `gpt-4` (deployment) | `AZURE_OPENAI_DEFAULT_DEPLOYMENT` | `rucora/src/provider/azure_openai.rs` |

### 2. API 变更

#### 新增方法

```rust
// 所有 Provider 新增
pub fn with_model(base_url, api_key, default_model) -> Self
pub fn default_model(&self) -> &str

// Azure OpenAI 特殊处理
pub fn with_deployment(endpoint, api_key, deployment_id) -> Self
pub fn default_deployment_id(&self) -> &str
```

#### 行为变更

| 方法 | 修改前 | 修改后 |
|------|--------|--------|
| `default_model` 字段 | `Option<String>` | `String` |
| `from_env()` | 不读取默认模型环境变量 | 读取环境变量，回退到内置默认值 |
| `new()` | `default_model: None` | 使用内置默认常量 |
| `with_default_model()` | `self.default_model = Some(...)` | `self.default_model = ...` |
| `chat()` / `stream_chat()` | 可能返回 "缺少 model" 错误 | 始终有默认值，不会失败 |

### 3. 新增文档

- `docs/provider_default_model.md` - 完整的配置指南和使用示例

### 4. 更新的示例

- `rucora/examples/01_basic_chat.rs` - 添加默认模型说明和日志输出
- `rucora/examples/04_memory.rs` - 已包含完整的 model 设置示例

## 使用示例

### 最简单用法（开箱即用）

```rust
use rucora::provider::OpenAiProvider;

// 自动使用 gpt-4o-mini，无需任何配置
let provider = OpenAiProvider::from_env()?;
```

### 通过环境变量配置

```bash
export OPENAI_API_KEY=sk-your-key
export OPENAI_DEFAULT_MODEL=gpt-4o
```

```rust
// 使用环境变量中指定的 gpt-4o
let provider = OpenAiProvider::from_env()?;
```

### 手动指定（最高优先级）

```rust
let provider = OpenAiProvider::from_env()?
    .with_default_model("gpt-4o-turbo");
```

### 在 Runtime 中使用

```rust
use rucora::prelude::*;
use rucora::provider::OpenAiProvider;
use rucora::runtime::{DefaultRuntime, ToolRegistry};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Provider 会自动使用默认模型
    let provider = OpenAiProvider::from_env()?;
    
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

### 在 Agent 中使用

```rust
use rucora::prelude::*;
use rucora::provider::AnthropicProvider;
use rucora::agent::DefaultAgent;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let provider = AnthropicProvider::from_env()?
        .with_default_model("claude-3-opus-20240229");
    
    let agent = DefaultAgent::builder()
        .provider(provider)
        .system_prompt("你是有用的助手")
        .build();
    
    let output = agent.run("你好").await?;
    println!("{}", output.text().unwrap_or("无回复"));
    
    Ok(())
}
```

### 临时覆盖模型

```rust
use rucora_core::provider::types::{ChatMessage, ChatRequest, Role};

// Provider 默认使用 gpt-4o-mini
let provider = OpenAiProvider::from_env()?;

// 但这次请求临时使用 gpt-4o
let request = ChatRequest {
    model: Some("gpt-4o".to_string()), // 临时指定
    messages: vec![ChatMessage::new(Role::User, "你好")],
    ..Default::default()
};

let response = provider.chat(request).await?;
```

## 迁移指南

### 旧代码（仍然有效）

```rust
let provider = OpenAiProvider::new(base_url, api_key);
let request = ChatRequest {
    model: Some("gpt-4o".to_string()), // 必须手动指定
    ..Default::default()
};
```

### 新代码（推荐）

```rust
// 方式 1：使用环境变量
export OPENAI_DEFAULT_MODEL=gpt-4o
let provider = OpenAiProvider::from_env()?;
let request = ChatRequest::default(); // 自动使用默认值

// 方式 2：代码中指定
let provider = OpenAiProvider::from_env()?
    .with_default_model("gpt-4o");
let request = ChatRequest::default();
```

## 验证结果

### 编译检查

```bash
cargo check --workspace
# Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.70s
```

### 测试通过

```bash
cargo test -p rucora --lib provider
# test result: ok. 20+ passed; 0 failed
```

## 最佳实践建议

### 开发环境

```bash
# .env
OPENAI_API_KEY=sk-dev-key
OPENAI_DEFAULT_MODEL=gpt-4o-mini  # 快速、便宜
```

### 生产环境

```bash
# .env.production
OPENAI_API_KEY=sk-prod-key
OPENAI_DEFAULT_MODEL=gpt-4o      # 高质量
```

### 多 Provider 配置

```bash
# 主要 Provider
OPENAI_API_KEY=sk-key
OPENAI_DEFAULT_MODEL=gpt-4o

# 备用 Provider
ANTHROPIC_API_KEY=sk-ant-key
ANTHROPIC_DEFAULT_MODEL=claude-3-5-sonnet-20241022

# 测试 Provider
GOOGLE_API_KEY=gemini-key
GEMINI_DEFAULT_MODEL=gemini-1.5-flash
```

## 总结

### 改进点

✅ **开箱即用**：无需配置即可使用  
✅ **灵活配置**：支持环境变量全局配置  
✅ **精确控制**：支持代码级手动指定  
✅ **向后兼容**：旧代码仍然有效  
✅ **文档完善**：详细的使用指南和示例  

### 用户体验提升

- **减少错误**：不再因为忘记设置 model 而报错
- **简化代码**：大多数场景无需显式指定 model
- **灵活配置**：不同环境使用不同模型
- **易于调试**：日志中显示当前使用的默认模型

这个改进让 rucora 的使用体验更加流畅，特别是对于新手用户和快速原型开发场景。
