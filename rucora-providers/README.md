# rucora Providers

rucora 的 LLM Provider 实现。

## 概述

本 crate 包含 rucora 的各种 LLM Provider 具体实现，用于与不同的大语言模型服务交互。

## 支持的 Provider

| Provider | 说明 |
|----------|------|
| OpenAiProvider | OpenAI GPT 系列模型 |
| AnthropicProvider | Anthropic Claude 模型 |
| GeminiProvider | Google Gemini 模型 |
| AzureOpenAiProvider | Azure OpenAI 服务 |
| OllamaProvider | Ollama 本地模型 |
| OpenRouterProvider | OpenRouter 多模型服务 |
| DeepSeekProvider | DeepSeek 模型 |
| MoonshotProvider | Moonshot（Kimi）模型 |
| ResilientProvider | 带重试逻辑的 Provider |

## 安装

```toml
[dependencies]
rucora-providers = "0.1"
```

或通过主 rucora crate：

```toml
[dependencies]
rucora = { version = "0.1", features = ["providers"] }
```

## 使用方式

### OpenAI Provider

```rust
use rucora_providers::OpenAiProvider;
use rucora_core::provider::LlmProvider;

let provider = OpenAiProvider::from_env()?;

let request = ChatRequest::from_user_text("你好");
let response = provider.chat(request).await?;
println!("{}", response.message.content);
```

### Anthropic Provider

```rust
use rucora_providers::AnthropicProvider;

let provider = AnthropicProvider::from_env()?
    .with_default_model("claude-3-5-sonnet-20241022");
```

### Gemini Provider

```rust
use rucora_providers::GeminiProvider;

let provider = GeminiProvider::from_env()?
    .with_default_model("gemini-1.5-pro");
```

### Resilient Provider（带重试）

```rust
use rucora_providers::{OpenAiProvider, ResilientProvider, RetryConfig};
use std::sync::Arc;

let inner = Arc::new(OpenAiProvider::from_env()?);
let provider = ResilientProvider::new(inner)
    .with_config(RetryConfig::new()
        .with_max_retries(3)
        .with_base_delay_ms(200));
```

## Feature 配置

| Feature | 说明 |
|---------|------|
| `openai` | OpenAI Provider（默认启用） |
| `anthropic` | Anthropic Provider |
| `gemini` | Google Gemini Provider |
| `azure-openai` | Azure OpenAI Provider |
| `ollama` | Ollama Provider |
| `openrouter` | OpenRouter Provider |
| `deepseek` | DeepSeek Provider |
| `moonshot` | Moonshot Provider |
| `resilient` | Resilient Provider 包装器 |
| `all` | 启用所有 Provider |

## 环境变量

| 变量 | 说明 |
|------|------|
| `OPENAI_API_KEY` | OpenAI API Key |
| `OPENAI_BASE_URL` | OpenAI Base URL |
| `ANTHROPIC_API_KEY` | Anthropic API Key |
| `GOOGLE_API_KEY` / `GEMINI_API_KEY` | Google/Gemini API Key |
| `AZURE_OPENAI_API_KEY` | Azure OpenAI API Key |
| `AZURE_OPENAI_ENDPOINT` | Azure OpenAI Endpoint |
| `OLLAMA_BASE_URL` | Ollama Base URL |
| `OPENROUTER_API_KEY` | OpenRouter API Key |
| `DEEPSEEK_API_KEY` | DeepSeek API Key |
| `MOONSHOT_API_KEY` | Moonshot API Key |

## 许可证

MIT
